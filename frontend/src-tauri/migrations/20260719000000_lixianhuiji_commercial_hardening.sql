-- 离线会记 v0.7.0+: 商业化加固
-- 1) analytics_events 表: 本地落库, 不再 noop
-- 2) activation_codes.bound_machine_id 列: 防止一码多机

CREATE TABLE IF NOT EXISTS analytics_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER,
    event_name TEXT NOT NULL,
    properties_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_analytics_user ON analytics_events(user_id, created_at);
CREATE INDEX IF NOT EXISTS idx_analytics_event ON analytics_events(event_name, created_at);

-- 已有库兼容: ALTER TABLE 不带 IF NOT EXISTS (SQLite 不支持), 先查列存在再改
-- 这里用 CREATE INDEX IF NOT EXISTS + ALTER 兼容 (sqlite 没有 IF NOT EXISTS for ADD COLUMN)
-- 实际执行时若列已存在会报错, 可忽略
ALTER TABLE activation_codes ADD COLUMN bound_machine_id TEXT;
