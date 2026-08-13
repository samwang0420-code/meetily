// §P1-B speaker alias (MVP) — meeting-local rename of cam++ diar output IDs
//
// cam++ 输出 speaker_id (整数, 0..N). 用户在 UI 可把 speaker_0 显示为 "王伟" 等.
// alias 仅本 meeting 有效, 跨会议由 voice embedding clustering 解决 (不在本 MVP).
//
// schema (see migration 20260807000001):
//   speaker_aliases(id, meeting_id, speaker_id, label, created_at, updated_at)
//   UNIQUE(meeting_id, speaker_id)
//
// Tauri command:
//   api_speaker_alias_list(meeting_id) -> Vec<SpeakerAlias>
//   api_speaker_alias_set(meeting_id, speaker_id, label)
//   api_speaker_alias_delete(meeting_id, speaker_id)

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::{Manager, Runtime, State};

use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerAlias {
    pub id: i64,
    pub meeting_id: String,
    pub speaker_id: i64,
    pub label: String,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn list_aliases(
    pool: &SqlitePool,
    meeting_id: &str,
) -> Result<Vec<SpeakerAlias>, String> {
    let rows: Vec<(i64, String, i64, String, String, String)> = sqlx::query_as(
        "SELECT id, meeting_id, speaker_id, label, created_at, updated_at \
         FROM speaker_aliases WHERE meeting_id = ?1 ORDER BY speaker_id",
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("list_aliases: {e}"))?;
    Ok(rows
        .into_iter()
        .map(|(id, mid, sid, label, created, updated)| SpeakerAlias {
            id,
            meeting_id: mid,
            speaker_id: sid,
            label,
            created_at: created,
            updated_at: updated,
        })
        .collect())
}

pub async fn set_alias(
    pool: &SqlitePool,
    meeting_id: &str,
    speaker_id: i64,
    label: &str,
) -> Result<(), String> {
    let label = label.trim();
    if label.is_empty() {
        // 空 label 视为删除
        sqlx::query("DELETE FROM speaker_aliases WHERE meeting_id = ?1 AND speaker_id = ?2")
            .bind(meeting_id)
            .bind(speaker_id)
            .execute(pool)
            .await
            .map_err(|e| format!("set_alias delete: {e}"))?;
        return Ok(());
    }
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO speaker_aliases (meeting_id, speaker_id, label, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?4) \
         ON CONFLICT(meeting_id, speaker_id) DO UPDATE SET \
           label = excluded.label, updated_at = excluded.updated_at",
    )
    .bind(meeting_id)
    .bind(speaker_id)
    .bind(label)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| format!("set_alias: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn api_speaker_alias_list<R: Runtime>(
    app: tauri::AppHandle<R>,
    meeting_id: String,
) -> Result<Vec<SpeakerAlias>, String> {
    let state: State<'_, AppState> = app.state();
    list_aliases(state.db_manager.pool(), &meeting_id).await
}

#[tauri::command]
pub async fn api_speaker_alias_set<R: Runtime>(
    app: tauri::AppHandle<R>,
    meeting_id: String,
    speaker_id: i64,
    label: String,
) -> Result<(), String> {
    let state: State<'_, AppState> = app.state();
    set_alias(state.db_manager.pool(), &meeting_id, speaker_id, &label).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_alias_set_then_list_keeps_label() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE speaker_aliases (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                meeting_id TEXT NOT NULL,
                speaker_id INTEGER NOT NULL,
                label TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(meeting_id, speaker_id))",
        )
        .execute(&pool)
        .await
        .unwrap();

        set_alias(&pool, "m1", 0, "王伟").await.unwrap();
        set_alias(&pool, "m1", 1, "张伟").await.unwrap();
        let list = list_aliases(&pool, "m1").await.unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].label, "王伟");
        assert_eq!(list[1].label, "张伟");

        // 改 label
        set_alias(&pool, "m1", 0, "王伟 CEO").await.unwrap();
        let list = list_aliases(&pool, "m1").await.unwrap();
        assert_eq!(list[0].label, "王伟 CEO");
        assert_eq!(list.len(), 2); // update not insert

        // 空 label 删除
        set_alias(&pool, "m1", 1, "").await.unwrap();
        let list = list_aliases(&pool, "m1").await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].speaker_id, 0);
    }
}
