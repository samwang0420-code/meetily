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

// §91 Bug 5 fix: 不再用 SummaryBlocks JSON 路径. 从 markdown 直接解析"行动事项"表格.
// 占位行 ("无行动事项" / "无明确事项" / "本节无相关事项") 自动跳过.
const ACTION_ITEMS_PLACEHOLDERS: &[&str] = &[
    "无行动事项",
    "无明确事项",
    "本节无相关事项",
    "本次无行动事项",
    "Owner: Not specified",
    "Deadline: Not specified",
];

fn parse_markdown_action_items(md: &str) -> Vec<String> {
    let mut start = None;
    for marker in &["**行动事项**", "## 行动事项"] {
        if let Some(pos) = md.find(marker) {
            start = Some(pos + marker.len());
            break;
        }
    }
    let Some(start) = start else { return Vec::new() };
    let tail = &md[start..];
    let end = tail
        .find("
**")
        .or_else(|| tail.find("
## "))
        .unwrap_or(tail.len());
    let section = &tail[..end];

    let mut items = Vec::new();
    let mut in_table = false;
    for line in section.lines() {
        let line = line.trim();
        if line.starts_with('|') {
            if line.contains("---") && line.chars().all(|c| c == '|' || c == '-' || c == ' ' || c == ':') {
                in_table = true;
                continue;
            }
            if !in_table { continue; }
            let cols: Vec<String> = line
                .trim_matches('|')
                .split('|')
                .map(|c| c.trim().to_string())
                .collect();
            if cols.is_empty() { continue; }
            let first = cols[0].clone();
            if first.contains("事项") || first.contains("Action") || first.contains("Item") {
                continue;
            }
            let placeholder = ACTION_ITEMS_PLACEHOLDERS.iter().any(|p| first.contains(p));
            if placeholder { continue; }
            let content = if cols.len() >= 3 {
                let owner = &cols[1];
                let deadline = &cols[2];
                if owner == "未明确" || owner.contains("Not specified") {
                    first.clone()
                } else if deadline == "未明确" || deadline.contains("Not specified") {
                    format!("{} — {}", first, owner)
                } else {
                    format!("{} — {} — {}", first, owner, deadline)
                }
            } else {
                first.clone()
            };
            items.push(content);
        } else if !line.is_empty() {
            if ACTION_ITEMS_PLACEHOLDERS.iter().any(|p| line.contains(p)) {
                return items;
            }
        }
    }
    items
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

    // §91 Bug 5: 旧版只解析 JSON summary.action_items.blocks 路径. 但 build_summary_result_json
    // 实际上只写 markdown 字段 (含 "**行动事项**\n\n| 事项 | ... |" 表格). 旧路径永远空数组.
    // 修复: 直接从 markdown 提取"行动事项"段落的表格行, 跳过"无行动事项"/"无明确事项"等占位.
    let action_items_blocks = parse_markdown_action_items(&json_str);
    if action_items_blocks.is_empty() {
        log::info!("[action_items] {meeting_id} no action_items in markdown (无行动事项/无明确事项/本节无相关事项)");
        return;
    }
    let now = Utc::now().to_rfc3339();
    let mut inserted = 0usize;
    for (idx, content) in action_items_blocks.iter().enumerate() {
        let content_str = content.trim().to_string();
        if content_str.is_empty() {
            continue;
        }
        let content = &content_str;
        // INSERT OR IGNORE — 已经存在的 (meeting_id, item_index) 行不动,
        // 保留用户已 toggle 的 done 状态, 不覆盖
        let res = sqlx::query(
            "INSERT OR IGNORE INTO action_items (meeting_id, item_index, content, done, created_at, updated_at) \
             VALUES (?1, ?2, ?3, 0, ?4, ?4)",
        )
        .bind(&meeting_id)
        .bind(idx as i64)
        .bind(content.clone())
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

    #[test]
    fn test_parse_e5b78a31_real_meeting() {
        let md = r#"**会议摘要**
本次会议...
**行动事项**

| **事项** | **负责人** | **截止时间** |
| :--- | :--- | :--- |
| 资金汇兑与现金准备 | 杨未央、潘潘 | 立即执行 |
| 物资采购下单（水、食物、设备） | 杨未央 | 6 月 10 日之后 24 小时内完成 |
| 房屋改造施工（密封隔热层） | 潘潘 | 6 月 11 日深夜前完工 |
| 物资分拣与分区收纳 | 杨未央 | 6 月 11 日深夜前完成 |

**遗留与风险**
问题"#;
        let items = parse_markdown_action_items(md);
        assert_eq!(items.len(), 4, "expected 4 items, got {}: {:?}", items.len(), items);
        assert!(items[0].contains("资金汇兑"));
        assert!(items[0].contains("杨未央"));
        assert!(items[0].contains("立即执行"));
        assert!(items[1].contains("6 月 10 日"));
    }

    #[test]
    fn test_parse_no_action_items_placeholder() {
        let md = "**会议摘要**\n...\n**行动事项**\n本次无行动事项。\n\n**遗留**\n无遗留问题。";
        let items = parse_markdown_action_items(md);
        assert_eq!(items.len(), 0, "placeholder must be skipped");
    }

    #[test]
    fn test_parse_no_action_items_section() {
        let md = "**会议摘要**\n...\n**遗留**\n";
        let items = parse_markdown_action_items(md);
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_parse_unknown_owner_keeps_item() {
        let md = "**行动事项**\n\n| 事项 | 负责人 | 截止 |\n| --- | --- | --- |\n| 和珅生平梳理 | 未明确 | 未明确 |\n";
        let items = parse_markdown_action_items(md);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0], "和珅生平梳理");
    }

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
