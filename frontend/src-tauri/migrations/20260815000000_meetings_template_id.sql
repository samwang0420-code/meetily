-- Migration: §123 模板选择持久化
-- 用户每次起新会议/重新生成摘要时, 选了什么模板写入 meetings.template_id,
-- 下次进入会议详情默认显示该模板 (而不是每次都回 standard_meeting).

-- 1. 加列 (可空, 老数据保持 NULL → 前端 useTemplates fallback 到 standard_meeting)
ALTER TABLE meetings ADD COLUMN template_id TEXT;

-- 2. 老 completed summary 反推 template_id (用当前 summary_processes 关联不严谨,
--    LLM 不直接存 template_id, 简单留 NULL → 前端 fallback standard_meeting)
--    不写 SQL.

-- 3. 索引 (按 template_id 查"所有用 X 模板的会议"将来可能用到)
CREATE INDEX IF NOT EXISTS idx_meetings_template_id ON meetings(template_id);
