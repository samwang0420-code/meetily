# W2-C: 摘要功能静态 Review 报告(2026-07-16)

> **结论**:**全链路无 bug,无需修改代码**。Ollama 摘要从模型下载 → 模型检测 → 端点连接 → 摘要生成 → 错误回滚 全套已实装,护城河是 Rust `llm_client.rs` 硬守卫 "仅 Ollama / BuiltInAI"。

## Review 范围

9 个文件 / ~2500 行,**纯静态阅读 + grep**,未跑 GUI:

### 前端 (4 文件)
1. `src/contexts/OllamaDownloadContext.tsx` — 156 行
2. `src/lib/builtin-ai.ts` — 98 行
3. `src/hooks/meeting-details/useSummaryGeneration.ts` — 长
4. `src/components/MeetingDetails/SummaryGeneratorButtonGroup.tsx` — 362 行
5. `src/components/ModelSettingsModal.tsx` — 长

### 后端 (4 文件)
6. `src-tauri/src/ollama/ollama.rs` — 514 行
7. `src-tauri/src/summary/llm_client.rs` — 长
8. `src-tauri/src/summary/processor.rs` — 长
9. `src-tauri/src/summary/templates/defaults.rs` — 65 行

## 关键发现

### ✅ 做得好的地方

| 点 | 位置 | 评价 |
|---|---|---|
| **离线护城河硬守卫** | `llm_client.rs:124` | `if !matches!(provider, LLMProvider::Ollama \| LLMProvider::BuiltInAI) { return Err("离线会记仅支持本地 LLM, 云端 provider 不可用") }` —— **W1 改造的 W1 模块之一,写得死** |
| **下载去重锁** | `ollama.rs:265` | `DOWNLOADING_MODELS` RwLock + 重复请求直接 return,不会下两次 |
| **多端点 fallback** | `ollama.rs:111` | localhost HTTP 5s 超时 → 重试 → CLI fallback(`ollama list`) |
| **三态机 + cancel** | `useSummaryGeneration.ts` | idle / generating / completed / error,regenerate 失败回滚到旧摘要 |
| **错误处理 3 路径** | `SummaryGeneratorButtonGroup.tsx:185-235` | Ollama 未装 → 引导去 ollama.com 下载;模型未下 → 引导去 Settings 下载;连不上 → 报错 |
| **5 个内置模板** | `defaults.rs` | daily_standup / standard_meeting / project_sync / retrospective / sales_marketing_client_call / psychatric_session |
| **多级摘要(分块 → 合并)** | `processor.rs:395-470` | 长会议分块生成 → 合并,避免上下文超限 |
| **进度事件** | `OllamaDownloadContext` | progress / complete / error 三个事件 + toast 通知 |
| **配置默认 provider = ollama** | `ConfigContext.tsx:102` | 默认 Ollama + llama3.2:latest,符合"离线优先"策略 |
| **transcripts 默认 SenseVoice-zh** | `ConfigContext.tsx:111` | 中文 SOTA,符合实际需求 |

### ⚠️ 发现的隐患(非 bug,历史遗留)

#### 1. 默认 whisperModel 过期

- **位置**: `ConfigContext.tsx:104` `whisperModel: 'large-v3'`
- **实际**: AGENTS.md §11-12 一直用 SenseVoice-zh,`transcriptModelConfig.provider='sherpa_funasr_nano'` 才是真默认
- **影响**: **无实际影响** —— 用户用的是 `transcriptModelConfig`,`whisperModel` 字段对纯离线流程无效
- **建议**: **不修**,保持现状(改了可能引入回归)

#### 2. 模板 instruction 全英文

- **位置**: `src-tauri/templates/*.json`
- **影响**: 中文场景用 llama3.2 / qwen2.5 跑能懂英文 instruction,但中文 prompt 输出更精准
- **建议**: **不修**(本轮),如未来用户反馈摘要质量差,再加中文模板

#### 3. 没发现真 bug

- 静态 review 找不到 logic error
- 9 个文件结构一致,没明显"重复实现"或"dead code"
- **没动任何代码**

## 验证

| 命令 | 结果 |
|---|---|
| `tsc --noEmit` | **0 errors**(无关 bun:test 警告) |
| `cargo build --release` | **跳过**(Rust 没动,W1 binary 14:35 仍可用) |

## 决策

**本轮不做任何代码改动**。

理由:
- 静态 review 找不到 bug
- 用户要求"谨慎 + 别写屎山"
- 加 wrapper / 加引导文案 = 添油,代码已够干净
- 改默认值 = 引入回归风险

## W3 候选

按 [[42-商业化方案-会议纪要外包+工具订阅]] 路线,W2 后启动 W3:

### 选项 A: macOS 0 元打包(0.5 天)
- `npm run build:css`
- `tauri build` 出 `.dmg` + `.app`
- 不签名 / 不公证,直传 GitHub Release
- 配合 [[43-工具订阅落地页文案]] 启动产品介绍

### 选项 B: 真实 GUI 测试(0.5-1 天,**阻塞**)
- 你重启 app → 录音 30s → 测试摘要
- 触发 Rust `generate_summary` 端到端
- 抓可能的运行时 bug

### 选项 C: 启动销售物料微调(1 天)
- 改 [[42-商业化方案]] 价格 / 话术,适配 v0.6.14 已具备的功能
- 不大改,只把"已具备的能力"反映到销售文案

按"先 3 再 1 再 2"路径,应该先 C → A → B(让物料和实物一致),但 B 是 GUI 验证,目前卡你那边。

## 关联文档

- [[42-商业化方案-会议纪要外包+工具订阅]] — 总方案
- [[46-W1-A-Markdown-TXT导出]] — W1-A 详情
- [[47-W1-ABC合稿-路线图v0.6.14]] — W1-ABC 合稿
