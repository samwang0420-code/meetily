use crate::api::{TranscriptSearchResult, TranscriptSegment};
use crate::audio::asr_sanitize::{normalize_aliases, sanitize_asr_text, AsrQuality};
use chrono::Utc;
use sqlx::{Connection, Error as SqlxError, SqlitePool};
use tracing::{error, info};
use uuid::Uuid;

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
        // §105: 录音 stop 路径加 user_id, 跟 import 路径一致
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

        // §138 P0.2 ASR sanitize 统计
        let mut sanitized_count = 0usize;
        let mut low_quality_count = 0usize;

        info!("Successfully created meeting with id: {}", meeting_id);

        // 2. Save each transcript segment with audio timing fields
        // §138 P0.2: ASR 错字过滤 — 折叠连续重复字符 + 截断长无标点段
        // 严重错字段 (Low quality) 不写入 transcripts, 避免污染摘要 prompt
        for segment in transcripts {
            let transcript_id = format!("transcript-{}", Uuid::new_v4());
            // §138 P2.1: 别名规范化 (转录写入前替换, 让 LLM 看到一致输入)
            let (aliased_text, alias_count) = normalize_aliases(&segment.text);
            // §138 P0.2: ASR 错字过滤
            let (sanitized_text, was_modified, quality) = sanitize_asr_text(&aliased_text);
            if was_modified {
                sanitized_count += 1;
            }
            if alias_count > 0 {
                tracing::debug!(
                    "§138 P2.1 normalized {} aliases in segment {}",
                    alias_count,
                    transcript_id
                );
            }
            if quality == AsrQuality::Low {
                low_quality_count += 1;
                tracing::warn!(
                    "§138 P0.2 dropped low-quality ASR segment: id={} chars={} preview={:?}",
                    transcript_id,
                    segment.text.chars().count(),
                    &segment.text.chars().take(50).collect::<String>()
                );
                continue; // 不写入 DB
            }
            let result = sqlx::query(
                "INSERT OR IGNORE INTO transcripts (id, meeting_id, transcript, timestamp, audio_start_time, audio_end_time, duration)
                 VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&transcript_id)
            .bind(&meeting_id)
            .bind(&sanitized_text)
            .bind(&segment.timestamp)
            .bind(segment.audio_start_time)
            .bind(segment.audio_end_time)
            .bind(segment.duration)
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

        // §138 P0.2 ASR sanitize 统计 log
        info!(
            "§138 P0.2 ASR sanitize summary: meeting={} total={} sanitized={} dropped_low_quality={}",
            meeting_id,
            transcripts.len(),
            sanitized_count,
            low_quality_count
        );

        Ok(meeting_id)
    }

    /// Searches for a query string within the transcripts.
    /// It returns a list of matching transcripts with context.
    pub async fn search_transcripts(
        pool: &SqlitePool,
        query: &str,
    ) -> Result<Vec<TranscriptSearchResult>, SqlxError> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let search_query = format!("%{}%", query.to_lowercase());

        let rows = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT m.id, m.title, t.transcript, t.timestamp
             FROM meetings m
             JOIN transcripts t ON m.id = t.meeting_id
             WHERE LOWER(t.transcript) LIKE ?",
        )
        .bind(&search_query)
        .fetch_all(pool)
        .await?;

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
    ///
    /// §P1-A2 (audit 2026-08-23): the previous byte-index slicing panics with
    /// "byte index N is not a char boundary" for any Chinese query that lands on
    /// a multi-byte UTF-8 codepoint. Compute all indices in char-space so we
    /// always land on valid char boundaries before slicing.
    fn get_match_context(transcript: &str, query: &str) -> String {
        let transcript_lower = transcript.to_lowercase();
        let query_lower = query.to_lowercase();

        // Operate in char indices: keeps us on UTF-8 boundaries for any language.
        let chars: Vec<(usize, char)> = transcript_lower.char_indices().collect();
        let query_chars: Vec<char> = query_lower.chars().collect();

        if query_chars.is_empty() {
            return transcript.chars().take(200).collect();
        }

        // Find first occurrence by char equality (case-fold done via to_lowercase).
        let match_start_char = chars
            .windows(query_chars.len())
            .position(|w| w.iter().map(|(_, c)| *c).eq(query_chars.iter().copied()));

        match match_start_char {
            Some(idx) => {
                let pad_chars = 100usize;
                let start_char_idx = idx.saturating_sub(pad_chars);
                let end_char_idx = (idx + query_chars.len() + pad_chars).min(chars.len());
                let start_byte = chars[start_char_idx].0;
                let end_byte = if end_char_idx < chars.len() {
                    chars[end_char_idx].0
                } else {
                    transcript.len()
                };

                let mut context = String::new();
                if start_byte > 0 {
                    context.push_str("...");
                }
                // Bound the slice so a query near the end cannot exceed transcript.len().
                let safe_end = end_byte.min(transcript.len()).max(start_byte);
                context.push_str(&transcript[start_byte..safe_end]);
                if end_byte < transcript.len() {
                    context.push_str("...");
                }
                context
            }
            None => transcript.chars().take(200).collect(), // Fallback to the start of the transcript
        }
    }

    /// §P1-A2 regression: keep UTF-8 boundary safe behavior pinned.
    fn _p1_a2_tests() {}
}

/// §P1-A2 tests: ensure char-boundary-safe slicing for CJK queries.
#[cfg(test)]
mod tests_p1_a2 {
    use super::TranscriptsRepository;

    #[test]
    fn get_match_context_chinese_query_does_not_panic() {
        let transcript = "今天晚上打开发型 总结一下今天的内容";
        let query = "总结";
        let ctx = TranscriptsRepository::get_match_context(transcript, query);
        assert!(ctx.contains(query), "context must include the query: {}", ctx);
    }

    #[test]
    fn get_match_context_ascii_query_still_works() {
        let transcript = "the quick brown fox jumps over the lazy dog";
        let query = "fox";
        let ctx = TranscriptsRepository::get_match_context(transcript, query);
        assert!(ctx.contains(query));
    }

    #[test]
    fn get_match_context_no_match_falls_back_to_prefix() {
        let transcript = "你好世界 hello world";
        let ctx = TranscriptsRepository::get_match_context(transcript, "xyz");
        assert!(!ctx.is_empty());
    }
}
