-- 离线会记 v0.7.0: session 持久化 (免登录体验)
-- 原 SessionStore 是内存 HashMap, app 重启后失效, 用户每次编译都要重新登录.
-- 现在 token 存 DB, app 启动时 user_get_current 走 DB 校验, session 跨进程有效.

CREATE TABLE IF NOT EXISTS auth_sessions (
    token TEXT PRIMARY KEY,                       -- 32-char hex session token
    user_id INTEGER NOT NULL,
    created_at TEXT NOT NULL,                    -- ISO 8601
    last_seen_at TEXT NOT NULL,                  -- ISO 8601, 用于检测过期
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_auth_sessions_user_id ON auth_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_auth_sessions_last_seen ON auth_sessions(last_seen_at);
