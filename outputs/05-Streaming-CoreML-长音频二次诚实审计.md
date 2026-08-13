# LLM Streaming / CoreML / 长音频二次诚实审计（2026-07-20）

## 最终结论

上一轮“全部做到”的表述不够准确。二次逐链路审计后，真实状态如下：

| 项目 | 真实状态 | 是否可称完成 |
|---|---|---|
| Ollama LLM streaming | SSE 真 token streaming 已实现，本机 Ollama 协议实测成功 | 代码完成，仍需 App GUI 验收 |
| BuiltInAI LLM streaming | sidecar 仍在整段生成完成后才返回；当前只是把完整结果分块动画展示 | 不能称真 streaming |
| CoreML | 编译、链接和 fallback 均存在，但本机没有任何 Whisper `.mlmodelc` encoder | 不能称运行时加速已生效 |
| 一小时说话人分离 | 已有历史 3619 秒运行报告；本轮只复核报告，没有重新执行一小时原始音频 | 可确认历史失败结论，不能称本轮重跑 |

## 1. LLM Streaming 审计

### 已确认

- 最终报告阶段调用 `generate_summary_with_stream`。
- Ollama 请求发送 `stream: true`。
- 解析 OpenAI-compatible SSE：`data: {...}` 与 `data: [DONE]`。
- 后端通过 Tauri `summary-stream` 事件发送 `meeting_id` 和 `delta`。
- 前端只接收当前会议的增量，并在生成中展示。
- 本机 Ollama `qwen2.5:1.5b` 真实请求返回多行 SSE，协议假设成立。

### 未完成

- 本机 App 数据库当前 provider 是 `builtin-ai / qwen3.5:2b`。
- BuiltInAI `llama-helper` 当前协议是一请求一响应，只有生成完成后才返回整个 `text`。
- 代码随后用 `chunk_for_stream` 把完整结果拆成约 48 字符片段并立即发事件。
- 因此当前默认用户看到的是“生成完成后的快速分块动画”，不是等待期间的 token 实时输出。

### 准确说法

**Ollama 真 streaming 已实现；BuiltInAI 真 streaming 未实现。**

## 2. CoreML 审计

### 已确认

- Cargo feature tree 同时包含 `whisper-rs/coreml` 和 `whisper-rs/metal`。
- release 产物是 arm64 Mach-O。
- 构建目录存在 `libwhisper.coreml.a`。
- 二进制链接 CoreML、Metal、MetalKit、Accelerate framework。
- 构建参数包含 `WHISPER_COREML=ON` 和 `WHISPER_COREML_ALLOW_FALLBACK=1`。
- Apple Silicon 状态路由现在显示 `CoreML+Metal`。

### 未完成

- whisper.cpp 运行时会从 Whisper `.bin` 同路径寻找对应 `*-encoder.mlmodelc`。
- 本机 App 数据目录和 Documents 下没有任何 `.mlmodelc`。
- 本机也没有 Whisper `.bin`；当前主要 ASR 是 Sherpa/SenseVoice/FunASR 路线。
- 所以 CoreML 加载会失败并因 `WHISPER_COREML_ALLOW_FALLBACK=1` 回退 Metal，不会崩溃，但不会获得 CoreML encoder 加速。
- 状态标签在这种情况下仍会写 `CoreML+Metal`，属于能力编译态，不是运行时成功态。

### 准确说法

**CoreML 已编译并可回退，但运行时加速尚未生效；还缺匹配的 `.mlmodelc` 模型和真实加载日志/性能对比。**

## 3. 一小时说话人分离审计

### 已确认

历史报告包含：

- 音频长度：3619.23 秒
- 预期说话人：4
- 实际说话人：5
- 输出段数：848
- 最后一段结束：3618.34 秒
- 窗口：17
- 处理耗时：446.87 秒
- 峰值内存：360,136,704 bytes（约 343MB）
- 决策：`disable_over_300_seconds`

报告在基线提交 `cfd351a` 中首次进入 Git。本轮新增验证器只验证这些报告、原始 848 段 JSON 和运行时代码的 300 秒保护一致。

### 未完成

- 一小时原始 WAV 被 `*.wav` 忽略，目前本地不存在 `four-speaker-hour.wav`。
- 因此本轮没有重新跑 446.87 秒的推理。
- 没有音频 hash，无法仅凭仓库独立重现同一输入。

### 准确说法

**历史一小时运行证据显示生产失败，300 秒保护合理；本轮是证据复核，不是重新执行一小时验证。**

## 4. 代码安全性

- BuiltInAI 分块动画不会改变最终数据库摘要。
- CoreML 缺模型时允许 fallback，不会因此让 Whisper 初始化硬失败。
- release build 已通过。
- 当前 Git 工作区保持干净，二次审计没有修改业务代码。

## 5. 后续若要真正完成

1. BuiltInAI：修改 `llama-helper` stdout 协议，使每个 token/chunk 带 request id 输出；SidecarManager 持续读取，直到 final，再映射到现有 `summary-stream`。
2. CoreML：为实际 Whisper 模型下载/生成匹配的 `ggml-*-encoder.mlmodelc`，运行真实 Whisper 录音，检查日志出现 `Core ML model loaded`，再对比 Metal-only RTF。
3. 一小时分离：保留或重新获得原始一小时 WAV，记录 SHA-256、机器信息、命令、stdout/stderr、time -l，再重跑一次；但当前产品决策仍应保持 300 秒保护。
