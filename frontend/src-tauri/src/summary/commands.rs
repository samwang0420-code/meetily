// §29 Pro tier gate for FunASR-Nano: pro_only_funasr_nano
use crate::database::repositories::{
    meeting::MeetingsRepository,
    summary::SummaryProcessesRepository, transcript_chunk::TranscriptChunksRepository,
};
use crate::state::AppState;
use crate::summary::metadata::{
    read_detected_summary_language_from_metadata, read_summary_language_from_metadata,
    write_detected_summary_language_to_metadata, write_summary_language_to_metadata,
};
use crate::summary::language_detection::{
    detect_summary_language, SummaryLanguageDetection,
};
use crate::summary::service::SummaryService;
use log::{error as log_error, info as log_info, warn as log_warn};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Runtime};

#[derive(Debug, Serialize, Deserialize)]
pub struct SummaryResponse {
    pub status: String,
    #[serde(rename = "meetingName")]
    pub meeting_name: Option<String>,
    pub meeting_id: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessTranscriptResponse {
    pub message: String,
    pub process_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredTranscriptEvidence {
    pub text: String,
    pub start_seconds: Option<f64>,
    pub end_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SummaryLanguageStorage {
    Metadata,
    LocalFallback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSummaryLanguagePreference {
    pub language: Option<String>,
    pub storage: SummaryLanguageStorage,
}

impl MeetingSummaryLanguagePreference {
    fn metadata(language: Option<String>) -> Self {
        Self {
            language,
            storage: SummaryLanguageStorage::Metadata,
        }
    }

    fn local_fallback() -> Self {
        Self {
            language: None,
            storage: SummaryLanguageStorage::LocalFallback,
        }
    }
}

enum MeetingFolderResolution {
    Folder(PathBuf),
    NoFolder,
}

/// Saves a meeting summary (Native SQLx implementation)
///
/// Expected format: { "markdown": "...", "summary_json": [...BlockNote blocks...] }
#[tauri::command]
pub async fn api_save_meeting_summary<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    summary: serde_json::Value,
    _auth_token: Option<String>,
) -> Result<serde_json::Value, String> {
    log_info!(
        "api_save_meeting_summary (native) called for meeting_id: {}",
        meeting_id
    );
    let pool = state.db_manager.pool();

    match SummaryProcessesRepository::update_meeting_summary(pool, &meeting_id, &summary).await {
        Ok(true) => {
            log_info!("Summary saved successfully for meeting_id: {}", meeting_id);
            Ok(serde_json::json!({
                "message": "Meeting summary saved successfully"
            }))
        }
        Ok(false) => {
            log_warn!(
                "Meeting not found or invalid JSON for meeting_id: {}",
                meeting_id
            );
            Err("Meeting not found or can't convert the json".into())
        }
        Err(e) => {
            log_error!("Failed to save meeting summary for {}: {}", meeting_id, e);
            Err(e.to_string())
        }
    }
}

/// Gets the per-meeting summary language override from metadata.json.
#[tauri::command]
pub async fn api_get_meeting_summary_language<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<MeetingSummaryLanguagePreference, String> {
    log_info!(
        "api_get_meeting_summary_language called for meeting_id: {}",
        meeting_id
    );

    match resolve_meeting_folder(state.db_manager.pool(), &meeting_id).await? {
        MeetingFolderResolution::Folder(folder) => read_summary_language_from_metadata(&folder)
            .map(MeetingSummaryLanguagePreference::metadata)
            .map_err(|e| e.to_string()),
        MeetingFolderResolution::NoFolder => Ok(MeetingSummaryLanguagePreference::local_fallback()),
    }
}

/// Saves or clears the per-meeting summary language override in metadata.json.
#[tauri::command]
pub async fn api_save_meeting_summary_language<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    summary_language: Option<String>,
) -> Result<MeetingSummaryLanguagePreference, String> {
    log_info!(
        "api_save_meeting_summary_language called for meeting_id: {}, language: {:?}",
        meeting_id,
        summary_language
    );

    match resolve_meeting_folder(state.db_manager.pool(), &meeting_id).await? {
        MeetingFolderResolution::Folder(folder) => {
            write_summary_language_to_metadata(&folder, summary_language.as_deref())
                .map_err(|e| e.to_string())?;
            read_summary_language_from_metadata(&folder)
                .map(MeetingSummaryLanguagePreference::metadata)
                .map_err(|e| e.to_string())
        }
        MeetingFolderResolution::NoFolder => Ok(MeetingSummaryLanguagePreference::local_fallback()),
    }
}

/// Gets the cached Auto-detected summary language from metadata.json.
#[tauri::command]
pub async fn api_get_meeting_detected_summary_language<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<MeetingSummaryLanguagePreference, String> {
    log_info!(
        "api_get_meeting_detected_summary_language called for meeting_id: {}",
        meeting_id
    );

    match resolve_meeting_folder(state.db_manager.pool(), &meeting_id).await? {
        MeetingFolderResolution::Folder(folder) => read_detected_summary_language_from_metadata(&folder)
            .map(MeetingSummaryLanguagePreference::metadata)
            .map_err(|e| e.to_string()),
        MeetingFolderResolution::NoFolder => Ok(MeetingSummaryLanguagePreference::local_fallback()),
    }
}

/// Saves or clears the cached Auto-detected summary language in metadata.json.
#[tauri::command]
pub async fn api_save_meeting_detected_summary_language<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    detected_summary_language: Option<String>,
) -> Result<MeetingSummaryLanguagePreference, String> {
    log_info!(
        "api_save_meeting_detected_summary_language called for meeting_id: {}, language: {:?}",
        meeting_id,
        detected_summary_language
    );

    match resolve_meeting_folder(state.db_manager.pool(), &meeting_id).await? {
        MeetingFolderResolution::Folder(folder) => {
            write_detected_summary_language_to_metadata(&folder, detected_summary_language.as_deref())
                .map_err(|e| e.to_string())?;
            read_detected_summary_language_from_metadata(&folder)
                .map(MeetingSummaryLanguagePreference::metadata)
                .map_err(|e| e.to_string())
        }
        MeetingFolderResolution::NoFolder => Ok(MeetingSummaryLanguagePreference::local_fallback()),
    }
}

/// Detects the dominant supported summary language from transcript segments.
#[tauri::command]
pub async fn api_detect_transcript_summary_language(
    transcript_texts: Vec<String>,
) -> Result<SummaryLanguageDetection, String> {
    Ok(detect_summary_language(&transcript_texts))
}

async fn resolve_meeting_folder(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
) -> Result<MeetingFolderResolution, String> {
    let meeting = MeetingsRepository::get_meeting_metadata(pool, meeting_id)
        .await
        .map_err(|e| format!("Failed to load meeting metadata: {}", e))?
        .ok_or_else(|| format!("Meeting not found: {}", meeting_id))?;

    let Some(folder_path) = meeting.folder_path.filter(|p| !p.trim().is_empty()) else {
        return Ok(MeetingFolderResolution::NoFolder);
    };

    Ok(MeetingFolderResolution::Folder(PathBuf::from(folder_path)))
}

/// Gets summary status and data (Native SQLx implementation)
///
/// Returns summary status (pending/processing/completed/failed) and parsed result data
#[tauri::command]
pub async fn api_get_summary<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    _auth_token: Option<String>,
) -> Result<SummaryResponse, String> {
    log_info!(
        "api_get_summary (native) called for meeting_id: {}",
        meeting_id
    );
    let pool = state.db_manager.pool();

    match SummaryProcessesRepository::get_summary_data_for_meeting(pool, &meeting_id).await {
        Ok(Some(process)) => {
            let status = process.status.to_lowercase();
            let error = process.error;

            // Parse result data if it exists (regardless of status)
            // This allows displaying restored summaries after cancellation or failure
            let data = if let Some(result_str) = process.result {
                match serde_json::from_str::<serde_json::Value>(&result_str) {
                    Ok(parsed) => Some(parsed),
                    Err(e) => {
                        log_error!("Failed to parse summary result JSON: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            // Fetch meeting title from database
            let meeting_name = match MeetingsRepository::get_meeting(pool, &meeting_id).await {
                Ok(Some(meeting_details)) => {
                    log_info!("Fetched meeting title: {}", &meeting_details.title);
                    Some(meeting_details.title)
                }
                Ok(None) => {
                    log_warn!("Meeting not found for meeting_id: {}", meeting_id);
                    None
                }
                Err(e) => {
                    log_error!("Failed to fetch meeting title: {}", e);
                    None
                }
            };

            let response = SummaryResponse {
                status: status.clone(),
                meeting_name,
                meeting_id: meeting_id.clone(),
                start: process.start_time.map(|t| t.to_rfc3339()),
                end: process.end_time.map(|t| t.to_rfc3339()),
                data,
                error,
            };

            log_info!(
                "Summary status for {}: {}, has_data: {}, meeting_name: {:?}",
                meeting_id,
                status,
                response.data.is_some(),
                response.meeting_name
            );
            Ok(response)
        }
        Ok(None) => {
            log_info!("No summary process found for meeting_id: {}", meeting_id);

            // Still fetch meeting title for idle state
            let meeting_name = match MeetingsRepository::get_meeting(pool, &meeting_id).await {
                Ok(Some(meeting_details)) => Some(meeting_details.title),
                _ => None,
            };

            Ok(SummaryResponse {
                status: "idle".to_string(),
                meeting_name,
                meeting_id,
                start: None,
                end: None,
                data: None,
                error: None,
            })
        }
        Err(e) => {
            log_error!("Error retrieving summary for {}: {}", meeting_id, e);
            Err(format!("Failed to retrieve summary: {}", e))
        }
    }
}

/// Processes transcript and generates summary (Native SQLx implementation)
///
/// Spawns a background task and returns immediately with process_id
#[tauri::command]
pub async fn api_process_transcript<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    text: String,
    model: String,
    model_name: String,
    meeting_id: Option<String>,
    _chunk_size: Option<i32>,
    _overlap: Option<i32>,
    custom_prompt: Option<String>,
    template_id: Option<String>,
    summary_language: Option<String>,
    evidence: Option<Vec<StructuredTranscriptEvidence>>,
    _auth_token: Option<String>,
    // §169: 强制 bypass summary cache, 重新调用 LLM (用户主动 "重新生成" 时为 true)
    force_fresh: Option<bool>,
) -> Result<ProcessTranscriptResponse, String> {
    use uuid::Uuid;

    let m_id = meeting_id.unwrap_or_else(|| format!("meeting-{}", Uuid::new_v4()));
    log_info!(
        "api_process_transcript (native) called for meeting_id: {}, model: {}",
        &m_id,
        &model
    );

    let pool = state.db_manager.pool().clone();
    let final_prompt = custom_prompt.unwrap_or_else(|| "".to_string());
    // 取出 template_id 引用 / 备份 original, 避免被 unwrap_or_else move 后无法 §123 持久化
    let template_id_for_persist = template_id.clone();
    let final_template_id = template_id.unwrap_or_else(|| "daily_standup".to_string());

    // Normalise empty / whitespace-only to None so "" and null behave identically
    let summary_language = summary_language.and_then(|s| {
        let t = s.trim();
        if t.is_empty() { None } else { Some(t.to_string()) }
    });

    let structured_evidence = evidence.unwrap_or_default();

    // Create or reset the process entry in the database
    SummaryProcessesRepository::create_or_reset_process(&pool, &m_id)
        .await
        .map_err(|e| format!("Failed to initialize process: {}", e))?;

    log_info!("✓ Summary process initialized for meeting_id: {}", &m_id);

    // §123: 持久化用户选过的模板 ID. 下次进入会议详情默认显示同一模板.
    if let Some(tid) = template_id_for_persist.as_deref() {
        if !tid.trim().is_empty() {
            if let Err(e) = sqlx::query("UPDATE meetings SET template_id = ?1, updated_at = ?2 WHERE id = ?3")
                .bind(tid)
                .bind(chrono::Utc::now().to_rfc3339())
                .bind(&m_id)
                .execute(&pool)
                .await
            {
                log_warn!("§123 failed to persist meeting.template_id={tid} for {m_id}: {e}");
            }
        }
    }

    // Save transcript chunks data (matching Python backend behavior)
    let chunk_size = _chunk_size.unwrap_or(40000);
    let overlap = _overlap.unwrap_or(1000);

    TranscriptChunksRepository::save_transcript_data(
        &pool,
        &m_id,
        &text,
        &model,
        &model_name,
        chunk_size,
        overlap,
    )
    .await
    .map_err(|e| format!("Failed to save transcript data: {}", e))?;

    log_info!("✓ Transcript chunks saved for meeting_id: {}", &m_id);

    // Spawn background task for actual processing
    // §152 P0-1: panic 防护 — background task panic 必须 update_process_failed,
    // 否则 DB 永远卡 PENDING, 用户无错误提示也无法取消
    let meeting_id_clone = m_id.clone();
    tauri::async_runtime::spawn(async move {
        use futures_util::FutureExt;
        use std::panic::AssertUnwindSafe;

        let panic_result = AssertUnwindSafe(
            SummaryService::process_transcript_background(
                app.clone(),
                pool.clone(),
                meeting_id_clone.clone(),
                text,
                model,
                model_name,
                final_prompt,
                final_template_id,
                summary_language,
                structured_evidence,
                force_fresh.unwrap_or(false), // §169: 默认 false, 保留 cache; regenerate 时前端传 true
            ),
        )
        .catch_unwind()
        .await;

        match panic_result {
            Ok(()) => {
                log_info!(
                    "✓ Background task finished cleanly for meeting_id: {}",
                    &meeting_id_clone
                );
            }
            Err(panic_payload) => {
                // panic 时: 把错误写进 DB + 清理 CANCELLATION_REGISTRY
                let panic_msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_payload.downcast_ref::<&'static str>() {
                    s.to_string()
                } else {
                    "unknown panic in process_transcript_background".to_string()
                };
                log_error!(
                    "§152 P0-1 Background task PANICKED for meeting_id={}: {}",
                    &meeting_id_clone,
                    &panic_msg
                );
                if let Err(e) = SummaryProcessesRepository::update_process_failed(
                    &pool,
                    &meeting_id_clone,
                    &format!("Background task panicked: {}", panic_msg),
                )
                .await
                {
                    log_error!(
                        "§152 P0-1 Failed to write panic error to DB for {}: {}",
                        &meeting_id_clone,
                        e
                    );
                }
                // cleanup CANCELLATION_REGISTRY 让后续 cancel_summary 不再误以为还在跑
                SummaryService::cleanup_cancellation_token(&meeting_id_clone);
            }
        }
    });

    log_info!("🚀 Background task spawned (with §152 P0-1 panic guard) for meeting_id: {}", &m_id);

    Ok(ProcessTranscriptResponse {
        message: "Summary generation started".to_string(),
        process_id: m_id,
    })
}

/// Cancels an ongoing summary generation process
///
/// This command triggers the cancellation token for the specified meeting,
/// stopping the summary generation gracefully.
#[tauri::command]
pub async fn api_cancel_summary<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<serde_json::Value, String> {
    log_info!("api_cancel_summary called for meeting_id: {}", meeting_id);

    // §152 P0-3-A: 即使 token 不在 CANCELLATION_REGISTRY (panic / 已完成 / 已退出),
    // 用户主动点 stop 必须让 DB status 真改为 cancelled.
    // 否则 PANIC 后永远卡 PENDING, 用户无错误提示, 也不能 stop.
    let cancelled = SummaryService::cancel_summary(&meeting_id);

    let pool = state.db_manager.pool();
    if let Err(e) = SummaryProcessesRepository::update_process_cancelled(pool, &meeting_id).await {
        log_error!(
            "Failed to update DB status to cancelled for {}: {}",
            meeting_id,
            e
        );
        return Err(format!("Failed to update cancellation status: {}", e));
    }

    if cancelled {
        log_info!(
            "Successfully cancelled summary generation for meeting_id: {}",
            meeting_id
        );
        Ok(serde_json::json!({
            "message": "Summary generation cancelled successfully",
            "meeting_id": meeting_id,
        }))
    } else {
        // §152 P0-3-A: token 没找到 (可能 panic 后 cleanup 了 / 已完成), 但 DB 已强制 update
        log_warn!(
            "No active token found for {}, but DB status forced to cancelled",
            meeting_id
        );
        Ok(serde_json::json!({
            "message": "Summary status forced to cancelled",
            "meeting_id": meeting_id,
        }))
    }
}

// === §135 多次生成摘要历史 ===

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct SummaryHistoryEntry {
    pub id: i64,
    pub meeting_id: String,
    pub template_id: Option<String>,
    pub template_name: Option<String>,
    pub model_name: Option<String>,
    pub chunk_count: i64,
    pub processing_time: f64,
    pub created_at: String,
    pub archived_at: String,
    pub backup_reason: String,
    /// 完整 result JSON 字符串 (前端解析)
    pub result_json: String,
}

/// §135: 拉该会议所有历史摘要 (按 archived_at 倒序, 最新归档在前)
#[tauri::command]
pub async fn api_summary_history<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<Vec<SummaryHistoryEntry>, String> {
    let pool = state.db_manager.pool();
    let rows: Vec<SummaryHistoryEntry> = sqlx::query_as::<_, SummaryHistoryEntry>(
        r#"
        SELECT id, meeting_id, template_id, template_name, model_name,
               chunk_count, processing_time,
               created_at, archived_at, backup_reason, result_json
        FROM summary_history
        WHERE meeting_id = ?1
        ORDER BY archived_at DESC
        "#,
    )
    .bind(&meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("api_summary_history query: {e}"))?;
    Ok(rows)
}

/// §135: 拉当前 summary_processes 的 result (最新) — 与 history 区分开
#[tauri::command]
pub async fn api_summary_current<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<Option<serde_json::Value>, String> {
    let pool = state.db_manager.pool();
    let row: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT result FROM summary_processes WHERE meeting_id = ?1"
    )
    .bind(&meeting_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("api_summary_current query: {e}"))?;
    Ok(row.and_then(|(r,)| r.and_then(|s| serde_json::from_str(&s).ok())))
}

/// §135: 拉某条 history 的 result (前端切换历史摘要时调)
#[tauri::command]
pub async fn api_summary_history_get<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    history_id: i64,
) -> Result<Option<serde_json::Value>, String> {
    let pool = state.db_manager.pool();
    let row: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT result_json FROM summary_history WHERE id = ?1"
    )
    .bind(history_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("api_summary_history_get query: {e}"))?;
    Ok(row.and_then(|(r,)| r.and_then(|s| serde_json::from_str(&s).ok())))
}
