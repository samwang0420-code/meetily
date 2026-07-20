-- v0.6.11+ bug fix: enforce unique transcript segments per (meeting_id, audio time range)
-- 用 0.05s 容差 (SQLite 无原生范围索引, 用 ROUND(audio_start_time * 20) / 20 作为 bucket)
-- 这样 force_final 切句可能产生毫秒差也能合并

-- 1. 先删现有的重复行 (保留最早 rowid)
DELETE FROM transcripts
WHERE id IN (
  SELECT t.id FROM transcripts t
  JOIN (
    SELECT meeting_id, ROUND(audio_start_time * 20) / 20 as start_bucket,
           ROUND(audio_end_time * 20) / 20 as end_bucket,
           MIN(rowid) as keep_rowid
    FROM transcripts
    WHERE audio_start_time IS NOT NULL
    GROUP BY meeting_id, start_bucket, end_bucket
  ) keep ON t.meeting_id = keep.meeting_id
    AND ROUND(t.audio_start_time * 20) / 20 = keep.start_bucket
    AND ROUND(t.audio_end_time * 20) / 20 = keep.end_bucket
  WHERE t.rowid > keep.keep_rowid
);

-- 2. 加 unique index
CREATE UNIQUE INDEX IF NOT EXISTS idx_transcripts_meeting_time
  ON transcripts(meeting_id, ROUND(audio_start_time * 20), ROUND(audio_end_time * 20));
