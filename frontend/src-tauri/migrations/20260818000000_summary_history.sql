-- Migration: §135 多次生成摘要历史保留
-- 用户痛点: 90+ min 庭审录完, 用 court_hearing 模板生成了 15 分钟, 觉得内容抽象;
-- 切到 standard_meeting 又生成 15 分钟; 切到 legal_consultation 又 15 分钟.
-- 但前 2 次的摘要都丢了 — summary_processes 只有最新 result.
--
-- 修复: 每次重新生成时, 把旧 result INSERT INTO summary_history, 永久保留.
-- UI 加"历史摘要"弹窗, 让用户能看 1/2/3 次的不同模板生成, 还能 diff 对比.

CREATE TABLE IF NOT EXISTS summary_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id TEXT NOT NULL,
    template_id TEXT,
    template_name TEXT,
    model_name TEXT,
    chunk_count INTEGER NOT NULL DEFAULT 0,
    processing_time REAL NOT NULL DEFAULT 0.0,
    result_json TEXT NOT NULL,            -- 完整的 result JSON (含 markdown + fact_guard + source)
    created_at TEXT NOT NULL,            -- 旧 result 的原始时间 (= summary_processes.updated_at 备份当时)
    archived_at TEXT NOT NULL,           -- 本次 INSERT 时间 (历史归档时间)
    backup_reason TEXT NOT NULL          -- 'regenerate' | 'template_switch' | 'manual_backup'
);

-- 按会议查该会议所有历史 (按时间倒序)
CREATE INDEX IF NOT EXISTS idx_summary_history_meeting_archived
    ON summary_history(meeting_id, archived_at DESC);

-- 按模板统计"多少会议用 X 模板生成过"
CREATE INDEX IF NOT EXISTS idx_summary_history_template
    ON summary_history(template_id);
