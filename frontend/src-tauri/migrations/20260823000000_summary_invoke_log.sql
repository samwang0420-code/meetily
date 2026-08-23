-- Migration: §169.4 invoke 实际参数诊断表 (2026-08-23)
-- 触发: macOS .app bundle 启动后 stderr 被 LaunchServices 丢弃, 没法看 backend log
-- 解决: 在 api_process_transcript 入口处把实际收到的 invoke 参数写进这张表
-- 用户点 regenerate 后 SELECT * 即可看到 force_fresh / force_fresh_camel / force_fresh_alias 实际接收值
-- 也可用来诊断 Tauri invoke 序列化是否真把 snake_case 转 camelCase

CREATE TABLE IF NOT EXISTS summary_invoke_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id TEXT NOT NULL,
    invoked_at TEXT NOT NULL,
    force_fresh_recv BOOLEAN,
    force_fresh_camel_recv BOOLEAN,
    force_fresh_alias_recv BOOLEAN,
    regeneration_flag_recv BOOLEAN,
    summary_language TEXT,
    model_provider TEXT,
    template_id TEXT,
    effective_force_fresh BOOLEAN NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_summary_invoke_log_meeting_invoked
    ON summary_invoke_log(meeting_id, invoked_at DESC);
