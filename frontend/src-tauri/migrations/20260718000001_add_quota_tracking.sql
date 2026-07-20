-- v0.6.10+: 商业化配额追踪
-- 每个用户每月已用 meetings 计数 (自然月 UTC, 不必清零: 跟 created_at 关联记录)
-- 关键: 不加新表, 给 users 加 month_quota_key + month_meetings_used 列
ALTER TABLE users ADD COLUMN month_quota_key TEXT;          -- "2026-07"
ALTER TABLE users ADD COLUMN month_meetings_used INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_users_month_quota ON users(month_quota_key);

-- 升级历史购买激活日志 (admin 客服手工激活时记录, 给 C7 admin 后台看)
CREATE TABLE IF NOT EXISTS activation_orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email TEXT NOT NULL,                      -- 用户邮箱 (外键逻辑)
    tier TEXT NOT NULL,                       -- 'member'
    amount_cents INTEGER NOT NULL,            -- 8800 = ¥88
    currency TEXT NOT NULL DEFAULT 'CNY',
    channel TEXT NOT NULL,                    -- 'wxpay' | 'usdt' | 'card' | 'admin_grant'
    proof TEXT,                               -- 凭证 (交易号 / 截图描述)
    operator_email TEXT,                      -- admin 邮箱
    created_at TEXT NOT NULL,
    notes TEXT
);
CREATE INDEX IF NOT EXISTS idx_activation_orders_email ON activation_orders(email);
CREATE INDEX IF NOT EXISTS idx_activation_orders_created ON activation_orders(created_at);

-- 用户升级意向表 (C5 用户点 "我想升级" 时记录, 留联系方式)
CREATE TABLE IF NOT EXISTS upgrade_leads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email TEXT NOT NULL,
    contact TEXT,                             -- 微信号 / 邮箱 / 其他
    note TEXT,
    created_at TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'new'        -- 'new' | 'contacted' | 'paid' | 'lost'
);
CREATE INDEX IF NOT EXISTS idx_upgrade_leads_status ON upgrade_leads(status);
