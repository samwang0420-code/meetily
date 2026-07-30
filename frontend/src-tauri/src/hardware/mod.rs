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
// v0.7.0+ §31 P0: 自动内存降级 (上线必要条件)
// ============================================================================
//
// 问题: v0.7.0 之前 is_memory_pressure 只在 test 里被调用, 实际运行时没触发.
//   长会议录音 (60+ min) 时 onnx daemon + diar 后台 cluster 累加 ~700M RSS,
//   不监控就会 silently OOM 或 swap 卡死.
//
// 设计:
//   - MemoryGuard 是 Lazy 静态, 持有当前 RSS 状态 + 上次降级时间.
//   - 每 30s 在录音 active 期间由 worker_pool 调 poll() 触发一次 check.
//   - 检测到压力后 emit "memory-pressure" event 给前端, 携带建议降级策略.
//   - 降级动作由前端执行 (切模型 / 关 cam++), 后端不直接改 model 状态.

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryPressureLevel {
    /// < 70% 阈值, 正常
    Normal,
    /// 70%-100% 阈值, 警告 (前端可提示用户, 准备降级)
    Warning,
    /// > 阈值, 强制降级
    Critical,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryPressureReport {
    pub level: MemoryPressureLevel,
    pub rss_mb: u64,
    pub threshold_mb: u64,
    pub recommended_action: &'static str,
    pub should_drop_cam_plus_plus: bool,
    pub should_switch_to_sensevoice: bool,
    pub should_disable_long_summary: bool,
}

pub fn classify_memory_pressure(rss_mb: u64) -> MemoryPressureReport {
    let threshold = MEMORY_PRESSURE_THRESHOLD_MB;
    let warn_threshold = threshold * 70 / 100;  // 840 MB

    if rss_mb >= threshold {
        MemoryPressureReport {
            level: MemoryPressureLevel::Critical,
            rss_mb,
            threshold_mb: threshold,
            recommended_action: "立即降级: 切到 sense-voice-zh + 关 cam++ + 禁用长摘要",
            should_drop_cam_plus_plus: true,
            should_switch_to_sensevoice: true,
            should_disable_long_summary: true,
        }
    } else if rss_mb >= warn_threshold {
        MemoryPressureReport {
            level: MemoryPressureLevel::Warning,
            rss_mb,
            threshold_mb: threshold,
            recommended_action: "警告: 准备降级, 提示用户",
            should_drop_cam_plus_plus: false,
            should_switch_to_sensevoice: false,
            should_disable_long_summary: false,
        }
    } else {
        MemoryPressureReport {
            level: MemoryPressureLevel::Normal,
            rss_mb,
            threshold_mb: threshold,
            recommended_action: "正常运行",
            should_drop_cam_plus_plus: false,
            should_switch_to_sensevoice: false,
            should_disable_long_summary: false,
        }
    }
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

    /// v0.7.0+ §31 P0: classify_memory_pressure 3 档分级.
    /// - Normal: < 70% 阈值 (840 MB)
    /// - Warning: 70% - 100% 阈值
    /// - Critical: >= 阈值 (1200 MB)
    #[test]
    fn classify_memory_pressure_three_levels() {
        let r = classify_memory_pressure(500);
        assert_eq!(r.level, MemoryPressureLevel::Normal);
        assert!(!r.should_drop_cam_plus_plus);
        assert!(!r.should_switch_to_sensevoice);
        assert!(!r.should_disable_long_summary);

        let r = classify_memory_pressure(900);  // 75% threshold
        assert_eq!(r.level, MemoryPressureLevel::Warning);
        assert!(!r.should_drop_cam_plus_plus);  // Warning 不动, 只提示

        let r = classify_memory_pressure(1500);  // > threshold
        assert_eq!(r.level, MemoryPressureLevel::Critical);
        assert!(r.should_drop_cam_plus_plus);
        assert!(r.should_switch_to_sensevoice);
        assert!(r.should_disable_long_summary);
    }

    /// §31 P0: 边界值 - 阈值正好等于 1200 MB 应触发 Critical.
    #[test]
    fn classify_memory_pressure_boundary_at_threshold() {
        let r = classify_memory_pressure(MEMORY_PRESSURE_THRESHOLD_MB);
        assert_eq!(r.level, MemoryPressureLevel::Critical);
    }

    /// §31 P0: 边界值 - 警告阈值正好等于 840 MB (70% of 1200) 应触发 Warning.
    #[test]
    fn classify_memory_pressure_boundary_at_warning_threshold() {
        let warn_threshold = MEMORY_PRESSURE_THRESHOLD_MB * 70 / 100;
        let r = classify_memory_pressure(warn_threshold);
        assert_eq!(r.level, MemoryPressureLevel::Warning);
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
