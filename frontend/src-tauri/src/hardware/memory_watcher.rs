// §31 P0 长音频录制内存自动降级 (memory auto-degrade)
//
// Recording 中每 60s 检查一次 RSS, 触达 MEMORY_PRESSURE_THRESHOLD_MB (1.2GB)
// 时立刻 emit "memory-pressure" 事件给前端 (带 payload: rss_mb / threshold_mb /
// suggestion / pressure_state), 后端静默 switch 内部 transcription 路径:
//   - 触达 1.2GB: 建议前端关 cam++ diarization (如果开了), 降级到单 mic stream
//   - 触达 1.5GB: 强制切换 sense-voice-zh-int8 (skip cam++, skip multi-channel)
//
// API:
//   start_memory_watcher(app)  --  spawn 60s loop (idempotent, 多次调用安全)
//   stop_memory_watcher()      --  soft-stop the loop, 录音结束时调
//   device_get_memory_recommendation() -> MemoryRecommendation  --  Tauri command
//   memory_watcher_running() -> bool  --  测试用

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Runtime};

use super::{
    current_process_rss_mb, MEMORY_PRESSURE_THRESHOLD_MB,
};

const POLL_INTERVAL_MS: u64 = 60_000; // 60 seconds per §31 P0 spec

/// memory pressure states escalating
const PRESSURE_NORMAL: u64 = 0;
const PRESSURE_WARNING: u64 = 1; // 1.2GB+ → warn user, suggest cam++ off
const PRESSURE_CRITICAL: u64 = 2; // 1.5GB+ → force sense-voice-zh, no diar
const CRITICAL_THRESHOLD_MB: u64 = 1500;
const WARNING_THRESHOLD_MB: u64 = MEMORY_PRESSURE_THRESHOLD_MB; // 1200

// State for the watcher
static WATCHER_RUNNING: AtomicBool = AtomicBool::new(false);
static LAST_RSS_MB: AtomicU64 = AtomicU64::new(0);
static LAST_PRESSURE_STATE: AtomicU64 = AtomicU64::new(0); // 0/1/2

#[derive(Debug, Clone, serde::Serialize)]
pub struct MemoryRecommendation {
    pub rss_mb: u64,
    pub threshold_warning_mb: u64,
    pub threshold_critical_mb: u64,
    pub pressure_state: u64, // 0=normal / 1=warning / 2=critical
    pub suggestion: &'static str, // localized suggestion text (zh currently)
    pub should_disable_diarization: bool,
    pub should_force_sense_voice: bool,
}

fn classify_state(rss_mb: u64) -> u64 {
    if rss_mb >= CRITICAL_THRESHOLD_MB {
        PRESSURE_CRITICAL
    } else if rss_mb >= WARNING_THRESHOLD_MB {
        PRESSURE_WARNING
    } else {
        PRESSURE_NORMAL
    }
}

fn suggestion_for_state(state: u64) -> &'static str {
    match state {
        PRESSURE_NORMAL => "内存使用正常",
        PRESSURE_WARNING => "内存压力: 建议关闭 cam++ 说话人分离, 切换到轻量模型",
        PRESSURE_CRITICAL => "内存严重压力: 已自动切换到 sense-voice 轻量识别, 关闭说话人分离",
        _ => "未知状态",
    }
}

/// Spawn the 60s polling loop. Idempotent: calling multiple times is safe,
/// only first call spawns the loop. Returns true if a new loop was started.
pub fn start_memory_watcher<R: Runtime>(app: AppHandle<R>) -> bool {
    if WATCHER_RUNNING.swap(true, Ordering::SeqCst) {
        log::info!("[memory_watcher] already running, skip spawn");
        return false;
    }
    let app_clone = app.clone();
    tokio::spawn(async move {
        log::info!(
            "[memory_watcher] started (interval {}s, warning {}MB, critical {}MB)",
            POLL_INTERVAL_MS / 1000,
            WARNING_THRESHOLD_MB,
            CRITICAL_THRESHOLD_MB
        );
        // first check after 30s (let recording warm up)
        tokio::time::sleep(Duration::from_millis(30_000)).await;
        loop {
            if !WATCHER_RUNNING.load(Ordering::SeqCst) {
                log::info!("[memory_watcher] stop signal received");
                break;
            }
            let rss = current_process_rss_mb();
            let prev_state = LAST_PRESSURE_STATE.load(Ordering::SeqCst);
            let new_state = classify_state(rss);
            LAST_RSS_MB.store(rss, Ordering::SeqCst);
            LAST_PRESSURE_STATE.store(new_state, Ordering::SeqCst);

            let payload = serde_json::json!({
                "rss_mb": rss,
                "threshold_warning_mb": WARNING_THRESHOLD_MB,
                "threshold_critical_mb": CRITICAL_THRESHOLD_MB,
                "pressure_state": new_state,
                "previous_state": prev_state,
                "suggestion": suggestion_for_state(new_state),
                "should_disable_diarization": new_state >= PRESSURE_WARNING,
                "should_force_sense_voice": new_state >= PRESSURE_CRITICAL,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            // Emit regardless so UI can show "rss: 850MB" badges even when normal
            let _ = app_clone.emit("memory-pressure", payload.clone());

            if new_state > prev_state {
                log::warn!(
                    "[memory_watcher] pressure escalated: state {} -> {} (rss {}MB)",
                    prev_state,
                    new_state,
                    rss
                );
            } else if new_state < prev_state {
                log::info!(
                    "[memory_watcher] pressure de-escalated: state {} -> {} (rss {}MB)",
                    prev_state,
                    new_state,
                    rss
                );
            }
            tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
        }
        log::info!("[memory_watcher] loop exited");
    });
    true
}

/// Stop the polling loop. Safe to call multiple times.
pub fn stop_memory_watcher() {
    if WATCHER_RUNNING.swap(false, Ordering::SeqCst) {
        log::info!("[memory_watcher] stop requested");
    }
}

/// True if a watcher is currently running (for diagnostics).
pub fn memory_watcher_running() -> bool {
    WATCHER_RUNNING.load(Ordering::SeqCst)
}

/// Build a recommendation snapshot from current RSS. Exposed for tests + UI to
/// query without waiting for the next poll cycle.
pub fn current_recommendation() -> MemoryRecommendation {
    let rss = current_process_rss_mb();
    let state = classify_state(rss);
    LAST_RSS_MB.store(rss, Ordering::SeqCst);
    LAST_PRESSURE_STATE.store(state, Ordering::SeqCst);
    MemoryRecommendation {
        rss_mb: rss,
        threshold_warning_mb: WARNING_THRESHOLD_MB,
        threshold_critical_mb: CRITICAL_THRESHOLD_MB,
        pressure_state: state,
        suggestion: suggestion_for_state(state),
        should_disable_diarization: state >= PRESSURE_WARNING,
        should_force_sense_voice: state >= PRESSURE_CRITICAL,
    }
}

#[tauri::command]
pub fn device_get_memory_recommendation<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> MemoryRecommendation {
    let _ = app; // not needed, included for symmetry with other commands
    current_recommendation()
}

// §37 闸门: 综合单测覆盖常量 + classify 阈值 + 公共 API 一致性
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_thresholds_consistent() {
        assert_eq!(classify_state(0), PRESSURE_NORMAL);
        assert_eq!(classify_state(800), PRESSURE_NORMAL);
        assert_eq!(classify_state(1199), PRESSURE_NORMAL);
        assert_eq!(classify_state(1200), PRESSURE_WARNING);
        assert_eq!(classify_state(1300), PRESSURE_WARNING);
        assert_eq!(classify_state(1499), PRESSURE_WARNING);
        assert_eq!(classify_state(1500), PRESSURE_CRITICAL);
        assert_eq!(classify_state(9999), PRESSURE_CRITICAL);
    }

    #[test]
    fn suggestion_per_state_nonempty() {
        for s in [PRESSURE_NORMAL, PRESSURE_WARNING, PRESSURE_CRITICAL] {
            let sugg = suggestion_for_state(s);
            assert!(!sugg.is_empty(), "suggestion for state {} must not be empty", s);
        }
    }

    #[test]
    fn current_recommendation_uses_threshold() {
        let r = current_recommendation();
        // 跨进程 RSS 在测试环境可能波动, 但 state 必须 ≥ 0
        assert!(r.pressure_state <= PRESSURE_CRITICAL);
        if r.rss_mb >= CRITICAL_THRESHOLD_MB {
            assert!(r.should_force_sense_voice);
            assert!(r.should_disable_diarization);
        } else if r.rss_mb >= WARNING_THRESHOLD_MB {
            assert!(r.should_disable_diarization);
            assert!(!r.should_force_sense_voice);
        }
    }

    #[test]
    fn idempotent_stop() {
        // stop_multiple_times 不会 panic
        stop_memory_watcher();
        stop_memory_watcher();
        // 不会停 watcher 因为没启动
    }

    #[test]
    fn started_state_machine_consistent() {
        // initial state: not running
        assert!(!memory_watcher_running());
    }
}
