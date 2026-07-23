use crate::api::{TranscriptSearchResult, TranscriptSegment};
use chrono::Utc;
use sqlx::{Connection, Error as SqlxError, SqlitePool};
use tracing::{error, info};
use uuid::Uuid;

// P1-I: placeholder 标题兜底 — 录音开始时 INSERT placeholder "Recording in progress (...)" 占位,
// save_transcript 阶段如果新 title 不再是 placeholder 模式 (即前端已经从 stop 路径传入真实标题),
// 主动 UPDATE 覆盖, 否则用户列表里看到一堆 "Recording in progress (Untitled)" 鬼会议卡.
// 判定: 新 title 不以 "Recording in progress" 开头, 且与当前 title 不同, 则覆盖.
fn is_placeholder_title(title: &str) -> bool {
    let t = title.trim();
    t.starts_with("Recording in progress") || t == "Untitled" || t.is_empty()
}

pub struct TranscriptsRepository;

impl TranscriptsRepository {
    /// Saves a new meeting and its associated transcript segments.
    /// This function uses a transaction to ensure that either both the meeting
    /// and all its transcripts are saved, or none of them are.
    pub async fn save_transcript(
        pool: &SqlitePool,
        meeting_title: &str,
        transcripts: &[TranscriptSegment],
        folder_path: Option<String>,
        // v0.7.1+: 长会议 diar pickup — 前端录音停止时复用 start_recording 生成的 id,
        // 让 sherpa_asr 后台线程 UPDATE 的 transcripts.speaker 行能命中.
        // None 时回落到自动生成 (兼容旧调用).
        meeting_id_in: Option<&str>,
        user_id: Option<i64>,
    ) -> Result<String, SqlxError> {
        let meeting_id = meeting_id_in
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("meeting-{}", Uuid::new_v4()));

        let mut conn = pool.acquire().await?;
        let mut transaction = conn.begin().await?;

        let now = Utc::now();

        // 1. Create the new meeting (INSERT OR IGNORE 兼容 start_recording 时已 placeholder)
        let result = sqlx::query(
            "INSERT OR IGNORE INTO meetings (id, title, created_at, updated_at, folder_path, user_id) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&meeting_id)
        .bind(meeting_title)
        .bind(now)
        .bind(now)
        .bind(&folder_path)
        .bind(user_id)
        .execute(&mut *transaction)
        .await;

        if let Err(e) = result {
            error!("Failed to create meeting '{}': {}", meeting_title, e);
            transaction.rollback().await?;
            return Err(e);
        }

        info!("Successfully created meeting with id: {}", meeting_id);

        // 1b. v0.7.x P0 quick-fix: start_recording 时 placeholder meetings 行 folder_path = NULL,
        // 上面的 INSERT OR IGNORE 会直接 skip (placeholder 已存在), folder_path 永不更新.
        // 兜底 UPDATE: 只在 folder_path 是 NULL 时覆盖, 保护用户手动改名.
        // folder_path 为空字符串不算 NULL, 用 TRIM(folder_path) = '' 一起覆盖.
        if folder_path.is_some() {
            sqlx::query(
                "UPDATE meetings SET folder_path = ?, updated_at = ?, user_id = COALESCE(user_id, ?) WHERE id = ? AND (folder_path IS NULL OR TRIM(folder_path) = '')"
            )
            .bind(folder_path.as_deref())
            .bind(now)
            .bind(user_id)
            .bind(&meeting_id)
            .execute(&mut *transaction)
            .await
            .map_err(|e| {
                error!("Failed to backfill meeting folder_path for {}: {}", meeting_id, e);
                e
            })?;
        }

        // 1c. P1-I: title 兜底 UPDATE — placeholder "Recording in progress (Untitled)"
        // 在 start_recording 时已经 INSERT, 上面的 INSERT OR IGNORE 会 skip, 但 meeting_title
        // 会保留 placeholder. save_transcript 阶段如果传入了真实 title, 主动覆盖.
        // 条件: 新 title 非空 && 不是 placeholder 模式 && 与当前 title 不同.
        if !is_placeholder_title(meeting_title) {
            sqlx::query(
                "UPDATE meetings SET title = ?, updated_at = ? WHERE id = ? AND (title LIKE 'Recording in progress%' OR title = 'Untitled' OR TRIM(title) = '')"
            )
            .bind(meeting_title)
            .bind(now)
            .bind(&meeting_id)
            .execute(&mut *transaction)
            .await
            .map_err(|e| {
                error!("Failed to backfill meeting title for {}: {}", meeting_id, e);
                e
            })?;
        }

        // 2. Save each transcript segment with audio timing fields
        for segment in transcripts {
            let transcript_id = format!("transcript-{}", Uuid::new_v4());
            let result = sqlx::query(
                "INSERT OR IGNORE INTO transcripts (id, meeting_id, transcript, timestamp, audio_start_time, audio_end_time, duration, user_id)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&transcript_id)
            .bind(&meeting_id)
            .bind(&segment.text)
            .bind(&segment.timestamp)
            .bind(segment.audio_start_time)
            .bind(segment.audio_end_time)
            .bind(segment.duration)
            .bind(user_id)
            .execute(&mut *transaction)
            .await;

            if let Err(e) = result {
                error!(
                    "Failed to save transcript segment for meeting {}: {}",
                    meeting_id, e
                );
                transaction.rollback().await?;
                return Err(e);
            }
        }

        info!(
            "Successfully saved {} transcript segments for meeting {}",
            transcripts.len(),
            meeting_id
        );

        // Commit the transaction
        transaction.commit().await?;

        Ok(meeting_id)
    }

    /// Searches for a query string within the transcripts.
    /// It returns a list of matching transcripts with context.
    /// v0.7.0+: 按 user_id 隔离 (跨用户不能搜到别人转录)
    pub async fn search_transcripts(
        pool: &SqlitePool,
        query: &str,
        user_id: Option<i64>,
    ) -> Result<Vec<TranscriptSearchResult>, SqlxError> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let search_query = format!("%{}%", query.to_lowercase());

        let rows = match user_id {
            Some(uid) => {
                sqlx::query_as::<_, (String, String, String, String)>(
                    "SELECT m.id, m.title, t.transcript, t.timestamp
                 FROM meetings m
                 JOIN transcripts t ON m.id = t.meeting_id
                 WHERE LOWER(t.transcript) LIKE ? AND m.user_id = ?",
                )
                .bind(&search_query)
                .bind(uid)
                .fetch_all(pool)
                .await?
            }
            None => {
                sqlx::query_as::<_, (String, String, String, String)>(
                    "SELECT m.id, m.title, t.transcript, t.timestamp
                 FROM meetings m
                 JOIN transcripts t ON m.id = t.meeting_id
                 WHERE LOWER(t.transcript) LIKE ?",
                )
                .bind(&search_query)
                .fetch_all(pool)
                .await?
            }
        };

        let results = rows
            .into_iter()
            .map(|(id, title, transcript, timestamp)| {
                let match_context = Self::get_match_context(&transcript, query);
                TranscriptSearchResult {
                    id,
                    title,
                    match_context,
                    timestamp,
                }
            })
            .collect();

        Ok(results)
    }

    /// Helper function to extract a snippet of text around the first match of a query.
    fn get_match_context(transcript: &str, query: &str) -> String {
        let transcript_lower = transcript.to_lowercase();
        let query_lower = query.to_lowercase();

        match transcript_lower.find(&query_lower) {
            Some(match_index) => {
                let start_index = match_index.saturating_sub(100);
                let end_index = (match_index + query.len() + 100).min(transcript.len());

                let mut context = String::new();
                if start_index > 0 {
                    context.push_str("...");
                }
                context.push_str(&transcript[start_index..end_index]);
                if end_index < transcript.len() {
                    context.push_str("...");
                }
                context
            }
            None => transcript.chars().take(200).collect(), // Fallback to the start of the transcript
        }
    }
}

#[cfg(test)]
mod placeholder_title_tests {
    use super::is_placeholder_title;

    #[test]
    fn detects_recording_in_progress() {
        assert!(is_placeholder_title("Recording in progress (Untitled)"));
        assert!(is_placeholder_title("Recording in progress (MyMeeting)"));
        assert!(is_placeholder_title("Recording in progress"));
    }

    #[test]
    fn detects_unitled_and_empty() {
        assert!(is_placeholder_title("Untitled"));
        assert!(is_placeholder_title(""));
        assert!(is_placeholder_title("   "));
    }

    #[test]
    fn accepts_real_title() {
        assert!(!is_placeholder_title("和珅传"));
        assert!(!is_placeholder_title("Meeting 2026-07-21_14-30-01"));
        assert!(!is_placeholder_title("胡明浩律师简历分享"));
        assert!(!is_placeholder_title("Recording with intent"));  // 含 Recording 但不前置
    }
}
