-- Migration: §81 P2-A 行动项可点击完成
-- 把 summary_processes.result JSON 里的 action_items.blocks 拆出来建独立行,
-- 让用户可以逐条 toggle done 状态,持久化跨 session。

CREATE TABLE IF NOT EXISTS action_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id TEXT NOT NULL,
    item_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    done INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(meeting_id, item_index)
);

CREATE INDEX IF NOT EXISTS idx_action_items_meeting ON action_items(meeting_id);
