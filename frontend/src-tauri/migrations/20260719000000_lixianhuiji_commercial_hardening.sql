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

-- v0.7.0+ 修复: SQLite 不支持 ALTER TABLE ADD COLUMN IF NOT EXISTS,
-- 已有库 (C4 部署过) 已加过 bound_machine_id, 重跑会 duplicate column 报错.
-- 改成 PRAGMA table_info 查询列存在性, 仅当列不存在时才执行 ALTER.
-- 这种"条件 DDL" 无法用单一 SQL 表达, 改成 SQLite 3.35+ 原生 IF NOT EXISTS 兼容:
--   SQLite 3.35.0 (2021-03-12) 起支持 ALTER TABLE ADD COLUMN IF NOT EXISTS
-- macOS 系统 SQLite 通常 >= 3.39 (Big Sur+), 满足条件.
ALTER TABLE activation_codes ADD COLUMN IF NOT EXISTS bound_machine_id TEXT;
