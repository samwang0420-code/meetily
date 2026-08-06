-- §P0-A 跨会议知识图谱 (对齐 Charoite 71 报告 P0-A 战略级)
-- 核心: 4 张表 (topic_node / meeting_episode_node / relates_to / topic_dossier)
-- 用户场景: "上次讨论过 API 限流吗?状态是什么?" → 3 秒内返完整图谱
-- 设计原则: dedupe by canonical_name, 一场会议对一 topic 只记一次, dossier 可重建

CREATE TABLE IF NOT EXISTS topic_node (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    canonical_name TEXT NOT NULL UNIQUE,
    topic_type TEXT NOT NULL DEFAULT 'general',  -- general / project / person / decision
    first_seen_at TEXT NOT NULL,
    last_touched_at TEXT NOT NULL,
    mention_count INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_topic_canonical ON topic_node(canonical_name);
CREATE INDEX IF NOT EXISTS idx_topic_mention_count ON topic_node(mention_count DESC);

CREATE TABLE IF NOT EXISTS meeting_episode_node (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    topic_id INTEGER NOT NULL,
    meeting_id TEXT NOT NULL,
    excerpt TEXT,
    sentiment TEXT NOT NULL DEFAULT 'neutral',  -- positive / negative / neutral
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (topic_id) REFERENCES topic_node(id) ON DELETE CASCADE,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE,
    UNIQUE(topic_id, meeting_id)
);

CREATE INDEX IF NOT EXISTS idx_episode_meeting ON meeting_episode_node(meeting_id);
CREATE INDEX IF NOT EXISTS idx_episode_topic ON meeting_episode_node(topic_id);

CREATE TABLE IF NOT EXISTS relates_to (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_topic_id INTEGER NOT NULL,
    to_topic_id INTEGER NOT NULL,
    relation_type TEXT NOT NULL DEFAULT 'related',  -- related / causes / contradicts / supersedes
    strength REAL NOT NULL DEFAULT 1.0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (from_topic_id) REFERENCES topic_node(id) ON DELETE CASCADE,
    FOREIGN KEY (to_topic_id) REFERENCES topic_node(id) ON DELETE CASCADE,
    UNIQUE(from_topic_id, to_topic_id, relation_type)
);

CREATE INDEX IF NOT EXISTS idx_relates_from ON relates_to(from_topic_id);
CREATE INDEX IF NOT EXISTS idx_relates_to ON relates_to(to_topic_id);

CREATE TABLE IF NOT EXISTS topic_dossier (
    topic_id INTEGER PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'open',  -- open / resolved / parked
    summary TEXT,
    open_questions TEXT,
    last_decided TEXT,
    last_updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    rebuild_count INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (topic_id) REFERENCES topic_node(id) ON DELETE CASCADE
);
