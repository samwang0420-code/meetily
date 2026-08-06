-- Migration: §P1-B speaker alias (meeting-local, MVP)
-- 用户可以在会议详情页给 cam++ diar 输出的 speaker_X 起名 (如 "王伟")
-- alias 仅本会议有效 — 跨会议声音追踪 (voice embedding clustering) 不在本 MVP 范围

CREATE TABLE IF NOT EXISTS speaker_aliases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id TEXT NOT NULL,
    speaker_id INTEGER NOT NULL,
    label TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(meeting_id, speaker_id)
);

CREATE INDEX IF NOT EXISTS idx_speaker_alias_meeting ON speaker_aliases(meeting_id);
