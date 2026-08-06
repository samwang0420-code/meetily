// §81 P2-A 行动项可点击完成
//
// schema (see migration 20260807000000):
//   action_items(id, meeting_id, item_index, content, done 0/1, created_at, updated_at)
//
// service.rs:701 摘要完成时 spawn extract_action_items_from_summary → 把
// summary_processes.result JSON 里 action_items.blocks 拆行 INSERT 进表.
// Tauri command:
//   api_action_item_list(meeting_id) -> Vec<ActionItem>
//   api_action_item_toggle(id, done)  -> ()

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::{Emitter, Manager, Runtime, State};

use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionItem {
    pub id: i64,
    pub meeting_id: String,
    pub item_index: i64,
    pub content: String,
    pub done: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
struct SummaryBlocks {
    summary: Option<SummarySection>,
    raw_summary: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SummarySection {
    action_items: Option<BlocksSection>,
}

#[derive(Debug, Deserialize)]
struct BlocksSection {
    blocks: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Deserialize)]
struct ContentBlock {
    content: String,
}

/// 摘要完成后由 service.rs spawn 调用.
/// 从 summary_processes.result 取 action_items.blocks 拆行 INSERT.
/// 已经在 action_items 表里 (meeting_id, item_index) 存在的行不动 (upsert).
pub async fn extract_action_items_from_summary<R: Runtime>(
    app: tauri::AppHandle<R>,
    pool: SqlitePool,
    meeting_id: String,
) {
    // 读 summary_processes.result (JSON 字符串)
    let result_json: Option<String> = match sqlx::query_scalar(
        "SELECT result FROM summary_processes WHERE meeting_id = ?1 ORDER BY updated_at DESC LIMIT 1",
    )
    .bind(&meeting_id)
    .fetch_optional(&pool)
    .await
    {
        Ok(v) => v,
        Err(e) => {
            log::warn!("[action_items] {meeting_id} read summary_processes error: {e}");
            return;
        }
    };
    let Some(json_str) = result_json else {
        log::info!("[action_items] {meeting_id} no summary_processes.result, skip");
        return;
    };

    // Parse JSON (best-effort)
    let parsed: SummaryBlocks = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("[action_items] {meeting_id} parse summary JSON error: {e}");
            return;
        }
    };
    let action_items_blocks = parsed
        .summary
        .as_ref()
        .and_then(|s| s.action_items.as_ref())
        .map(|b| b.blocks.clone())
        .unwrap_or_default();
    if action_items_blocks.is_empty() {
        log::info!("[action_items] {meeting_id} no action_items blocks");
        return;
    }
    let now = Utc::now().to_rfc3339();
    let mut inserted = 0usize;
    for (idx, block) in action_items_blocks.iter().enumerate() {
        let content = block.content.trim();
        if content.is_empty() {
            continue;
        }
        // INSERT OR IGNORE — 已经存在的 (meeting_id, item_index) 行不动,
        // 保留用户已 toggle 的 done 状态, 不覆盖
        let res = sqlx::query(
            "INSERT OR IGNORE INTO action_items (meeting_id, item_index, content, done, created_at, updated_at) \
             VALUES (?1, ?2, ?3, 0, ?4, ?4)",
        )
        .bind(&meeting_id)
        .bind(idx as i64)
        .bind(content)
        .bind(&now)
        .execute(&pool)
        .await;
        match res {
            Ok(r) if r.rows_affected() > 0 => inserted += 1,
            Ok(_) => {}
            Err(e) => log::warn!("[action_items] insert #{idx} failed: {e}"),
        }
    }
    log::info!(
        "[action_items] {meeting_id} extracted {} / {} items",
        inserted,
        action_items_blocks.len()
    );

    // 前端 nudge: emit "meeting-updated" event 让 SummaryPanel 重读
    let _ = app.emit("action-items-updated", serde_json::json!({
        "meeting_id": meeting_id,
        "count": action_items_blocks.len(),
    }));
}

pub async fn list_action_items(
    pool: &SqlitePool,
    meeting_id: &str,
) -> Result<Vec<ActionItem>, String> {
    let rows: Vec<(i64, String, i64, String, i64, String, String)> = sqlx::query_as(
        "SELECT id, meeting_id, item_index, content, done, created_at, updated_at \
         FROM action_items WHERE meeting_id = ?1 ORDER BY item_index",
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("list_action_items: {e}"))?;
    Ok(rows
        .into_iter()
        .map(|(id, mid, idx, content, done, created, updated)| ActionItem {
            id,
            meeting_id: mid,
            item_index: idx,
            content,
            done: done != 0,
            created_at: created,
            updated_at: updated,
        })
        .collect())
}

pub async fn toggle_action_item(
    pool: &SqlitePool,
    id: i64,
    done: bool,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE action_items SET done = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(if done { 1 } else { 0 })
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("toggle_action_item: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn api_action_item_list<R: Runtime>(
    app: tauri::AppHandle<R>,
    meeting_id: String,
) -> Result<Vec<ActionItem>, String> {
    let state: State<'_, AppState> = app.state();
    let pool = state.db_manager.pool();
    list_action_items(pool, &meeting_id).await
}

#[tauri::command]
pub async fn api_action_item_toggle<R: Runtime>(
    app: tauri::AppHandle<R>,
    id: i64,
    done: bool,
) -> Result<(), String> {
    let state: State<'_, AppState> = app.state();
    let pool = state.db_manager.pool();
    toggle_action_item(pool, id, done).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insert_ignores_duplicate_keeps_done_state() {
        // 用 in-memory sqlite 验证 upsert 语义 + done 字段保留
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE action_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                meeting_id TEXT NOT NULL,
                item_index INTEGER NOT NULL,
                content TEXT NOT NULL,
                done INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(meeting_id, item_index))",
        )
        .execute(&pool)
        .await
        .unwrap();

        let now = "2026-08-07T00:00:00Z".to_string();
        // 第 1 次 extract 插入 2 行
        for idx in 0..2 {
            sqlx::query("INSERT OR IGNORE INTO action_items (meeting_id, item_index, content, done, created_at, updated_at) VALUES (?1, ?2, ?3, 0, ?4, ?4)")
                .bind("m").bind(idx as i64).bind(format!("item {idx}"))
                .bind(&now).execute(&pool).await.unwrap();
        }
        // 用户 toggle 第 0 项为 done
        toggle_action_item(&pool, 1, true).await.unwrap();
        // 第 2 次 extract 重新 INSERT OR IGNORE — done 状态应保留
        for idx in 0..2 {
            sqlx::query("INSERT OR IGNORE INTO action_items (meeting_id, item_index, content, done, created_at, updated_at) VALUES (?1, ?2, ?3, 0, ?4, ?4)")
                .bind("m").bind(idx as i64).bind(format!("item {idx} REWRITE"))
                .bind(&now).execute(&pool).await.unwrap();
        }
        let items = list_action_items(&pool, "m").await.unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].done, true, "user toggled done must persist");  // ← expect keep
        assert_eq!(items[0].content, "item 0", "duplicate insert must NOT rewrite content");
        assert_eq!(items[1].done, false);
    }
}
