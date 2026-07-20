// 离线会记 — analytics::commands (v0.7.0+ 本地落库版)
//
// 历史: 原 Meetily 的 25 个 analytics 事件都通过 PostHog 上传到 us.i.posthog.com。
// v0.6.x: 改成全部 noop (函数签名保留以兼容 lib.rs invoke_handler)。
// v0.7.0+: 关键事件落本地 SQLite analytics_events 表 (user_id, event_name, properties_json, created_at)。
//          完全本地, 零网络请求, 0 隐私风险. 客服看使用模式 / bug 频率靠这张表.
//
// schema (由 setup.rs 启动时建好):
//   CREATE TABLE analytics_events (
//     id INTEGER PRIMARY KEY AUTOINCREMENT,
//     user_id INTEGER,
//     event_name TEXT NOT NULL,
//     properties_json TEXT,
//     created_at TEXT NOT NULL DEFAULT (datetime('now'))
//   )

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{command, AppHandle, Manager, Runtime};
use crate::state::AppState;

/// v0.7.0+: 全局 opt-in 标志. 默认 false (尊重隐私承诺).
/// `init_analytics()` 设为 true, `disable_analytics()` 设为 false.
/// `write_event` 检查这个标志 — 用户关闭时一个事件都不写库.
static ANALYTICS_OPT_IN: AtomicBool = AtomicBool::new(false);

fn db_pool<R: Runtime>(app: &AppHandle<R>) -> Result<sqlx::SqlitePool, String> {
    let state: tauri::State<AppState> = app.state();
    Ok(state.db_manager.pool().clone())
}

/// v0.7.0+: 内部 helper. 把事件写 analytics_events.
/// - opt-in 检查: 用户在 settings 关了就不写 (合规承诺)
/// - 失败静默 (不阻塞主流程)
async fn write_event<R: Runtime>(
    app: &AppHandle<R>,
    event_name: &str,
    properties: Option<&HashMap<String, String>>,
) {
    // v0.7.0+ 修复: opt-in 关闭时直接 return, 不写库
    if !ANALYTICS_OPT_IN.load(Ordering::Relaxed) {
        return;
    }
    if let Ok(pool) = db_pool(app) {
        let props = properties
            .map(|m| serde_json::to_string(m).unwrap_or_else(|_| "{}".to_string()));
        let _ = sqlx::query(
            "INSERT INTO analytics_events (event_name, properties_json) VALUES (?1, ?2)",
        )
        .bind(event_name)
        .bind(props)
        .execute(&pool)
        .await;
    }
}

/// v0.7.0+: 用户在 settings 打开"本地行为分析"开关时调用
#[command]
pub async fn init_analytics() -> Result<(), String> {
    ANALYTICS_OPT_IN.store(true, Ordering::Relaxed);
    Ok(())
}

/// v0.7.0+: 用户关闭"本地行为分析"开关时调用
#[command]
pub async fn disable_analytics() -> Result<(), String> {
    ANALYTICS_OPT_IN.store(false, Ordering::Relaxed);
    Ok(())
}

#[command]
pub async fn track_event<R: Runtime>(
    app: AppHandle<R>,
    event_name: String,
    properties: Option<HashMap<String, String>>,
) -> Result<(), String> {
    write_event(&app, &event_name, properties.as_ref()).await;
    Ok(())
}

#[command]
pub async fn identify_user<R: Runtime>(
    app: AppHandle<R>,
    user_id: String,
    properties: Option<HashMap<String, String>>,
) -> Result<(), String> {
    let mut p = properties.unwrap_or_default();
    p.insert("identify_target".to_string(), user_id);
    write_event(&app, "identify_user", Some(&p)).await;
    Ok(())
}

#[command]
pub async fn track_meeting_started<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
) -> Result<(), String> {
    let p: HashMap<String, String> = [("meeting_id".to_string(), meeting_id)].into_iter().collect();
    write_event(&app, "meeting_started", Some(&p)).await;
    Ok(())
}

#[command]
pub async fn track_recording_started<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
) -> Result<(), String> {
    let p: HashMap<String, String> = [("meeting_id".to_string(), meeting_id)].into_iter().collect();
    write_event(&app, "recording_started", Some(&p)).await;
    Ok(())
}

#[command]
pub async fn track_recording_stopped<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
    duration_seconds: Option<u64>,
) -> Result<(), String> {
    let mut p = HashMap::new();
    p.insert("meeting_id".to_string(), meeting_id);
    if let Some(d) = duration_seconds {
        p.insert("duration_s".to_string(), d.to_string());
    }
    write_event(&app, "recording_stopped", Some(&p)).await;
    Ok(())
}

#[command]
pub async fn track_meeting_deleted<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
) -> Result<(), String> {
    let p: HashMap<String, String> = [("meeting_id".to_string(), meeting_id)].into_iter().collect();
    write_event(&app, "meeting_deleted", Some(&p)).await;
    Ok(())
}

#[command]
pub async fn track_settings_changed<R: Runtime>(
    app: AppHandle<R>,
    setting_type: String,
    new_value: String,
) -> Result<(), String> {
    let mut p = HashMap::new();
    p.insert("setting_type".to_string(), setting_type);
    p.insert("new_value".to_string(), new_value);
    write_event(&app, "settings_changed", Some(&p)).await;
    Ok(())
}

#[command]
pub async fn track_feature_used<R: Runtime>(
    app: AppHandle<R>,
    feature_name: String,
) -> Result<(), String> {
    let p: HashMap<String, String> = [("feature".to_string(), feature_name)].into_iter().collect();
    write_event(&app, "feature_used", Some(&p)).await;
    Ok(())
}

#[command]
pub async fn is_analytics_enabled() -> bool { ANALYTICS_OPT_IN.load(Ordering::Relaxed) }

#[command]
pub async fn start_analytics_session<R: Runtime>(
    app: AppHandle<R>,
    user_id: String,
) -> Result<String, String> {
    let sid = uuid::Uuid::new_v4().to_string();
    let mut p = HashMap::new();
    p.insert("user_id".to_string(), user_id);
    p.insert("session_id".to_string(), sid.clone());
    write_event(&app, "session_start", Some(&p)).await;
    Ok(sid)
}

#[command]
pub async fn end_analytics_session<R: Runtime>(
    app: AppHandle<R>,
) -> Result<(), String> {
    write_event(&app, "session_end", None).await;
    Ok(())
}

#[command]
pub async fn track_daily_active_user<R: Runtime>(
    app: AppHandle<R>,
) -> Result<(), String> {
    write_event(&app, "daily_active", None).await;
    Ok(())
}

#[command]
pub async fn track_user_first_launch<R: Runtime>(
    app: AppHandle<R>,
) -> Result<(), String> {
    write_event(&app, "first_launch", None).await;
    Ok(())
}

#[command]
pub async fn track_summary_generation_started<R: Runtime>(
    app: AppHandle<R>,
    model_provider: String,
    model_name: String,
    transcript_length: usize,
) -> Result<(), String> {
    let mut p = HashMap::new();
    p.insert("model_provider".to_string(), model_provider);
    p.insert("model_name".to_string(), model_name);
    p.insert("transcript_length".to_string(), transcript_length.to_string());
    write_event(&app, "summary_started", Some(&p)).await;
    Ok(())
}

#[command]
pub async fn track_summary_generation_completed<R: Runtime>(
    app: AppHandle<R>,
    model_provider: String,
    model_name: String,
    success: bool,
    duration_seconds: Option<u64>,
    error_message: Option<String>,
) -> Result<(), String> {
    let mut p = HashMap::new();
    p.insert("model_provider".to_string(), model_provider);
    p.insert("model_name".to_string(), model_name);
    p.insert("success".to_string(), success.to_string());
    if let Some(d) = duration_seconds {
        p.insert("duration_s".to_string(), d.to_string());
    }
    if let Some(e) = error_message {
        p.insert("error".to_string(), e);
    }
    write_event(&app, "summary_completed", Some(&p)).await;
    Ok(())
}

#[command]
pub async fn track_summary_regenerated<R: Runtime>(
    app: AppHandle<R>,
    model_provider: String,
    model_name: String,
) -> Result<(), String> {
    let mut p = HashMap::new();
    p.insert("model_provider".to_string(), model_provider);
    p.insert("model_name".to_string(), model_name);
    write_event(&app, "summary_regenerated", Some(&p)).await;
    Ok(())
}

#[command]
pub async fn track_model_changed<R: Runtime>(
    app: AppHandle<R>,
    old_provider: String,
    old_model: String,
    new_provider: String,
    new_model: String,
) -> Result<(), String> {
    let mut p = HashMap::new();
    p.insert("old_provider".to_string(), old_provider);
    p.insert("old_model".to_string(), old_model);
    p.insert("new_provider".to_string(), new_provider);
    p.insert("new_model".to_string(), new_model);
    write_event(&app, "model_changed", Some(&p)).await;
    Ok(())
}

#[command]
pub async fn track_custom_prompt_used<R: Runtime>(
    app: AppHandle<R>,
    prompt_length: usize,
) -> Result<(), String> {
    let mut p = HashMap::new();
    p.insert("prompt_length".to_string(), prompt_length.to_string());
    write_event(&app, "custom_prompt_used", Some(&p)).await;
    Ok(())
}

#[command]
pub async fn track_meeting_ended<R: Runtime>(
    app: AppHandle<R>,
    transcription_provider: String,
    transcription_model: String,
    summary_provider: String,
    summary_model: String,
    total_duration_seconds: Option<f64>,
    active_duration_seconds: f64,
    pause_duration_seconds: f64,
    microphone_device_type: String,
    system_audio_device_type: String,
    chunks_processed: u64,
    transcript_segments_count: u64,
    had_fatal_error: bool,
) -> Result<(), String> {
    let mut p = HashMap::new();
    p.insert("transcription_provider".to_string(), transcription_provider);
    p.insert("transcription_model".to_string(), transcription_model);
    p.insert("summary_provider".to_string(), summary_provider);
    p.insert("summary_model".to_string(), summary_model);
    if let Some(d) = total_duration_seconds {
        p.insert("total_duration_s".to_string(), format!("{:.1}", d));
    }
    p.insert("active_duration_s".to_string(), format!("{:.1}", active_duration_seconds));
    p.insert("pause_duration_s".to_string(), format!("{:.1}", pause_duration_seconds));
    p.insert("mic".to_string(), microphone_device_type);
    p.insert("sys".to_string(), system_audio_device_type);
    p.insert("chunks".to_string(), chunks_processed.to_string());
    p.insert("segments".to_string(), transcript_segments_count.to_string());
    p.insert("fatal".to_string(), had_fatal_error.to_string());
    write_event(&app, "meeting_ended", Some(&p)).await;
    Ok(())
}

#[command]
pub async fn track_analytics_enabled<R: Runtime>(
    app: AppHandle<R>,
) -> Result<(), String> {
    write_event(&app, "analytics_enabled", None).await;
    Ok(())
}

#[command]
pub async fn track_analytics_disabled<R: Runtime>(
    app: AppHandle<R>,
) -> Result<(), String> {
    write_event(&app, "analytics_disabled", None).await;
    Ok(())
}

#[command]
pub async fn track_analytics_transparency_viewed<R: Runtime>(
    app: AppHandle<R>,
) -> Result<(), String> {
    write_event(&app, "analytics_transparency", None).await;
    Ok(())
}

#[command]
pub async fn is_analytics_session_active() -> bool { ANALYTICS_OPT_IN.load(Ordering::Relaxed) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opt_in_flag_toggles() {
        // 初始 false
        ANALYTICS_OPT_IN.store(false, Ordering::Relaxed);
        assert_eq!(ANALYTICS_OPT_IN.load(Ordering::Relaxed), false);

        // 开启
        ANALYTICS_OPT_IN.store(true, Ordering::Relaxed);
        assert_eq!(ANALYTICS_OPT_IN.load(Ordering::Relaxed), true);

        // 关闭
        ANALYTICS_OPT_IN.store(false, Ordering::Relaxed);
        assert_eq!(ANALYTICS_OPT_IN.load(Ordering::Relaxed), false);
    }

    #[tokio::test]
    async fn test_is_analytics_enabled_returns_flag() {
        // opt-out 时返回 false
        ANALYTICS_OPT_IN.store(false, Ordering::Relaxed);
        assert_eq!(is_analytics_enabled().await, false);

        // opt-in 时返回 true
        ANALYTICS_OPT_IN.store(true, Ordering::Relaxed);
        assert_eq!(is_analytics_enabled().await, true);

        // 恢复默认
        ANALYTICS_OPT_IN.store(false, Ordering::Relaxed);
    }
}
