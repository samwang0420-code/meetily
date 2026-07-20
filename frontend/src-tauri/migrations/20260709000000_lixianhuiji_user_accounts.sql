-- 离线会记 v0.5.0: 用户账号 + 会员买断 (¥88 永久)
-- 与原有 licensing (RSA-based) 表兼容, 用户登录态与机器码强绑定

CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,                 -- SHA-256(password + salt), 不存明文
    salt TEXT NOT NULL,                          -- 16-byte hex random salt
    machine_id TEXT,                             -- 首次登录时绑定的机器码
    display_name TEXT,                            -- 可选显示名
    created_at TEXT NOT NULL,                    -- ISO 8601
    last_login_at TEXT,                          -- ISO 8601
    -- 会员态 (参考原 licensing 表的字段, 但永久买断, 无 expiry)
    membership TEXT NOT NULL DEFAULT 'free',      -- free / member
    membership_activated_at TEXT,
    license_key TEXT,                            -- 首次激活码 (复用 licensing 表)
    is_active INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_machine_id ON users(machine_id);

-- 会员购买历史 (用 license_key 复用原 licensing 表)
-- 因为我们的会员是"¥88 永久买断绑定机器", 我们直接用 licensing 表的 license_key
-- 一条记录 = 一台机器绑定一个 license
INSERT OR IGNORE INTO licensing (license_key, encrypted_key, signature_hash, activation_date, expiry_date, soft_expiry_date, max_activation_time, duration, generated_on, is_soft_expired, grace_period)
VALUES ('member-bundle-2026-07-v1', 'local-bundle', '00', '2099-12-31T00:00:00Z', '2099-12-31T00:00:00Z', '2099-12-31T00:00:00Z', '2099-12-31T00:00:00Z', 9999999999, '2026-07-09T00:00:00Z', 0, 0);

-- 用户偏好扩展 (保存 hotwords 选择)
-- 与 settings 表不同, 我们把 hotwords 单独建表
CREATE TABLE IF NOT EXISTS hotwords_config (
    user_id INTEGER PRIMARY KEY,
    builtin_pack TEXT NOT NULL DEFAULT 'none',    -- none / legal / education / medical / finance / tech
    custom_words TEXT NOT NULL DEFAULT '',        -- 逗号分隔
    enabled INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);
