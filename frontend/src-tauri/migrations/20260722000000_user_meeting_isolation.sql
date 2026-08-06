-- v0.7.0+: 用户 × 会议数据隔离 (P0 安全加固)
-- 背景: 之前 meetings 表无 user_id, 任何登录用户能看到所有会议. 严重数据泄漏.
-- 改法: 加 user_id 列 + 索引 + 回填现有数据到「机器 owner」 + 重写 command 按 user_id 过滤.

-- 1) meetings 加 user_id 列 (NULLABLE 先, 回填完改 NOT NULL)
ALTER TABLE meetings ADD COLUMN user_id INTEGER;

-- 2) 回填策略: 把所有现有 meeting 归到「首次用这台机器的用户」
--    即 SELECT MIN(id) FROM users WHERE machine_id = 当前 machine_id
--    这里用一段 Rust 代码在启动时跑 (lib.rs startup), migration 只负责加列
--    启动后那段代码执行 UPDATE meetings SET user_id = <owner_id> WHERE user_id IS NULL
--    回填后我们再 ALTER COLUMN 改 NOT NULL (在 20260722000001 migration 做)

-- 3) 索引: 按 user_id 查会议是主要路径, 必须有索引
CREATE INDEX IF NOT EXISTS idx_meetings_user_id ON meetings(user_id);
CREATE INDEX IF NOT EXISTS idx_meetings_user_created ON meetings(user_id, created_at DESC);

-- 4) 同理 transcripts 表加 user_id 列 (避免每查都 join meetings)
ALTER TABLE transcripts ADD COLUMN user_id INTEGER;
CREATE INDEX IF NOT EXISTS idx_transcripts_user_id ON transcripts(user_id);
