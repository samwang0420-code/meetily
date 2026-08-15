# §121 P0-A / P2-C LLM trigger 必须用 Ollama 不 BuiltInAI (2026-08-15 立)

## 触发

用户最近 8/14 导入 `566fe7a9-07a3-4415-8265-1f7aa2ee0e84` (1h49m 音频 / 537 段 / 26302 字 / duration 6597s) → 转录 OK → 但 topic_node / topic_dossier 全部 0 行.

- `sqlite3 ... "SELECT COUNT(*) FROM topic_node"` → **0**
- `sqlite3 ... "SELECT COUNT(*) FROM topic_dossier"` → **0**

P0-A 知识图谱从未触发过. ⌥+Space LiveQA 用户按也没反应 (未修前的同类 bug).

## 根因 (5 跳)

1. `frontend/src-tauri/src/topic_graph/mod.rs:397` (`trigger_after_summary`) → `LLMProvider::BuiltInAI`
2. `frontend/src-tauri/src/topic_graph/mod.rs:538` (`rebuild_topic_dossier`) → `LLMProvider::BuiltInAI`
3. `frontend/src-tauri/src/live_qa/mod.rs:131` (`run_live_qa`) → `LLMProvider::BuiltInAI`
4. `frontend/src-tauri/src/summary/llm_client.rs` `generate_summary` 对 `BuiltInAI` 强制要 `app_data_dir: Option<&Path>` (sidecar binary `llama-helper` 路径)
5. 3 处 trigger 链都传 `None` → `generate_summary` 内部 `.ok_or_else(|| "app_data_dir is required for BuiltInAI")` → Err → 上层 swallow 写 warn log

**结果**: 用户从不知情, 触发链 silent fail, 知识图谱表全空.

## 修复

3 处全部改用 `LLMProvider::Ollama`, 走 `localhost:11434`, 用户机器已跑 `qwen3.5:2b` (2.74GB).

### §121 铁律 #3: 禁止 BuiltInAI swallow log

`trigger_after_summary` 和 `rebuild_topic_dossier` 的 LLM 失败从 `log::warn!` 升级为:
- `log::error!` (production 日志可见)
- `app.emit("topic-extract-failed" / "topic-dossier-failed", {meeting_id, error, at})` (前端 toast)

前端 `frontend/src/app/meeting-details/page.tsx` 加 listener:
```ts
listen<{ meeting_id: string; error: string }>('topic-extract-failed', async (event) => {
  if (event.payload.meeting_id === meetingId) {
    toast.error('主题提取失败: ' + event.payload.error);
  }
});
```

触发链任何 LLM call 失败, 用户立即看到 toast, 不再 silent.

## §121 铁律

1. **任何 spawn hook / 异步 trigger 调 BuiltInAI 必须传 `app_data_dir: Some(&app.path().app_data_dir()?)`** —— 否则永远 fail.
2. **或者改用 Ollama (`localhost:11434`)** —— 本地 Ollama 在 P0-A / LiveQA 这种 trigger 链路更稳, 不依赖 sidecar binary 启动.
3. **禁止 BuiltInAI swallow log**: trigger 链路任何 LLM call 失败必须升级 error + emit Tauri 事件 (前端 toast 可见), 不能再 silent.
4. **新增 trigger 必加单元测试 mock LLM**: 防止 "传 None" 类 bug 永远跑不到.
5. **任何 §X 改动 LLM 调用必须 cargo test + 实跑 trigger 一次验证 DB 表非空**.

## 验证

```bash
# 1) cargo check
cd frontend/src-tauri && cargo check --lib   # 0 errors, 28 §18 warnings 不动

# 2) cargo test
cargo test --lib --no-fail-fast              # 337 passed / 0 failed / 3 ignored

# 3) guard
cd /Users/wangwei/Documents/离线会记 && python3 scripts/check_historical_fixes.py
# 180/180 PASS (含 5 个 §121 anchor + 4 个 §122 anchor)

# 4) 重生成 28a6c63c 摘要 (老 completed summary, 强制 re-trigger)
# GUI: 打开 28a6c63c → 重新生成摘要 → 等待 1-3 min
# 后端日志: "[topic_graph] spawn for meeting=... summary=3377 chars" → "[topic_graph] ... parsed N topics"
sqlite3 "$DB" "SELECT COUNT(*) FROM topic_node"   # 期望 > 0
sqlite3 "$DB" "SELECT COUNT(*) FROM topic_dossier" # 期望 > 0

# 5) ⌥+Space LiveQA
# 进 meeting → 按 ⌥+Space → 输入问题 → 期望 3 条建议 ~12s 内返回
```

## §15 GUI 验收 (用户必做, 不能 CLI 测)

```bash
killall meetily 2>/dev/null
bash /Users/wangwei/Documents/离线会记/scripts/sync_app_bundle.sh
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'

# 1) 重生成 28a6c63c 摘要 (任一老 summary completed 会议)
#    等待 ~2 min, 日志含 "parsed N topics" (N > 0)
#    sqlite3 topic_node 应该 > 0

# 2) ⌥+Space LiveQA
#    进任一 meeting → 按 ⌥+Space → 输入 "刚才讨论的核心点是什么" → 期望 3 条建议

# 3) 验证 §121 铁律 #3: 关 Ollama → 重生成摘要 → 期望 toast "主题提取失败: ..."

# 4) 关 Ollama → 按 ⌥+Space → 期望 toast "LiveQA 调用失败: ..." (后续 §123 加)
```

## 回退方案

```bash
cd /Users/wangwei/Documents/离线会记
git reset --hard 21ebdbe   # §121 之前的 HEAD
cd frontend/src-tauri && cargo build --release
bash scripts/sync_app_bundle.sh
```

## 关联

- `frontend/src-tauri/src/topic_graph/mod.rs` (改) - 2 处 trigger + 2 处 emit failure event
- `frontend/src-tauri/src/live_qa/mod.rs` (改) - P2-C ⌥+Space trigger
- `frontend/src/app/meeting-details/page.tsx` (改) - 监听 topic-extract-failed + topic-dossier-failed, toast 提示
- `scripts/check_historical_fixes.py` (改) - 5 个 §121 anchor (171 → 176 → 180)
- §91 (P0-A 完整化收尾, §121 是其 silent-fail 补丁)
- §88 (P2-B/C 收尾, §121 修复 §P2-C ⌥+Space)
- §85 (MVP 起点) / §18 (不主动改无关 bug) / §37 (硬闸门) / §15 (GUI 验收)
- §99.5 (Tauri spawn 边界 — 不同话题但容易混淆)
- [[121-P0-A-LLM-trigger-改Ollama-2026-08-15]] (Obsidian 主份)
