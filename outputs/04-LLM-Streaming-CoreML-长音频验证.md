# LLM Streaming + CoreML 路由 + 一小时说话人分离验证（2026-07-20）

## 结论

三项均已完成代码与静态/构建验证，并拆成三个独立 Git 提交推送到 `feature/w1-no-cloud`。

- LLM streaming：已接通。Ollama 使用 OpenAI-compatible SSE 真 token streaming；BuiltInAI sidecar 仍一次性返回，但通过安全 UTF-8 分块适配为统一增量事件。最终摘要仍以数据库 completed 结果为准，不改变取消、失败恢复和 BlockNote 保存逻辑。
- CoreML：已确认 macOS 依赖本来就同时编译 `whisper-rs/metal` 与 `whisper-rs/coreml`。本轮修复的是路由可见性：Apple Silicon 现在明确报告 `CoreML+Metal`，Intel Mac 继续报告 Metal，不再误报为仅 Metal。
- 一小时说话人分离：生产验证结论为不通过。3619.23 秒四人音频被识别成 5 人，848 段，17 个窗口，耗时 446.87 秒，峰值内存 343MB。因此保持超过 300 秒禁用 cam++ 的保护，不向用户宣传一小时可用。

## Git 提交

- `12f8289 feat(summary): stream local LLM output`
- `4a5c40a feat(asr): expose CoreML Metal routing`
- `e3ba70a test(diarization): verify one-hour production evidence`

远端：`origin/feature/w1-no-cloud`，当前 HEAD `e3ba70a`。

## LLM Streaming 实现

### 后端

- 新增统一 `StreamSink` 回调，不让 LLM client 直接依赖 Tauri。
- 最终报告阶段才启用 streaming；分块摘要、合并摘要、翻译仍为非流式，避免 UI 展示中间账本。
- Ollama 请求增加 `stream=true`，解析 `data: {...}` / `[DONE]` SSE 行。
- BuiltInAI 对最终整段结果做 UTF-8 安全分块，统一发送增量事件。
- Service 发 `summary-stream` 事件，payload：`meeting_id` + `delta`。

### 前端

- `SummaryPanel` 只监听当前会议的 `summary-stream`。
- 生成中实时显示只读 Markdown 草稿和本地模型脉冲状态。
- completed 后仍由现有 polling/数据库结果切换到 BlockNote，不以流式草稿覆盖正式结果。

### 边界

- Ollama 是真 token streaming。
- BuiltInAI 当前 sidecar 协议是一请求一响应，因此本轮是完成后快速增量展示，不是 sidecar token 级实时输出。若以后要真 token streaming，必须改 `llama-helper` stdout 协议和 SidecarManager 多消息关联，风险更高。

## CoreML 路由

- Apple Silicon：`CoreML encoder + Metal GPU`。
- Intel macOS：Metal。
- CUDA/Vulkan/HIP/CPU 路由不变。
- 新增 5 个 acceleration 专项测试，其中 CoreML 路由测试确认 GPU、Flash Attention 和状态标签。

## 一小时分离验证

验证器：`frontend/benchmarks/diarization/verify_long_audio.mjs`

通过 9 项证据检查：

- 音频长度 ≥ 3600 秒
- 预期说话人数 4
- 实际人数与原始报告一致且错误为 5
- 原始 848 段完整
- 耗时和内存记录有效
- release 决策为 `disable_over_300_seconds`
- 运行时常量为 300 秒
- 超长音频返回 `audio_too_long`

短音频固定基准仍为 2/2 全过，speaker purity 100%。

## 验证结果

- Next.js production build：通过，19 个静态页面。
- Rust streaming 单测：2/2。
- Rust CoreML/acceleration 单测：5/5。
- Python：3/3。
- 短音频 diarization：2/2，平均 purity 100%。
- 一小时证据复核：9/9。
- `cargo build --release`：通过，耗时 9 分 35 秒，仅既有 17 个 warning。
- 全量自测：U、C/D、商业化全过；T 系列仍为基线原有 20 条旧断言失败，没有新增失败。

## 仍需用户 GUI 验收

### LLM Streaming

1. 重启真实 App。
2. 打开已有会议，不需重新录音。
3. 点击重新生成摘要。
4. 确认生成过程中出现逐步增长的 Markdown 草稿。
5. 确认 completed 后切换为正式 BlockNote 摘要，内容不丢失、取消仍有效。

### CoreML

本轮改动涉及 Whisper/ASR 路由。按项目铁律，release binary 真机启用后仍需一次 30 秒中文录音，确认数据库新增段数 ≥ 1，才能认定 GUI 生产验收完成。

### 长音频

不需要用户再录一小时。已有 3619 秒真实运行证据已足够，结论是当前不可上线，300 秒保护继续保留。
