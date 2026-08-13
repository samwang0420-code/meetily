-- Migration: §101 add transcripts.speaker_id for §91 P1-B JOIN
-- transcripts 表之前只有 speaker TEXT (audio source 'mic'/'system'),
-- 没有 speaker_id INTEGER 用来跟 speaker_aliases.speaker_id 关联.
-- §91 P1-B commit 在 SQL 里写了 t.speaker_id, 但 schema 漏了,
-- 导致 meeting-detail 页 api_get_meeting_transcripts 失败, 用户看到 "Failed to load transcripts".
-- 修复: 加 speaker_id INTEGER 列 + 索引.
-- 注意: 老数据 speaker_id 全 NULL, 新数据由 import.rs 写入.
-- backfill: §101 (next migration) 把 speaker_aliases 反推回 transcripts.speaker_id.

ALTER TABLE transcripts ADD COLUMN speaker_id INTEGER;

CREATE INDEX IF NOT EXISTS idx_transcripts_speaker_id ON transcripts(meeting_id, speaker_id);
