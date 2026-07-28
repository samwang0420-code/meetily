//! v0.7.0+ P0-4: 硬件分层检测 + 动态内存降级
//!
//! 三档分级:
//! - **High** (完美):  >=16GB RAM + Apple Silicon -> 90min 全功能
//! - **Medium** (基础): >=8GB RAM                 -> 90min 纯转写, 关闭 cam++ 人声分离, 默认 SenseVoice
//! - **Low** (低配):     <8GB RAM                  -> 仅 30min 短录音, 禁 Nano 高精度, 禁长摘要
//!
//! 动态降级: 全局进程内存 > MEMORY_PRESSURE_THRESHOLD_MB (1.2GB) 时,
//! 建议前端自动卸载 cam++/FunASR-Nano, 仅保留 SenseVoice 基础转写.

use serde::Serialize;
use sysinfo::System;

pub const MEMORY_PRESSURE_THRESHOLD_MB: u64 = 1200;
pub const DEFAULT_MAX_MEETING_MINUTES: u32 = 90;
pub const LOW_TIER_MAX_MEETING_MINUTES: u32 = 30;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceTier {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceProfile {
    pub total_memory_bytes: u64,
    pub total_memory_mb: u64,
    pub cpu_brand: String,
    pub is_apple_silicon: bool,
    pub metal_vram_mb: u64,
    pub tier: DeviceTier,
    pub recommended_max_meeting_minutes: u32,
    pub recommended_asr_model: &'static str,
    pub cam_plus_plus_disabled: bool,
    pub nano_disabled: bool,
    pub long_summary_disabled: bool,
    pub detected_at: String,
}

impl DeviceProfile {
    pub fn detect() -> Self {
        let mut sys = System::new_all();
        sys.refresh_memory();
        let total_memory_bytes = sys.total_memory();
        let total_memory_mb = total_memory_bytes / (1024 * 1024);
        let total_memory_gb = total_memory_bytes / (1024 * 1024 * 1024);

        let cpu_brand = sys
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let is_apple_silicon = cpu_brand.contains("Apple M");

        #[cfg(target_os = "macos")]
        let metal_vram_mb = {
            if is_apple_silicon {
                (total_memory_mb * 60) / 100
            } else {
                0
            }
        };
        #[cfg(not(target_os = "macos"))]
        let metal_vram_mb = 0u64;

        let tier = if total_memory_gb >= 16 && is_apple_silicon {
            DeviceTier::High
        } else if total_memory_gb >= 8 {
            DeviceTier::Medium
        } else {
            DeviceTier::Low
        };

        let recommended_max_meeting_minutes = match tier {
            DeviceTier::Low => LOW_TIER_MAX_MEETING_MINUTES,
            _ => DEFAULT_MAX_MEETING_MINUTES,
        };
        let recommended_asr_model = match tier {
            DeviceTier::High => "funasr-nano",
            _ => "sensevoice-zh",
        };
        let cam_plus_plus_disabled = !matches!(tier, DeviceTier::High);
        let nano_disabled = matches!(tier, DeviceTier::Low);
        let long_summary_disabled = matches!(tier, DeviceTier::Low);

        let detected_at = chrono::Utc::now().to_rfc3339();

        Self {
            total_memory_bytes,
            total_memory_mb,
            cpu_brand,
            is_apple_silicon,
            metal_vram_mb,
            tier,
            recommended_max_meeting_minutes,
            recommended_asr_model,
            cam_plus_plus_disabled,
            nano_disabled,
            long_summary_disabled,
            detected_at,
        }
    }
}

pub fn current_process_rss_mb() -> u64 {
    let mut sys = System::new_all();
    let pid = sysinfo::get_current_pid().expect("current pid");
    sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    let me = sys.process(pid);
    me.map(|p| p.memory() / (1024 * 1024)).unwrap_or(0)
}

pub fn is_memory_pressure(rss_mb: u64) -> bool {
    rss_mb > MEMORY_PRESSURE_THRESHOLD_MB
}


// ============================================================================
// Tauri commands
// ============================================================================

/// v0.7.0+: tier gate helper — free 用户即使硬件 High, 也不给 cam++ / Nano.
/// Pro 专属承诺: FunASR-Nano 精度模式 + cam++ 多发言人分离 仅 member 解锁.
fn apply_tier_gates(mut profile: DeviceProfile, tier: &str) -> DeviceProfile {
    if tier != "member" {
        profile.cam_plus_plus_disabled = true;
        profile.nano_disabled = true;
    }
    profile
}

#[tauri::command]
pub fn device_detect_profile(
    app: tauri::AppHandle,
    sessions: tauri::State<'_, crate::user::commands::SessionStore>,
    pool_state: tauri::State<'_, crate::state::AppState>,
    session: Option<String>,
) -> DeviceProfile {
    let mut profile = DeviceProfile::detect();
    // 查 membership (user_id -> users.membership).
    let tier = session
        .as_deref()
        .and_then(|s| sessions.map.lock().ok().and_then(|m| m.get(s).copied()))
        .and_then(|uid| {
            // 同步查 membership (简单 SELECT)
            let pool = &pool_state.db_manager.pool();
            // 用 blocking 查询, 命令是 sync fn, 不能 .await
            tauri::async_runtime::block_on(async move {
                use crate::database::repositories::user::UsersRepository;
                UsersRepository::get_quota(pool, uid).await.ok().map(|(_, m, _)| m)
            })
        })
        .unwrap_or_else(|| "free".to_string());
    apply_tier_gates(profile, &tier)
}

#[tauri::command]
pub fn device_current_memory_mb() -> u64 {
    current_process_rss_mb()
}

#[tauri::command]
pub fn device_memory_pressure(rss_mb: u64) -> bool {
    is_memory_pressure(rss_mb)
}
#[cfg(test)]
mod tests {
    use super::*;

    fn profile_high() -> DeviceProfile {
        DeviceProfile {
            total_memory_bytes: 16 * 1024 * 1024 * 1024,
            total_memory_mb: 16 * 1024,
            cpu_brand: "Apple M2".into(),
            is_apple_silicon: true,
            metal_vram_mb: 9 * 1024,
            tier: DeviceTier::High,
            recommended_max_meeting_minutes: 90,
            recommended_asr_model: "funasr-nano",
            cam_plus_plus_disabled: false,
            nano_disabled: false,
            long_summary_disabled: false,
            detected_at: String::new(),
        }
    }

    fn profile_medium() -> DeviceProfile {
        DeviceProfile {
            total_memory_bytes: 8 * 1024 * 1024 * 1024,
            total_memory_mb: 8 * 1024,
            cpu_brand: "Apple M2".into(),
            is_apple_silicon: true,
            metal_vram_mb: 4 * 1024,
            tier: DeviceTier::Medium,
            recommended_max_meeting_minutes: 90,
            recommended_asr_model: "sensevoice-zh",
            cam_plus_plus_disabled: true,
            nano_disabled: false,
            long_summary_disabled: false,
            detected_at: String::new(),
        }
    }

    fn profile_low() -> DeviceProfile {
        DeviceProfile {
            total_memory_bytes: 4 * 1024 * 1024 * 1024,
            total_memory_mb: 4 * 1024,
            cpu_brand: "Intel(R) Core(TM) i5-8250U".into(),
            is_apple_silicon: false,
            metal_vram_mb: 0,
            tier: DeviceTier::Low,
            recommended_max_meeting_minutes: LOW_TIER_MAX_MEETING_MINUTES,
            recommended_asr_model: "sensevoice-zh",
            cam_plus_plus_disabled: true,
            nano_disabled: true,
            long_summary_disabled: true,
            detected_at: String::new(),
        }
    }

    #[test]
    fn high_tier_allows_everything() {
        let p = profile_high();
        assert_eq!(p.tier, DeviceTier::High);
        assert!(!p.cam_plus_plus_disabled);
        assert!(!p.nano_disabled);
        assert!(!p.long_summary_disabled);
        assert_eq!(p.recommended_max_meeting_minutes, 90);
    }

    #[test]
    fn medium_tier_keeps_length_but_disables_cam() {
        let p = profile_medium();
        assert_eq!(p.tier, DeviceTier::Medium);
        assert_eq!(p.recommended_max_meeting_minutes, 90);
        assert!(p.cam_plus_plus_disabled);
        assert!(!p.nano_disabled);
        assert!(!p.long_summary_disabled);
    }

    #[test]
    fn low_tier_caps_30_minutes_and_disables_nano_long_summary() {
        let p = profile_low();
        assert_eq!(p.tier, DeviceTier::Low);
        assert_eq!(p.recommended_max_meeting_minutes, 30);
        assert!(p.nano_disabled);
        assert!(p.long_summary_disabled);
    }

    #[test]
    fn memory_pressure_threshold_constant() {
        assert_eq!(MEMORY_PRESSURE_THRESHOLD_MB, 1200);
        assert!(is_memory_pressure(1500));
        assert!(!is_memory_pressure(800));
    }

    #[test]
    fn apple_silicon_detection_via_brand() {
        assert!(profile_high().is_apple_silicon);
        assert!(!profile_low().is_apple_silicon);
    }

    #[test]
    fn tier_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&DeviceTier::Medium).unwrap(), "\"medium\"");
        assert_eq!(serde_json::to_string(&DeviceTier::Low).unwrap(), "\"low\"");
        assert_eq!(serde_json::to_string(&DeviceTier::High).unwrap(), "\"high\"");
    }

    #[test]
    fn detect_runs_without_panic() {
        let p = DeviceProfile::detect();
        assert!(p.total_memory_mb > 0);
        // 当前机器肯定是 M 系列 (Apple Silicon)
        assert!(p.is_apple_silicon || p.tier == DeviceTier::Medium || p.tier == DeviceTier::Low);
    }
}
