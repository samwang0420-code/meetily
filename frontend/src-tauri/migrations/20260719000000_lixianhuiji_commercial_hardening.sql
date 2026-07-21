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

-- v0.7.0+ rc3 修复:
-- 之前用 ALTER TABLE ADD COLUMN IF NOT EXISTS 启动 panic
-- (libsqlite3-sys bundled 不支持这个语法, 即使 SQLite 3.35+).
-- 老库已存在 bound_machine_id 列 (C4 部署过), 用裸 ALTER 会 duplicate column.
-- 新库第一次跑这条 migration 时, 表里没这个列, 也需要加.
-- 解决: 在 manager.rs 里手工 idempotent ALTER (PRAGMA table_info 检查),
-- 然后手动把这次成功标记进 _sqlx_migrations, 避免重复跑.
