# §111 Ollama /api/chat + think:false — topic extract 全空修复 (2026-08-19)

**commit**: `c6c01dd` (branch main, push OK)
**触发**: 用户截图"主题提取失败: LLM request timed out after 60 seconds" + DB 显示最近 5 个会议 **全部 0 episode**(topic extract 系统性失败)
**binary**: target/release/meetily mtime 2026-08-19

## 根因 (3 跳)

1. **OpenAI-compat wrapper bug**: Ollama `/v1/chat/completions` 端点对 qwen3.5:2b 等 thinking 模型返回 `content=""` (空字符串) — 实测空响应 27.9s
2. **thinking mode 默认开**: qwen3.5:2b capability 含 `"thinking"`, `/api/generate` 不加 `think:false` 时 800 token 全耗在 thinking, content 仍空 — 实测 40s
3. **错误文案误导**: `llm_client.rs` 硬编码 `"after 60 seconds"`, 实际 `REQUEST_TIMEOUT_DURATION = 300s`, `trigger_after_summary` 外层 30s timeout — 用户看到的秒数跟实际不符

DB 实证:
```
meeting-213a1c41  赵立昌故意伤害案     166 transcripts  7742 chars  0 episode
meeting-911f52ae  魏某专利侵权案        304 transcripts 23631 chars  0 episode
meeting-908b5960  家庭纠纷与情感冲突     48 transcripts   454 chars  0 episode
meeting-67026eca  Untitled              112 transcripts  5727 chars  0 episode
meeting-4aa26c7e  走私运输毒品案公开审理 102 transcripts  5890 chars  0 episode
```

## Ollama API 实测对比

| 调用 | 时间 | content | tokens |
|---|---|---|---|
| `/api/generate` no think | 1.8s | ✅ "案件审判/司法公正..." (7 行) | 21 |
| `/api/generate` thinking 默认开 | 40s | ❌ "" | 800 (空) |
| `/v1/chat/completions` no think | 2s | ❌ "" | 800 |
| `/v1/chat/completions` think:false | 28s | ❌ "" | 800 |
| **`/api/chat` think:false** | **0.5s** | **✅ "8"** | **9** |
| **`/api/chat` think:false + 真 prompt (1500 chars)** | **15.9s** | **✅ 5 个有效主题** | **120** |

结论:**Ollama 原生 `/api/chat` + `think:false` 是唯一可用路径**。OpenAI-compat wrapper 在 thinking 模型上完全坏。

## 修复 (3 文件, +103/-8)

### 1. `frontend/src-tauri/src/summary/llm_client.rs`
- Ollama URL `/v1/chat/completions` → `/api/chat`
- Ollama 请求体加 `"think": false` (原生 schema, options.num_predict 传 max_tokens)
- Ollama 响应解析改 `message.content` (原生) 而非 `choices[0].message.content`
- 新增 `parse_ollama_stream_line` helper (JSON Lines 解析, 非 OpenAI SSE `data: ...\n\n`)
- 错误文案 `"after 60 seconds"` → `"after {} seconds"` 用 `REQUEST_TIMEOUT_DURATION.as_secs()` 动态反映

### 2. `frontend/src-tauri/src/topic_graph/mod.rs`
- `trigger_after_summary` 外层 reqwest client timeout 30s → **90s** (冷启动 + 大摘要仍可能慢)
- §132 注释更新:实测 qwen3.5:2b 推理时间远超 §132 估计 (think mode 吃满)

### 3. `scripts/check_historical_fixes.py`
- §132_timeout_30s → §111_timeout_90s
- 新增 4 个 §111 锚点: ollama_native_api_chat / ollama_think_false / ollama_stream_parser / error_msg_dynamic_timeout
- guard 337 → **341/341 PASS**

## §37 硬闸门

- ✅ tsc --noEmit: 0 errors (§18 bun:test 不动)
- ✅ next build: 11s OK
- ✅ cargo check --lib: 0 errors (36 §18 warnings 不动)
- ✅ cargo test --lib: **364/364 PASS**
- ✅ cargo build --release: 1m42s, binary 73M
- ✅ check_historical_fixes.py: **341/341 PASS**
- ✅ sync_app_bundle.sh: 3 binary 全 sync (main + llama-helper + ffmpeg)
- ✅ Ollama `/api/chat` 实测 15.9s/120 tokens 返 5 个有效主题

## §15 GUI 验收 (用户必做)

```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 1. 进 /knowledge 页 → 点"重新生成摘要"任一旧会议
# 2. 不应再弹 "主题提取失败: LLM request timed out after..."
# 3. DB 验证:
DB="$HOME/Library/Application Support/tech.yanjingai.app/meeting_minutes.sqlite"
sqlite3 "$DB" "SELECT meeting_id, COUNT(*) FROM meeting_episode_node GROUP BY meeting_id"
# 期望: 旧 0 episode 的会议应开始有 episode (重生成后 trigger_after_summary 触发)
```

## 摘要质量评估 (用户截图 meeting-213a1c41)

**整体**: 70 分 (专业可读, 接近庭审记录标准)

**强项** (照 §138 修复已落地):
- ✅ [证据:mm:ss] 时间定位全覆盖 (8 条事件 + 3 条当庭反应)
- ✅ "整体事叙述" 段简洁有力, 浓缩事实+争议+举证+判决
- ✅ "案件基本信息" 含案件编号
- ✅ "判决结果" 明确 (有期徒刑两年 / 刑期至2020-01-23)
- ✅ 控辩双方立场客观表述 (赵立昌拒不认罪 vs 多名证人 + 现场勘查 + 伤情鉴定)

**缺失** (商业化差距):
1. **法条引用**: 没列具体《刑法》第几条 (故意伤害罪是 §234, 致人死亡是 §234 第2款) — 用户做法律场景必备
2. **控辩焦点**: 被告辩点 = 自己摔倒 / 自行下床, 控方反驳 = 证人证言 / 现场勘查 / 伤情鉴定 — 应单独列出
3. **量刑分析**: 为什么"有期徒刑两年" / 是否适用缓刑 / 量刑情节 (自首? 坦白? 取得谅解? 累犯?)
4. **证据链结构**: 谁 (证人) 证明什么 — 没结构化
5. **上诉可能性**: 二审改判空间 / 一审程序合规性

**修复方向** (下一步 §112 候选, 不主动):
- 加 "法条引用" 段到 legal_consultation.json 模板
- 加 "量刑分析 / 上诉可能性" 段 (法律场景专用)
- 庭审摘要模板独立于 `standard_meeting.json`, 走 `legal_court_session.json`

## 关联

- §132.1 (Ollama banner i18n 修复)
- §138 (摘要质量 4 项根因修复: dedup + ASR sanitize + 0-编造 + alias)
- §56 (AGENTS.md §X 描述 vs 代码)
- §37 / §92 (决策迁移铁律) / §18 (云端 API 永不接入)
- [[111-Ollama-api-chat-think-false-topic-extract修复]] (Obsidian)

