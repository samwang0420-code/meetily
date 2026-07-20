-- v0.6.10+: Pro 激活码体系 (C4)
-- 给 admin 用于批量发内测买断授权 / 种子用户发放
ALTER TABLE users ADD COLUMN activated_via_code TEXT;  -- 用了哪个码 (留 audit)

CREATE TABLE IF NOT EXISTS activation_codes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code TEXT NOT NULL UNIQUE,                  -- 16 字符 base32, 例: PROMO-A7K9-3F2X-QWE4
    tier TEXT NOT NULL DEFAULT 'member',        -- 'member' (将来可 'team' / 'enterprise')
    duration_days INTEGER NOT NULL DEFAULT 365, -- 1 年有效期
    expires_at TEXT NOT NULL,                   -- ISO 8601, "2027-07-18T10:00:00Z"
    used_by_user_id INTEGER,                    -- NULL = 未用, 用了就不许再激活
    used_at TEXT,
    generated_by_operator TEXT,                 -- admin 邮箱 / "system"
    note TEXT,                                  -- "种子内测发放" / "Pro 买断 ¥88"
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (used_by_user_id) REFERENCES users(id)
);
CREATE INDEX IF NOT EXISTS idx_codes_code ON activation_codes(code);
CREATE INDEX IF NOT EXISTS idx_codes_used ON activation_codes(used_by_user_id);
CREATE INDEX IF NOT EXISTS idx_codes_expires ON activation_codes(expires_at);
