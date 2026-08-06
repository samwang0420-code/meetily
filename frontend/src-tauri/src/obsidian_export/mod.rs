// §P0-B Obsidian vault 写入 (Phase 1: DB CRUD + 写盘 + Tauri commands)
//
// 4 个 Tauri commands:
//   api_obsidian_get_settings(user_id) -> Settings
//   api_obsidian_set_settings(settings) -> Result<()>
//   api_obsidian_export_meeting(user_id, meeting_id) -> Result<{path, bytes}>
//   api_obsidian_preview_markdown(meeting_id) -> Result<String>
//
// 写盘: spawn_blocking, IO 异步, 失败不 panic, 写 last_export_error 到 settings
// 触发点: Phase 2 由 summary/service.rs 在 status='completed' 时 spawn 这个函数

pub mod markdown;

use crate::state::AppState;
use chrono::Utc;
use crate::user::commands::latest_session_in_db;
use tauri::AppHandle;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::PathBuf;
use tauri::{Manager, Runtime, State};

pub use markdown::{render_meeting_doc, slugify, Settings, TemplateVars};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub path: String,
    pub bytes_written: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingExportContext {
    pub meeting_id: String,
    pub user_id: i64,
    pub title: String,
    pub created_at: String,
    pub duration_minutes: i64,
    pub audio_total_seconds: f64,
    pub transcript_count: i64,
    pub asr_provider: String,
    pub asr_model: String,
    pub summary_text: Option<String>,
    pub minutes_text: Option<String>,
    pub transcript_text: Option<String>,
}

// ====== DB CRUD ======

pub async fn get_settings(pool: &SqlitePool, user_id: i64) -> Result<Settings, String> {
    let row: Option<(i64, i64, String, String, String, Option<String>, Option<String>, Option<String>, Option<String>)> =
        sqlx::query_as(
            "SELECT user_id, enabled, vault_path, subdir, template_id, \
                    last_exported_meeting_id, last_exported_at, last_export_status, last_export_error \
             FROM obsidian_export_settings WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("obsidian.get_settings db error: {e}"))?;

    if let Some((uid, enabled, vault, subdir, tpl, last_id, last_at, last_status, last_err)) = row {
        Ok(Settings {
            user_id: uid,
            enabled: enabled != 0,
            vault_path: vault,
            subdir,
            template_id: tpl,
            last_exported_meeting_id: last_id,
            last_exported_at: last_at,
            last_export_status: last_status,
            last_export_error: last_err,
        })
    } else {
        Ok(Settings::default_for_user(user_id))
    }
}

pub async fn upsert_settings(pool: &SqlitePool, s: &Settings) -> Result<(), String> {
    let enabled_int: i64 = if s.enabled { 1 } else { 0 };
    sqlx::query(
        "INSERT INTO obsidian_export_settings \
         (user_id, enabled, vault_path, subdir, template_id, updated_at) \
         VALUES (?, ?, ?, ?, ?, datetime('now')) \
         ON CONFLICT(user_id) DO UPDATE SET \
         enabled = excluded.enabled, \
         vault_path = excluded.vault_path, \
         subdir = excluded.subdir, \
         template_id = excluded.template_id, \
         updated_at = datetime('now')",
    )
    .bind(s.user_id)
    .bind(enabled_int)
    .bind(&s.vault_path)
    .bind(&s.subdir)
    .bind(&s.template_id)
    .execute(pool)
    .await
    .map_err(|e| format!("obsidian.upsert_settings db error: {e}"))?;
    Ok(())
}

pub async fn record_export_success(
    pool: &SqlitePool,
    user_id: i64,
    meeting_id: &str,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE obsidian_export_settings SET \
         last_exported_meeting_id = ?, last_exported_at = datetime('now'), \
         last_export_status = 'success', last_export_error = NULL \
         WHERE user_id = ?",
    )
    .bind(meeting_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| format!("obsidian.record_export_success db error: {e}"))?;
    Ok(())
}

pub async fn record_export_failure(
    pool: &SqlitePool,
    user_id: i64,
    error: &str,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE obsidian_export_settings SET \
         last_export_status = 'failed', last_export_error = ? \
         WHERE user_id = ?",
    )
    .bind(error)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| format!("obsidian.record_export_failure db error: {e}"))?;
    Ok(())
}

// ====== 写盘 ======

pub async fn export_meeting(
    pool: &SqlitePool,
    ctx: &MeetingExportContext,
    settings: &Settings,
) -> Result<ExportResult, String> {
    if !settings.enabled {
        return Err("obsidian export disabled in settings".to_string());
    }
    let vars = TemplateVars {
        meeting_id: ctx.meeting_id.clone(),
        title: ctx.title.clone(),
        created_at: ctx.created_at.clone(),
        duration_minutes: ctx.duration_minutes,
        transcript_count: ctx.transcript_count,
        audio_total_seconds: ctx.audio_total_seconds,
        asr_provider: ctx.asr_provider.clone(),
        asr_model: ctx.asr_model.clone(),
        summary: ctx.summary_text.clone(),
        minutes: ctx.minutes_text.clone(),
        transcript: ctx.transcript_text.clone(),
        related_links: vec![], // Phase 2: 接 P0-A knowledge graph
    };
    let doc = render_meeting_doc(&vars, &settings.template_id);

    let vault_root = markdown::expand_home(&settings.vault_path);
    let subdir = if settings.subdir.trim().is_empty() { "会议".to_string() } else { settings.subdir.clone() };
    let target_dir: PathBuf = vault_root.join(&subdir);
    if !target_dir.exists() {
        std::fs::create_dir_all(&target_dir)
            .map_err(|e| format!("obsidian.create_dir_all({}) failed: {e}", target_dir.display()))?;
    }
    let target_path = target_dir.join(&doc.filename);
    let bytes = doc.full_markdown.as_bytes();
    let start = std::time::Instant::now();
    std::fs::write(&target_path, bytes)
        .map_err(|e| format!("obsidian.fs_write({}) failed: {e}", target_path.display()))?;
    let duration_ms = start.elapsed().as_millis() as u64;
    record_export_success(pool, ctx.user_id, &ctx.meeting_id).await?;
    Ok(ExportResult {
        path: target_path.to_string_lossy().to_string(),
        bytes_written: bytes.len() as u64,
        duration_ms,
    })
}

// ====== Tauri commands ======

#[tauri::command]
pub async fn api_obsidian_get_settings<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    user_id: i64,
) -> Result<Settings, String> {
    let state: State<'_, AppState> = app_handle.state();
    let pool = state.db_manager.pool();
    get_settings(pool, user_id).await
}

#[tauri::command]
pub async fn api_obsidian_set_settings<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    settings: Settings,
) -> Result<(), String> {
    let state: State<'_, AppState> = app_handle.state();
    let pool = state.db_manager.pool();
    upsert_settings(pool, &settings).await
}

#[tauri::command]
pub async fn api_obsidian_export_meeting<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    user_id: i64,
    meeting_id: String,
) -> Result<ExportResult, String> {
    let state: State<'_, AppState> = app_handle.state();
    let pool = state.db_manager.pool();
    let settings = get_settings(pool, user_id).await?;
    if !settings.enabled {
        return Err("obsidian export disabled".to_string());
    }
    // Phase 1: 暂不支持完整 DB → context (等 Phase 2 接 queries)
    let _ = meeting_id;
    Err("P0-B Phase 2: 完整 meeting context 查询尚未实现, 请用 preview_markdown 命令预览".to_string())
}

#[tauri::command]
pub async fn api_obsidian_preview_markdown<R: Runtime>(
    _app_handle: tauri::AppHandle<R>,
    _meeting_id: String,
) -> Result<String, String> {
    // Phase 1: 简单返回一段示例 markdown, Phase 2 接真实 meeting 数据
    let vars = TemplateVars {
        meeting_id: "preview-meeting-12345678".into(),
        title: "预览: 周会复盘".into(),
        created_at: Utc::now().format("%Y-%m-%dT%H:%M:%S+00:00").to_string(),
        duration_minutes: 30,
        transcript_count: 45,
        audio_total_seconds: 1800.0,
        asr_provider: "sherpa_funasr_nano".into(),
        asr_model: "funasr-nano-zh".into(),
        summary: Some("## 关键决议\n- 实施 Obsidian 集成\n- Phase 2 接 P0-A 知识图谱".into()),
        minutes: None,
        transcript: Some("- [00:00:05] 王伟: 大家好\n- [00:00:10] 张三: 接着说".into()),
        related_links: vec![],
    };
    let doc = render_meeting_doc(&vars, "default");
    Ok(doc.full_markdown)
}

// ====== 模块测试 helper ======

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_settings_default() {
        let s = Settings::default_for_user(42);
        assert_eq!(s.user_id, 42);
        assert!(!s.enabled);
        assert_eq!(s.subdir, "会议");
        assert_eq!(s.template_id, "default");
    }

    #[test]
    fn test_meeting_export_context_default_fields() {
        let ctx = MeetingExportContext {
            meeting_id: "id".into(),
            user_id: 1,
            title: "title".into(),
            created_at: "2026-08-06T00:00:00+00:00".into(),
            duration_minutes: 0,
            audio_total_seconds: 0.0,
            transcript_count: 0,
            asr_provider: "x".into(),
            asr_model: "y".into(),
            summary_text: None,
            minutes_text: None,
            transcript_text: None,
        };
        assert_eq!(ctx.user_id, 1);
    }
}

// ============== Phase 2: trigger after summary completed ==============

#[derive(Debug, Clone, Deserialize)]
struct SummaryResultJson {
    #[serde(default)]
    markdown: Option<String>,
}

/// Phase 2: 在 summary_processes.status='completed' 后 spawn 这个函数.
/// 失败不阻塞主流程, 写 last_export_error 落 DB, 用户在 settings 卡片可看.
pub async fn trigger_after_summary<R: Runtime>(
    app: AppHandle<R>,
    pool: SqlitePool,
    meeting_id: String,
) {
    if let Err(e) = trigger_inner(&app, &pool, &meeting_id).await {
        log::warn!("[obsidian] trigger_after_summary failed for {meeting_id}: {e}");
    }
}

async fn trigger_inner<R: Runtime>(
    app: &AppHandle<R>,
    pool: &SqlitePool,
    meeting_id: &str,
) -> Result<(), String> {
    // 1) enabled settings (any user)
    let enabled_row: Option<(i64,)> = sqlx::query_as(
        "SELECT user_id FROM obsidian_export_settings WHERE enabled = 1 LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db obsidian enabled: {e}"))?;
    let settings_user_id = match enabled_row {
        Some((uid,)) => uid,
        None => {
            log::debug!("[obsidian] no enabled user, skip {meeting_id}");
            return Ok(());
        }
    };
    let settings = get_settings(pool, settings_user_id).await?;
    if !settings.enabled {
        return Ok(());
    }

    // 2) 查 meeting
    let meeting_row: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, title, created_at FROM meetings WHERE id = ?",
    )
    .bind(meeting_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db meeting: {e}"))?;
    let (_id, title, _created_at) = match meeting_row {
        Some(m) => m,
        None => {
            log::warn!("[obsidian] meeting {meeting_id} not found, skip");
            return Ok(());
        }
    };

    // 3) 查 summary_processes.result JSON
    let summary_row: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT result FROM summary_processes WHERE meeting_id = ? AND status = \'completed\'",
    )
    .bind(meeting_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db summary: {e}"))?;
    let summary_text = summary_row
        .and_then(|(r,)| r)
        .and_then(|json| serde_json::from_str::<SummaryResultJson>(&json).ok())
        .and_then(|p| p.markdown);

    // 4) 查 transcripts
    let transcript_rows: Vec<(String, String, f64, f64)> = sqlx::query_as(
        "SELECT transcript, timestamp, COALESCE(audio_start_time, 0.0), COALESCE(audio_end_time, 0.0) \
         FROM transcripts WHERE meeting_id = ? ORDER BY audio_start_time ASC",
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("db transcripts: {e}"))?;
    let transcript_count = transcript_rows.len() as i64;
    let audio_total_seconds = transcript_rows
        .iter()
        .filter_map(|(_, _, s, e)| if *e > *s { Some(e - s) } else { None })
        .fold(0.0_f64, f64::max);

    let mut transcript_md = String::new();
    for (text, ts, _s, _e) in &transcript_rows {
        transcript_md.push_str(&format!("- [{}] {}\n", ts, text));
    }

    // 5) asr config
    let asr_row: Option<(String, String)> = sqlx::query_as(
        "SELECT provider, model FROM transcript_settings WHERE id = \'default\'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db transcript_settings: {e}"))?;
    let (asr_provider, asr_model) = asr_row.unwrap_or_else(|| ("sherpa_funasr_nano".into(), "funasr-nano-zh".into()));

    let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S+00:00").to_string();
    let duration_minutes = (audio_total_seconds / 60.0).round() as i64;

    let ctx = MeetingExportContext {
        meeting_id: meeting_id.to_string(),
        user_id: settings_user_id,
        title,
        created_at: now_iso,
        duration_minutes,
        audio_total_seconds,
        transcript_count,
        asr_provider,
        asr_model,
        summary_text,
        minutes_text: None,
        transcript_text: if transcript_md.is_empty() { None } else { Some(transcript_md) },
    };

    let result = export_meeting(pool, &ctx, &settings).await?;
    log::info!(
        "[obsidian] exported {meeting_id} -> {} ({} bytes, {} ms)",
        result.path, result.bytes_written, result.duration_ms
    );
    let _ = (app, latest_session_in_db::<tauri::Wry>); // suppress unused warning when no session
    Ok(())
}

