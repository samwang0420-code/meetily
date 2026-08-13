-- §P0-B Obsidian vault 写入 (对齐 Charoite, 71 报告 P0-B)
-- 背景: 用户已在 ~/Documents/Obsidian Vault/, 每次会议结束自动生成 .md, 含 frontmatter + Summary/Minutes/Transcript + [[wikilink]]
-- 设计: per-user settings (enabled / vault_path / subdir / template_id), 与 transcript_settings 同样的 user_id 主键模式
-- 触发点: summary_processes.status='completed' 后, 在 service.rs spawn 一个 obsidian 写入 task, 不阻塞主流程

CREATE TABLE IF NOT EXISTS obsidian_export_settings (
    user_id INTEGER PRIMARY KEY,
    enabled INTEGER NOT NULL DEFAULT 0,
    vault_path TEXT NOT NULL DEFAULT '~/Documents/Obsidian Vault',
    subdir TEXT NOT NULL DEFAULT '会议',
    template_id TEXT NOT NULL DEFAULT 'default',
    last_exported_meeting_id TEXT,
    last_exported_at TEXT,
    last_export_status TEXT,
    last_export_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_obsidian_export_enabled
    ON obsidian_export_settings(enabled);
