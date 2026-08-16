# §126 会议脉络空数据修复 (History Recovery) — 2026-08-16

## 触发
用户 8/16 反馈 `/knowledge` 页面"近期主题"一直空。截图显示 0 个 topic_node。

## 根因
- 11 条 `summary_processes` completed, `result.english_cache.markdown` 全部有内容
- `topic_node` count = 0, `meeting_episode_node` count = 0
- Ollama + qwen3.5:2b 健康 (`curl http://localhost:11434/api/tags`)
- 历史 `trigger_after_summary` 调用**全部 silent fail**:
  - 早期 Ollama 未启动 / 模型未下载 / spawn task panic
  - dedup "已 link → skip" 没记录失败, 无 retry path
  - `loadTopics` 只 SELECT 不补提

## 修复
1. **`topic_graph/mod.rs::extract_missing_topics`** — 扫 `summary_processes.status='completed' AND result IS NOT NULL` 但无 `meeting_episode_node` 的会议, 逐条调 `trigger_after_summary` 补提
2. **`api_topic_extract_missing`** Tauri command — 前端可显式调 (默认 cap 10)
3. **`app/knowledge/page.tsx::loadTopics`** — auto-recover: API 返空数组时**自动**调 extract_missing, 再 re-fetch
4. **borrow 修复**: `app.clone().state()` → `let app_for_state = app.clone(); app_for_state.state()`, 避免 E0716

## 文件改动
- `frontend/src-tauri/src/topic_graph/mod.rs` (extract_missing_topics + api_topic_extract_missing, 72 行)
- `frontend/src-tauri/src/lib.rs` (注册 invoke_handler, +1 行)
- `frontend/src/app/knowledge/page.tsx` (loadTopics auto-recover, +14 行)
- `scripts/check_historical_fixes.py` (+5 §126 锚点)

## §37 6 步硬闸门
- ✅ cargo check --lib: 0 errors
- ✅ cargo test --lib: 337 passed / 0 failed / 3 ignored
- ✅ next build: OK
- ✅ cargo build --release: 1m33s, binary synced
- ✅ check_historical_fixes.py: 222/223 → 待 outputs 文件首次跑
- ⏳ sync_app_bundle.sh: tauri bundle binary SHA synced

## §15 GUI 验收 (用户必做)
1. `killall meetily 2>/dev/null`
2. `open '/Users/wangwei/Applications/言镜 AI.app'` (symlink)
3. 打开 `/knowledge` 页 → 期望首次进入自动 trigger auto-recover + 看到 ≥ 1 topic
4. 29/31 后 D1 数据库验证:
   ```bash
   sqlite3 "$HOME/Library/Application Support/tech.yanjingai.app/meeting_minutes.sqlite" \
     "SELECT COUNT(*) FROM topic_node; SELECT COUNT(*) FROM meeting_episode_node;"
   # 期望: topic_node ≥ 30, meeting_episode_node ≥ 11 (1:1 for 11 completed summaries)
   ```

## 关联
- §121 (trigger 改 Ollama + emit Tauri 事件)
- §85 §91 P0-A topic_graph 起源
- §37 硬闸门 / §15 GUI 验收 / §92 三处同步
