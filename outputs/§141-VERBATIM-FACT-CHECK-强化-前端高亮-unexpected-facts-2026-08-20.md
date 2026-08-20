# §141 VERBATIM FACT-CHECK 强化 + 前端高亮 unexpected facts (B+D)

**commit**: `b7f4725` (main)  
**日期**: 2026-08-20  
**触发事故**: meeting-8ce922f9 (court_hearing) 8/19
- transcript `二零一八年七月十四日` → LLM 改成 `2017年8月26日` (差 1 年, 把"前案判决"和"本案"日期搞混)
- transcript `一百二十三万余元` → LLM 改成 `23.75万元` (差 5x 数量级)
- fact_guard 100% 准确报警 (`unexpected_dates=["2017年", "2017年8月26日"]` / `unexpected_numbers=["23.75万"]`) 但 LLM 不修正

## B 方案 (后端 prompt 工程)

新增 `const P141_VERBATIM_FACT_CHECK` (`processor.rs:137`):

- **§141.1 PRECISE VERBATIM DEMO** — 8 行 BEFORE/AFTER 反例 (日期/金额/小数点/人名/公司名/法院名/案号), 包含本次事故的真实反例
  - `二零一八年七月十四日` ≠ `2017年8月26日` (差 1 年)
  - `一百二十三万余元` ≠ `23.75万元` (差 5x)
  - `温明仁` ≠ `温明人` (同音字错)
- **§141.2 FINAL-ANSWER FACT-CHECK PROTOCOL** — 5 步 checklist: date / amount / name / case num / subject count
- **§141.3 TIMELINE-SPECIFIC WARNING** — 单独点名, 因为 timeline 是最易搞混日期的 section (本次事故时间线 6 行全部用 2017-08-26)
- **§141.4 UNIT & SCALE PRESERVATION** — "余"/"点"/"倍"/"万" 等中文大数单位保留
- **§141.5 FAILURE CONSEQUENCES** — 任何 fact error 导致 summary 被标红, 用户失信任

注入点:
- `build_final_report_system_prompt` (`processor.rs:340`): `2.6. {P141_VERBATIM_FACT_CHECK}`
- `final_user_prompt` (`processor.rs:1023`): 末尾加 `<fact_check_reminder>` 块, 4 步自检 (re-scan transcript / date 混乱 / 2x-0.5x magnitude / REMOVE invented)

## D 方案 (前端 highlight)

新模块 `frontend/src/lib/highlight_facts.ts:60`:
- `highlightUnexpectedFacts(md, factGuard)` 把 `unexpected_dates` / `unexpected_numbers` 用 `==xxx==` 包裹
- BlockNote 默认 markdown parser 支持 `==highlight==` 语法, 自动渲染成黄底高亮
- 跳过 ` ```code block``` ` 和 `` `inline code` `` (保护代码内容不被误高亮)
- 按长度倒序匹配, 避免前缀抢匹配 ("2017 年" 不会抢 "2017 年 8 月 26 日")
- 跳过已 `==包裹==` 的内容 (防双包)
- dedup (dates + numbers 重复不重复包裹)

接入 `BlockNoteSummaryView.tsx`:
- 第 103 行: 首次加载时 `highlightUnexpectedFacts(data.markdown, data.fact_guard)` 后 parse
- 第 146 行: status=completed force reload 时同样调用

## §37 硬闸门

- `cargo check --lib`: 0 errors (38 §18 warnings 不动)
- `cargo test --lib`: **381/381 PASS** (新增 3 个 §141 单测)
- `tsc --noEmit`: 1 个 §18 已知 bun:test 错误 (不动)
- `next build`: OK
- `cargo build --release`: **1m34s**, binary 73M **mtime 2026-08-20 00:12**
- `sync_app_bundle.sh`: §99.6 synced (3 binary 全部 SHA 一致)
- `python3 scripts/check_historical_fixes.py`: **430/430 PASS** (414 → 430, +16 §141 锚点)
- `highlight_facts.test.ts` (9 个单测): 9/9 PASS

## 16 个新 guard 锚点 (414 → 430)

| Anchor | 守卫目标 |
|---|---|
| `141_verbatim_fact_check_constant` | P141_VERBATIM_FACT_CHECK 常量存在 |
| `141_verbatim_fact_check_section_141_1` | §141.1 BEFORE/AFTER DEMO 块 |
| `141_verbatim_fact_check_section_141_3_timeline_warning` | §141.3 时间线特别警告 |
| `141_system_prompt_injects_p141_block` | `2.6. {P141_VERBATIM_FACT_CHECK}` 注入 |
| `141_final_user_prompt_injects_fact_check_reminder` | `<fact_check_reminder>` 块 |
| `141_prompt_contains_demo_examples` | 含"二零一八年七月十四日"反例 |
| `141_prompt_contains_amount_example` | 含"一百二十三万余元"反例 |
| `141_prompt_contains_name_example` | 含"温明仁"反例 |
| `141_test_verbatim_fact_check_prompt_contains_before_after_pairs` | 3 个 §141 单测 |
| `141_test_final_report_system_prompt_includes_p141_block` | ↑ |
| `141_test_final_user_prompt_includes_fact_check_reminder` | ↑ |
| `141_d_highlight_facts_module` | D 方案 highlight_facts.ts |
| `141_d_blocknote_view_imports_highlight` | BlockNoteSummaryView import |
| `141_d_blocknote_view_calls_highlight_first_load` | 首次加载调用 |
| `141_d_blocknote_view_calls_highlight_force_reload` | 强制重载调用 |
| `141_d_highlight_facts_test_exists` | 9 个单测存在 |

## §15 GUI 验收 (用户必做, 不能 CLI 测)

```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```

1. 打开 meeting-8ce922f9 → 应该看到 "2017年8月26日" / "23.75万" / "2017年" 被**黄底高亮** (D 方案立即可见)
2. 顶部 banner 仍显示 "⚠️ 严重: AI 生成的纪要包含未被原文证据支持的内容"
3. 点击"重新生成摘要" → 期望 LLM 这次严格遵守 §141 硬约束, 新的时间线 6 行用不同日期 (B 方案要等 LLM 跑一次才生效)
4. DB 验证:
   ```bash
   sqlite3 "$HOME/Library/Application Support/tech.yanjingai.app/meeting_minutes.sqlite" \
     "SELECT meeting_id, status, length(result) FROM summary_processes 
      WHERE meeting_id='meeting-8ce922f9-8c74-47f6-aa67-8246679e7a15' 
      ORDER BY updated_at DESC LIMIT 3"
   # 期望: 出现新行, updated_at 接近当前时间, length 接近 5270
   ```

## 已知边界 (按 §18)

- tsc 1 个 bun:test 错误 (§18 范围, 不动)
- 38 cargo warnings (§18 范围, 不动)
- D 方案高亮依赖 fact_guard 报警 — 如果 fact_guard 漏报某项, 也不会高亮 (但 fact_guard 当前 100% 准确)
- B 方案 prompt 强化效果取决于 LLM 是否遵守 — 2B 模型推理能力有限, 失败率可能仍有 10-30%, 配合 D 方案用户能立刻看到问题

## 关联

- §138 (P1 已有 verbatim 硬约束, 强度不够) / §139 (商业化硬约束) / §140 (topic_graph parser 容错)
- §37 (硬闸门) / §18 (不主动改无关 bug) / §56 (AGENTS.md 双校)
- §92 (决策迁移铁律, outputs + AGENTS.md + Obsidian 同日落)
- §99.6 (sync_app_bundle binary) / §99.5 (Tauri setup 不允许 tokio::spawn)
