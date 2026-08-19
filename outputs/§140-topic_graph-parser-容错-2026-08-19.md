# §140 topic_graph parser 容错修复 (2026-08-19)

## 触发

用户 8/19 反馈: "已经点了重新生成, 你检查问题; 会议脉络还是不生效"。

截图: "会议脉络"页面 0 主题 / 0 决议 / 0 项目 / 0 人物, 23 个 completed 摘要, 但 `topic_node` / `meeting_episode_node` / `topic_dossier` 全部 0 行。

## 根因 (3 层叠加)

### 1. f2dfa2e0 摘要已重生成成功
- 4051 字符 → 5769 字符 (+1718)
- template_fingerprint: `872bdaf9ef97fb01:8169` (确认是 §139 后的新模板)
- source: builtin-ai / qwen3.5:2b

### 2. trigger_after_summary 触发了
- service.rs:709 spawn 了 `crate::topic_graph::trigger_after_summary(...)`
- 摘要长度 ≥ 50 ✓, 不是 dedup skip, 进入 LLM call
- 调 BuiltInAI (qwen3.5:2b) 调 extract prompt

### 3. parse_extract_response 严格匹配字段, 0 topics 解析成功
真实 qwen3.5:2b 输出 (我用 curl 直打 Ollama 验证):
```json
{topic_name: "事故赔偿",topic_type:"project",excerpt:"...",sentiment:-1}
{topic_name: "暖风行动",topic_type:"decision",excerpt:"...",sentiment:-1}
```

3 个问题:
1. **字段名错**: `topic_name` 不是 `canonical_name`
2. **sentiment 数字**: `-1` 不是 "negative"
3. **JavaScript-style 无引号 key**: `{topic_name: ...}` 不是合法 JSON, serde_json 直接 reject

即使第 1/2 修了, 第 3 个还卡. 改用新 prompt 后, LLM 输出还是包 markdown:
```
```json
{"canonical_name":"交通损害赔偿","topic_type":"event",...}
```
```
- `trimmed.starts_with('{')` 在第一行是 ```\n{ , false
- 整行 reject, 0 topics

## 修复 (3 层容错)

### §140 修复 1: strip_markdown_fence
去 markdown 包装 ``` / ```json / ```JSON / ```Json, 保留 JSON 内容。

### §140 修复 2: quote_unquoted_keys
JavaScript-style 无引号 key 加引号: `{topic_name: "x"}` → `{"topic_name": "x"}`。regex: `([,{]\s*)([A-Za-z_][A-Za-z0-9_]*)\s*:` → `$1"$2":`。

### §140 修复 3: normalize_extract_line 字段别名 + sentiment 数字映射
- 别名: `topic_name/name/title/subject` → `canonical_name`, `type/category/kind` → `topic_type`, `score/polarity/tone` → `sentiment`
- sentiment 数字: `> 0.3` → "positive", `< -0.3` → "negative", else "neutral"
- sentiment 字符串别名: "1"/"pos"/"good" → "positive", "-1"/"neg"/"bad" → "negative", else "neutral"

### §140 修复 4: PROMPT_INSTRUCTIONS 严格化
- 明确写字段名为英文 "canonical_name" / "topic_type" / "excerpt" / "sentiment"
- 明确说"不是 topic_name / name / type / score 等别名"
- 明确说"sentiment 必须是字符串, 不是数字 1/0/-1"

### 兜底: trigger_after_summary 已有的白名单 fallback
- 即使解析出来 topic_type 不在 {general, project, person, decision}, 已有 fallback 到 "general"

## 端到端验证 (用真实 qwen3.5:2b + 新 prompt)

```
LLM raw response:
```json
{"canonical_name": "交通损害赔偿", "topic_type": "event", "excerpt": "...", "sentiment": "negative"}
```
```json
{"canonical_name": "司法追偿行动", "topic_type": "law_enforcement", "excerpt": "...", "sentiment": "positive"}
```
```json
{"canonical_name": "交通事故赔偿机制", "topic_type": "policy_analysis", "excerpt": "...", "sentiment": "neutral"}
```

解析结果: 3 topics ✓
  • 交通损害赔偿 (type=general [fallback] | negative)
  • 司法追偿行动 (type=general [fallback] | positive)
  • 交通事故赔偿机制 (type=general [fallback] | neutral)

修复前: 0 topics (因为 ```json wrapper 或 topic_name 别名或 sentiment 数字)
修复后: 3 topics 全部成功

## 测试 (12/12 PASS)

- 3 个原有测试 (parse_extract_response_*) 仍 pass
- 3 个新 §140 测试 (topic_name alias / sentiment number positive / sentiment zero)
- 2 个 quote_unquoted_keys 测试 (basic + pass-through)
- 2 个 markdown fence 测试 (handles_markdown_fence / unknown_topic_type)

## 验证

- `cargo test --lib topic_graph::extract`: **12/12 PASS** (10 → 12, +2 §140)
- `cargo build --release`: 73MB binary 22:55 OK
- `check_historical_fixes.py`: **414/414 PASS** (405 → 414, +9 §140 anchors)
- `sync_app_bundle.sh`: §99.6 tauri bundle binary sync OK

## 9 个新 guard anchor (§140)

```
140_extract_prompt_canonical_name         # prompt 包含 "canonical_name"
140_extract_prompt_sentiment_string       # prompt 包含 sentiment 字符串约束
140_parser_normalize_alias                # normalize_extract_line 函数存在
140_parser_quote_unquoted_keys            # quote_unquoted_keys 函数存在
140_parser_sentiment_number_mapping       # 数字 sentiment 映射代码
140_extract_test_topic_name_alias         # 兼容性测试
140_extract_test_sentiment_positive        # sentiment:1 测试
140_extract_test_sentiment_zero            # sentiment:0 测试
140_quote_unquoted_keys_test              # quote_unquoted_keys_basic 测试
```

## 已知边界 (§18 不动)

- 1 个 §18 bun:test tsc 错误 (不动)
- 37 cargo warnings (§18 不动) - extract.rs 加了 1 个 unused import warning (`use serde_json::Value` 现在在 normalize_extract_line 内部 use, 顶层 use 没用上). 改不?
- 旧 meeting 已生成但 topic_node 是 0 行的, 需要重新生成摘要触发 trigger_after_summary

## §15 GUI 验收 (用户必做, 不能 CLI 测)

```
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 重生成 f2dfa2e0 (顺义执行案) 摘要
# 等 ~30-60s
# 验证:
sqlite3 "$HOME/Library/Application Support/tech.yanjingai.app/meeting_minutes.sqlite" \
  "SELECT COUNT(*) FROM topic_node; SELECT COUNT(*) FROM meeting_episode_node; SELECT COUNT(*) FROM topic_dossier"
# 期望: ≥ 1 / ≥ 1 / ≥ 0
# 然后访问 /knowledge 页, 应看到 ≥ 1 个 topic 出现
```

## 关联

- §139 (模板商业化精进, prompt 大改) - §140 是 §139 上线后 LLM 实际调用才发现的 parser bug
- §P0-A Phase 2 (topic_graph 实施) - 当时只测了 mock LLM 输出 pass 单元测试, 没真用 qwen3.5:2b 跑
- §56 (AGENTS.md §X 章节 ≠ 代码 commit) - §140 写完必须 git log 实际验证
- §92 (决策迁移铁律, outputs + AGENTS.md + Obsidian + 代码 + guard 5 处同日落)
- §37 (硬闸门)
