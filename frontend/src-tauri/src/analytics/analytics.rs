// 离线会记 — analytics 模块(noop 改造版)
//
// 原 Meetily 使用云端埋点服务(已彻底删除,本产品零外网请求)。
// 本产品"100% 本地,数据不出本机",所有云端通信已彻底删除。
//
// 保留 AnalyticsConfig/AnalyticsState/AnalyticsClient 类型签名,保证
// lib.rs 中 25 个 invoke_handler 注册的 tauri::command 编译通过。
// 所有方法内部直接返回 Ok(()) / false,零网络请求。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[allow(dead_code)] // §F: legacy constant,功能未启用
const SENSITIVE_KEYS: &[&str] = &[
    "meeting_title", "meetingTitle", "meeting_name", "meetingName",
    "file_name", "filename", "file_path", "folder_path", "path",
    "source_path", "meeting_folder_path", "device_name", "user_agent",
    "email", "phone", "user_id_external", "license_key", "machine_fingerprint",
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalyticsConfig {
    pub api_key: String,
    pub host: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalyticsState {
    pub enabled: bool,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
}

pub struct AnalyticsClient {
    pub config: AnalyticsConfig,
    pub state: std::sync::Arc<std::sync::Mutex<AnalyticsState>>,
}

impl AnalyticsClient {
    pub async fn new(config: AnalyticsConfig) -> Self {
        Self {
            config,
            state: std::sync::Arc::new(std::sync::Mutex::new(AnalyticsState {
                enabled: false,
                user_id: None,
                session_id: None,
            })),
        }
    }
    pub async fn track_event(&self, _event_name: &str, _properties: Option<HashMap<String, String>>) -> Result<(), String> { Ok(()) }
    pub async fn identify(&self, _user_id: String, _properties: Option<HashMap<String, String>>) -> Result<(), String> { Ok(()) }
    pub async fn track_meeting_started(&self, _meeting_id: &str) -> Result<(), String> { Ok(()) }
    pub async fn track_recording_started(&self, _meeting_id: &str) -> Result<(), String> { Ok(()) }
    pub async fn track_recording_stopped(&self, _meeting_id: &str, _duration_seconds: Option<u64>) -> Result<(), String> { Ok(()) }
    pub async fn track_meeting_deleted(&self, _meeting_id: &str) -> Result<(), String> { Ok(()) }
    pub async fn track_settings_changed(&self, _setting_type: &str, _new_value: &str) -> Result<(), String> { Ok(()) }
    pub async fn track_feature_used(&self, _feature_name: &str, _properties: Option<HashMap<String, String>>) -> Result<(), String> { Ok(()) }
    pub async fn start_session(&self, _session_id: String) -> Result<(), String> { Ok(()) }
    pub async fn end_session(&self) -> Result<(), String> { Ok(()) }
    pub async fn track_daily_active_user(&self) -> Result<(), String> { Ok(()) }
    pub async fn track_user_first_launch(&self) -> Result<(), String> { Ok(()) }
    pub async fn track_summary_generation_started(&self, _model_provider: &str, _model_name: &str, _transcript_length: usize) -> Result<(), String> { Ok(()) }
    pub async fn track_summary_generation_completed(&self, _model_provider: &str, _model_name: &str, _success: bool, _duration_ms: Option<u64>) -> Result<(), String> { Ok(()) }
    pub async fn track_summary_regenerated(&self, _model_provider: &str, _model_name: &str) -> Result<(), String> { Ok(()) }
    pub async fn track_model_changed(&self, _old_model: &str, _new_model: &str) -> Result<(), String> { Ok(()) }
    pub async fn track_custom_prompt_used(&self, _prompt_name: &str) -> Result<(), String> { Ok(()) }
    pub async fn track_meeting_ended(&self, _transcription_provider: &str, _transcription_model: &str, _summary_provider: &str, _summary_model: &str, _total_duration_seconds: Option<f64>, _active_duration_seconds: f64, _pause_duration_seconds: f64, _microphone_device_type: &str, _system_audio_device_type: &str, _chunks_processed: u64, _transcript_segments_count: u64, _had_fatal_error: bool) -> Result<(), String> { Ok(()) }
    pub async fn track_analytics_enabled(&self) -> Result<(), String> { Ok(()) }
    pub async fn track_analytics_disabled(&self) -> Result<(), String> { Ok(()) }
    pub async fn track_analytics_transparency_viewed(&self) -> Result<(), String> { Ok(()) }
    pub async fn is_enabled(&self) -> bool { false }
    pub async fn is_session_active(&self) -> bool { false }
    pub async fn get_persistent_user_id(&self) -> Option<String> { None }
    pub async fn flush(&self) -> Result<(), String> { Ok(()) }
}
