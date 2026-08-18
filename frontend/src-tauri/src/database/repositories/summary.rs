use crate::database::models::SummaryProcess;
use chrono::Utc;
use serde_json::Value;
use sqlx::SqlitePool;
use tracing::{error, info as log_info};

pub struct SummaryProcessesRepository;

impl SummaryProcessesRepository {
    /// Retrieves the current summary process state for a given meeting ID.
    pub async fn get_summary_data(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Option<SummaryProcess>, sqlx::Error> {
        sqlx::query_as::<_, SummaryProcess>("SELECT * FROM summary_processes WHERE meeting_id = ?")
            .bind(meeting_id)
            .fetch_optional(pool)
            .await
    }

    pub async fn update_meeting_summary(
        pool: &SqlitePool,
        meeting_id: &str,
        summary: &Value,
    ) -> Result<bool, sqlx::Error> {
        let mut transaction = pool.begin().await?;

        let meeting_exists: bool = sqlx::query("SELECT 1 FROM meetings WHERE id = ?")
            .bind(meeting_id)
            .fetch_optional(&mut *transaction)
            .await?
            .is_some();

        if !meeting_exists {
            log_info!(
                "Attempted to save summary for a non-existent meeting_id: {}",
                meeting_id
            );
            transaction.rollback().await?;
            return Ok(false);
        }

        let result_json = serde_json::to_string(summary);
        if result_json.is_err() {
            error!("Can't convert the json to string for saving to Database");
            transaction.rollback().await?;
            return Ok(false);
        }
        let now = Utc::now();

        sqlx::query("UPDATE summary_processes SET result = ?, updated_at = ? WHERE meeting_id = ?")
            .bind(&result_json.unwrap())
            .bind(now)
            .bind(meeting_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query("UPDATE meetings SET updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(meeting_id)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;

        log_info!(
            "Successfully updated summary and timestamp for meeting_id: {}",
            meeting_id
        );
        Ok(true)
    }

    pub async fn get_summary_data_for_meeting(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Option<SummaryProcess>, sqlx::Error> {
        sqlx::query_as::<_, SummaryProcess>(
            "SELECT p.* FROM summary_processes p JOIN transcript_chunks t ON p.meeting_id = t.meeting_id WHERE p.meeting_id = ?",
        )
        .bind(meeting_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn create_or_reset_process(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<(), sqlx::Error> {
        log_info!(
            "Creating or resetting summary process for meeting_id: {}",
            meeting_id
        );
        let now = Utc::now();
        // §135: 先把旧 result 写入 summary_history (永久保留), 再做 backup (用于失败回滚).
        //       一次生成成功: history + 1 条 (旧版本); 失败: 回滚到 backup, history 也保留.
        sqlx::query(
            r#"
            INSERT INTO summary_processes (meeting_id, status, created_at, updated_at, start_time, result, error)
            VALUES (?, 'PENDING', ?, ?, ?, NULL, NULL)
            ON CONFLICT(meeting_id) DO UPDATE SET
                status = 'PENDING',
                updated_at = excluded.updated_at,
                start_time = excluded.start_time,
                result_backup = result,
                result_backup_timestamp = excluded.updated_at,
                result = result,
                error = NULL
            "#
        )
        .bind(meeting_id)
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        // §135: 备份到 summary_history (只在旧 result 存在且非空时)
        let old: Option<(Option<String>, Option<String>, Option<String>, Option<String>, Option<i64>, Option<f64>)> =
            sqlx::query_as(
                r#"
                SELECT sp.result, sp.result_backup,
                       json_extract(sp.result, '$.source.template_id'),
                       json_extract(sp.result, '$.source.model_name'),
                       sp.chunk_count, sp.processing_time
                FROM summary_processes sp
                WHERE sp.meeting_id = ?1
                "#
            )
            .bind(meeting_id)
            .fetch_optional(pool)
            .await?;
        if let Some((Some(result_str), _, tid, mname, cc, pt)) = old {
            // 用 summary_processes.result_backup_timestamp 作为 created_at (旧版本原时间)
            let old_ts: Option<String> = sqlx::query_scalar(
                "SELECT result_backup_timestamp FROM summary_processes WHERE meeting_id = ?1"
            )
            .bind(meeting_id)
            .fetch_optional(pool)
            .await?
            .flatten();
            sqlx::query(
                r#"
                INSERT INTO summary_history
                    (meeting_id, template_id, template_name, model_name, chunk_count, processing_time,
                     result_json, created_at, archived_at, backup_reason)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'regenerate')
                "#
            )
            .bind(meeting_id)
            .bind(&tid)
            .bind(&tid)  // template_name 暂用 id (前端可读 built-in name 表)
            .bind(&mname)
            .bind(cc.unwrap_or(0))
            .bind(pt.unwrap_or(0.0))
            .bind(result_str)
            .bind(old_ts.unwrap_or_else(|| now.to_rfc3339()))
            .bind(now.to_rfc3339())
            .execute(pool)
            .await?;
            log_info!("[§135] archived previous summary to history for meeting_id: {}", meeting_id);
        }

        log_info!(
            "Backed up existing summary before regeneration for meeting_id: {}",
            meeting_id
        );
        Ok(())
    }



    /// §129 (2026-08-17): Cleanup summary_processes rows stuck in PENDING
    ///   after a previous app crash / force-quit / process kill.
    ///   Real root cause: api_process_transcript sets status='PENDING' at start,
    ///   update_process_completed/failed sets terminal status on success/failure.
    ///   If the process gets killed mid-flight (app force-quit, OOM, llama-helper crash),
    ///   PENDING rows stay forever and look stuck.
    ///   This function marks PENDING rows older than `stale_minutes` as 'failed'
    ///   with "Interrupted by app shutdown" message. Caller can choose threshold.
    pub async fn cleanup_stale_pending_processes(
        pool: &SqlitePool,
        stale_minutes: i64,
    ) -> Result<u64, sqlx::Error> {
        let now = Utc::now();
        let cutoff = now - chrono::Duration::minutes(stale_minutes);

        let result = sqlx::query(
            r#"
            UPDATE summary_processes
            SET status = 'failed',
                error = 'Interrupted by app shutdown — backend did not complete within '
                        || ? || ' minutes. Click Regenerate to retry.',
                updated_at = ?,
                end_time = ?,
                result = COALESCE(result_backup, result),
                result_backup = NULL,
                result_backup_timestamp = NULL
            WHERE status = 'PENDING'
              AND updated_at < ?
            "#,
        )
        .bind(stale_minutes)
        .bind(now)
        .bind(now)
        .bind(cutoff)
        .execute(pool)
        .await?;

        let count = result.rows_affected();
        if count > 0 {
            log_info!(
                "§129 cleanup_stale_pending_processes: marked {} stale PENDING rows as failed (cutoff = {} minutes ago)",
                count,
                stale_minutes
            );
        }
        Ok(count)
    }

    pub async fn update_process_completed(
        pool: &SqlitePool,
        meeting_id: &str,
        result: Value, // Keep this as Value to handle both old and new formats if needed
        chunk_count: i64,
        processing_time: f64,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        let result_str = serde_json::to_string(&result)
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to serialize result: {}", e)))?;

        sqlx::query(
            r#"
            UPDATE summary_processes
            SET status = 'completed', result = ?, updated_at = ?, end_time = ?, chunk_count = ?, processing_time = ?, error = NULL, result_backup = NULL, result_backup_timestamp = NULL
            WHERE meeting_id = ?
            "#
        )
        .bind(result_str)
        .bind(now)
        .bind(now)
        .bind(chunk_count)
        .bind(processing_time)
        .bind(meeting_id)
        .execute(pool)
        .await?;
        log_info!(
            "Summary completed and backup cleared for meeting_id: {}",
            meeting_id
        );
        Ok(())
    }

    pub async fn update_process_failed(
        pool: &SqlitePool,
        meeting_id: &str,
        error: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();

        // Restore from backup if it exists, otherwise keep current result
        sqlx::query(
            r#"
            UPDATE summary_processes
            SET
                status = 'failed',
                error = ?,
                updated_at = ?,
                end_time = ?,
                result = COALESCE(result_backup, result),
                result_backup = NULL,
                result_backup_timestamp = NULL
            WHERE meeting_id = ?
            "#,
        )
        .bind(error)
        .bind(now)
        .bind(now)
        .bind(meeting_id)
        .execute(pool)
        .await?;
        log_info!(
            "Summary generation failed and backup restored for meeting_id: {}",
            meeting_id
        );
        Ok(())
    }

    pub async fn update_process_cancelled(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();

        // Restore from backup if it exists, otherwise keep current result
        sqlx::query(
            r#"
            UPDATE summary_processes
            SET
                status = 'cancelled',
                updated_at = ?,
                end_time = ?,
                error = 'Generation was cancelled by user',
                result = COALESCE(result_backup, result),
                result_backup = NULL,
                result_backup_timestamp = NULL
            WHERE meeting_id = ?
            "#,
        )
        .bind(now)
        .bind(now)
        .bind(meeting_id)
        .execute(pool)
        .await?;
        log_info!(
            "Marked summary process as cancelled and restored backup for meeting_id: {}",
            meeting_id
        );
        Ok(())
    }
}
