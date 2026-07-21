-- v0.7.0+ rc6: 简化迁移.
-- 老版用 12-step rebuild (settings_new + INSERT ... SELECT * FROM settings),
-- 在 schema 不一致时会 panic 'N columns but M values supplied'.
-- 初始 schema (20250916100000) 现在已经包含 openRouterApiKey 列 (v0.7.0+ 同步补上),
-- 所以这条 migration 不再需要做任何事.
SELECT 1;
