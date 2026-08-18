# §132 history recovery 一直转 — Ollama 预检测 + 30s timeout + 进度 emit (2026-08-18)

## 触发

用户 8/18 截图: 进入会议脉络 → 紫色 banner "正在从历史会议摘要中回填主题 (首次进入自动执行, Ollama 推理约需 1~2 分钟)…" 一直转, 4 个 stat card 全 0, 右下角显示 offline (即 Ollama 未启).

## 根因 (3 跳)

1. **前端**: `knowledge/page.tsx:82` 调 `api_topic_extract_missing({maxMeetings: 30})` 同步等结果, recovering=true 一直显示
2. **后端**: `extract_missing_topics` 串行 await `trigger_after_summary` 每场
3. **底层**: `trigger_after_summary` `reqwest::Client` timeout = **120s/场**, Ollama 不可用时 30 场 × 120s = 60 分钟, 9 场 × 120s = 18 分钟
4. **文案误导**: "1~2 分钟" 是首次进入乐观估计, 实际 9 场全失败等 18 分钟

## 修复 (3 文件, +60/-10)

### 1) `frontend/src-tauri/src/topic_graph/mod.rs` 后端

- **`preflight_ollama_async()` 新函数** (3s timeout ping `http://localhost:11434/api/tags`):
  - 不可用 → emit `topic-recover-skipped` 事件 + return `(0, 0)`
  - 可用 → 继续
- **`trigger_after_summary` timeout 120s → 30s**:
  - Ollama connect refuse 通常 3s
  - qwen3.5:2b 推理 800 token ≤ 25s (实测 §120 RTF=0.291)
  - 单场 30s, 5 场 ≤ 2.5 min
- **`extract_missing_topics` 进度 emit**:
  - phase=start: `{ total, processed: 0 }`
  - phase=step (每场完): `{ total, processed, current_meeting }`
  - phase=done: `{ total, processed }`

### 2) `frontend/src/app/knowledge/page.tsx` 前端

- `maxMeetings: 30` → `5` (5 场 ≤ 2.5 min)
- 加 `recoverStatus: 'idle' | 'running' | 'ollama_offline' | 'done'` state
- useEffect 监听 `topic-recover-skipped` → 切 `ollama_offline` + 立刻 `setRecovering(false)`
- Ollama 不可用时显示琥珀色 banner: "历史主题回填已跳过 — Ollama 未运行. 启动 Ollama 后点击右上角'刷新'重试."
- 文案改准: "正在从历史摘要回填主题 (需 Ollama 在跑, 单场 ≤ 30s, 最多 5 场)…"
- import 加 `AlertCircle` (lucide-react)

### 3) `scripts/check_historical_fixes.py` 守卫

- 7 个 §132 锚点: preflight 函数 + skip emit + progress emit + 30s + maxMeetings:5 + listener + ollama_offline banner
- guard **266 → 273/273 PASS**

## §37 硬闸门

- tsc --noEmit: 0 errors (1 个 §18 bun:test 已知不动)
- next build: OK
- cargo check --lib: 0 errors (16 warnings §18 不动)
- cargo build --release: **2m18s**, binary ~70M **mtime 2026-08-18 12:00**
- check_historical_fixes.py: **273/273 PASS** (266 → 273)
- sync_app_bundle.sh: 全部 sync OK

## 验证 (binary grep)

```
preflight_ollama_async     FOUND
topic-recover-skipped      FOUND  (emit 事件字面量, 必定在 binary)
topic-recover-progress     FOUND  (emit 事件字面量, 必定在 binary)
```

源码字面量 `from_secs(30)` / `maxMeetings: 5` / `recoverStatus` 是 TS 字符串, Rust 编译期常量折叠, **不期望在 binary grep 中** (这是误判, 不是漏).

## §15 GUI 验收 (用户必做, 不能 CLI 测)

1. `killall meetily 2>/dev/null`
2. `open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'`
3. **路径 A — Ollama 不可用 (用户当前状态)**:
   - 进 /knowledge → 紫色 banner 应该几秒内消失
   - 替换为琥珀色 banner: "历史主题回填已跳过 — Ollama 未运行..."
   - 4 个 stat card 仍 0 (无内容)
   - **不再 "一直转"**
4. **路径 B — 启 Ollama 后**:
   - `ollama serve` + `ollama pull qwen3.5:2b`
   - 进 /knowledge → 紫色 banner 转 5 场 ≤ 2.5 min
   - 4 个 stat card 出现非 0 数字
5. 验证 DB:
   ```bash
   sqlite3 ~/Library/Application\ Support/tech.yanjingai.app/meeting_minutes.sqlite \
     "SELECT COUNT(*) FROM topic_node"
   # Ollama 在跑: 应该 ≥ 8 (每场 cap 8 topic_node)
   ```

## 关联

- §126 (history recovery 起点, maxMeetings 30 偏大)
- §121 (trigger 改 Ollama + 失败 emit 事件, 这次复用事件机制)
- §56 (AGENTS.md §X 描述 vs 代码 commit — 这次 commit 当日完成)
- §92 (决策迁移铁律)
- §37 (硬闸门)
- §15 (GUI 验收强制)
- [[132-history-recovery一直转-修复]] (Obsidian) / `outputs/§132-...md` (Codex)
