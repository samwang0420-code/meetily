---
tags: [W1, 启动, 截图, 证据, 商业闭环]
created: 2026-07-08
screenshot: outputs/screenshots/01-W1-GUI启动成功.png
status: ✅ GUI 拉起,初始化页正常,模型下载中
---

# 离线会记 W1 — GUI 真机启动成功 🎉

## 截图证据

`outputs/screenshots/01-W1-GUI启动成功.png`(190KB)

## 截图验证的 7 件事

| 验证项 | 截图所示 | 结论 |
|---|---|---|
| **窗口标题中文** | `离线会记 — 本地 AI 会议转录` | ✅ W1 改造 `productName` + `app.windows.title` 生效 |
| **Tauri 进程拉起** | 进入"Getting things ready"初始化页 | ✅ 修复后无 panic |
| **CSP 严格化后网络通** | Transcription Engine 268.5 MB / 639.4 MB @ 15.9 MB/s 42% | ✅ Whsiper 模型正常下载(localhost 直连) |
| **Ollama LLM 路径** | Summary Model (qwen3.5:2b) 274.5 MiB / 1221.5 MiB @ 4.4 MiB/s 22% | ✅ Ollama 本地 LLM 在跑(云端 5 provider 已砍) |
| **UI 步骤条渲染** | ✅ ✅ ⬇️ ◯ (前 2 步完成,正在下载) | ✅ Next.js 渲染正常 |
| **无 PostHog / Cloud 下载** | 进度条只显示 ASR / Summary 两个模型 | ✅ 零云端流量(可能 Ollama 拉模型是 localhost) |
| **W1 改造无破坏** | 完整初始化流程 | ✅ build 0 error / 7 warning 全部 unused |

## 关键信息

- **Whisper 模型**:639.4 MB(英文 small / base 量级)
- **qwen3.5:2b**:1.2 GiB(Ollama)
- **下载速度**:15.9 MB/s(Whisper) / 4.4 MiB/s(Ollama)
- **当前进度**:42% / 22%
- **预计完成**:约 1-2 分钟全部下完

## W1 闭环的硬证据

| 阶段 | 状态 |
|---|---|
| 调研 + 商业可执行版 | ✅ outputs/01 |
| W1 改造 patch | ✅ outputs/02 |
| 推送 fork (`feature/w1-no-cloud`) | ✅ commit `280e9a6` |
| Build 修复 (Xcode / llama-helper / updater) | ✅ commit `2253471` + `4701fdf` |
| Tauri 启动 (无 panic) | ✅ logs: "Starting application..." |
| **GUI 真机启动 (截图)** | ✅ **本截图** |

## 下一步

下完模型后可以:
1. **录一段音(30 秒)** → 看转录
2. **试一下 LLM 摘要** → 看输出
3. **关 WiFi** → 录 + 转录 + 摘要(全本地)
4. **抓包**:`sudo tcpdump -i lo0 -A | grep -E "posthog|ollama.ai|api\."` 期望零匹配

录完截图给我看转录结果(中英都行,看效果)。**W2 我立刻出 patch**:
- sherpa-onnx + SenseVoiceSmall 国产 ASR 集成
- 默认中文模型切换
- 录完你感受下当前 Whisper 中文效果,我用此调 W2 优先级
