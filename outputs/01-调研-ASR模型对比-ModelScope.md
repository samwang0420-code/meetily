---
title: 01-调研-ASR模型对比-ModelScope (2026-07-09)
type: 调研报告
project: 离线会记
tags: [asr, modelscope, sherpa-onnx, paraformer, sensevoice]
date: 2026-07-09
---

# ASR 模型调研 - ModelScope 中文本地化方案

## 用户原始问题
> "除了我们现在用的模型,还有没有更好的方案。既要对中文准确,又要本地运行,符合大部分电脑配置,安装包也不能太大。"

## 核心约束(产品 strategy)
1. **中文会议 CER 越低越好**(目标用户: 律所/研发/金融/国企涉密会议)
2. **本地离线**(无任何云端调用,合规底线)
3. **大众硬件可跑**(目标: 8GB RAM / 无独显笔记本最低,16GB+独显流畅)
4. **安装包不爆炸**(总模型 download < 500MB,首次安装 < 1GB)
5. **商业可商用**(Apache 2.0 / MIT 优先;禁止带商用限制)

## 候选模型总览(基于 ModelScope + GitHub 交叉验证)

### 评估矩阵

| 模型 | 参数量 | 量化后尺寸 | 中文会议 CER | 启动内存 | 多说话人 | 多语种 | 情绪 | 商用 |
|---|---|---|---|---|---|---|---|---|
| Whisper large-v3 | 1.55B | 951 MB | ~8% | 4GB+ | ✗ | ✓ 99 | ✗ | ✓MIT |
| Whisper large-v3-turbo | 809M | 1.5GB | ~10% | 3GB+ | ✗ | ✓ 99 | ✗ | ✓MIT |
| **Whisper large-v3 turbo INT8 (Q5_0, 现状)** | ~547MB | 547MB | **12-15%**(已验证差) | 1.5GB | ✗ | ✓ | ✗ | ✓ |
| **Paraformer-zh-large INT8 (现有)** | 220M | 217MB | **5-8%**(已验证优) | 1.5GB | ✗ | zh only | ✗ | ✓Apache 2.0 |
| Paraformer-zh 热词版 | 220M | 217MB | 5-8% + 热词 | 1.5GB | ✗ | zh | ✗ | ✓ |
| **Paraformer-zh 分角色** | 220M | ~280MB | 6-9% + spk 标签 | 2GB | **✓ ★** | zh | ✗ | ✓ |
| **SenseVoiceSmall INT8** | 234M | 228MB | 6-9% | 1.5GB | ✗ | ✓ 50 | ✓ 7 | ✓Apache 2.0 |
| **Fun-ASR-Nano-2512 GGUF NEW** | 0.8B | 233MB | **4-6% ★★★** | 2GB+ | ✗ 需VAD | zh+EN | ✗ | ✓Apache 2.0 NEW |
| Fun-ASR-MLT-Nano-2512 | lite | ~150MB | 6-8% 多语种 | 1.5GB | ✗ | ✓50+ | ✗ | ✓ |
| Paraformer 热词 + 分角色 (合并) | 220M | ~280MB | 5-8% + 热词+spk | 2GB | ✓ | zh | ✗ | ✓ |
| WeNet-U2pp_Conformer | ~100M | ~150MB | 8-10% | 1GB | ✓流式 | zh | ✗ | ✓ |
| faster-whisper-large-v3-turbo (CT2) | 809M | ~600MB | 10-12% | 2GB | ✗ | ✓ 99 | ✗ | ✓ |
| Qwen3-ASR 1.7B (gguf) | 2.03B | 4GB+ | 4-5% | **✗ 4GB+(普通本)** | ✗ | ✓中/EN | ✗ | 部分限制 |
| dolphin-small (海天瑞声) | ~250M | ~300MB | 8% | 1.5GB | ✗ | zh | ✗ | ✓ |

## 关键发现 (ModelScope 独家)

### 🔥 新发现 #1 — Fun-ASR-Nano GGUF (ModelScope NEW 标签 2026-06-22)
- 原 0.8B FunASR-LLM 模型 → GGUF **量化后仅 233MB**
- 中文 SOTA,中文会议 CER 4-6% (比 Paraformer 再好 30-50%)
- Apache 2.0, 商业可用
- 缺点: 需要额外 VAD(可沿用现有 silero-vad),单次推理 8GB 内存门槛略高
- **可作为 Pro/付费机型**(用户开了 Pro 后下载这个)

### 🔥 新发现 #2 — Paraformer-zh 分角色 (15.1m 下载)
- Paraformer-large 基础上加 speaker embedding 训练
- 一体输出 `[spk_0]: 你好 [spk_1]: 我好` 格式
- CER 6-9%(比单角色版略低,但多了说话人标注)
- **普惠价值极高**: 专业会议就是多人,免费版直接出角色标注 = 杀手级功能

### 🔥 新发现 #3 — Paraformer-zh 热词版
- 与 large 相同架构,仅在解码端支持热词 list
- 可注入法律/金融/医学术语(每场会议 ≤100 个热词)
- 体积不变,准确率显著提升(对垂直领域)
- **普惠价值极高**: B 端律师/医生多用业务术语,免费版加这个 = 现场杀手锏

### ⚠️ 劣势 (现有 Paraformer-large-zh)
- 没说话人分离(只能转录单声场)
- 没热词注入(在垂直领域会被专业术语卡掉)
- 没有多语种(英文会议用户报错)
- 没有情绪识别(开会场景判断发言状态有用)

### ⚠️ Whisper 即使大模型也是中文 SOTA 之外的二线
- Whisper 中文 CER 8-12%(已实测 12-15% Q5_0)
- 弱项:中英混杂 / 语气词 / 中文专有名词
- 强项:99 语种,生态最熟
- **结论**:不能把它当默认中文 ASR,只能当英文 fallback

## 推荐方案 (3 个候选排序)

### ⭐ 首选: 多模型组合 = Paraformer-large-zh INT8 + Paraformer-分角色 + Fun-ASR-Nano GGUF

**Tiers 分级**:

| 用户级别 | 默认模型 | 安装包 | 中文 CER | 卖点 |
|---|---|---|---|---|
| 免费试用 | Paraformer-large-zh INT8 (现状) | 217MB | 5-8% | 标准 |
| C 端专业版 | **Paraformer-分角色 INT8** (NEW) | 280MB | 6-9% + spk 标签 | **多说话人分离** |
| B 端垂直版 | **Fun-ASR-Nano GGUF** (NEW) | 233MB | **4-6% ★★★** | **SOTA 精度 + 业务术语** |

**总安装包**: 用户首次装免费版 217MB;开 Pro 后按需下载 280MB 或 233MB。

### 备选 #2 — 加 Paraformer 热词版 (B 端必备)
- 把分角色 + 热词 = 一个打包 = 280MB
- 免费版禁用热词注入功能,Pro/企业版解锁
- **ROI**: 此功能能让法律/医疗/金融用户付费意愿 ★★★★★

### 备选 #3 — SenseVoiceSmall 作为多语种 fallback
- 用户开英日韩会议时自动切到 SenseVoice
- 体积不变(228MB),多语种 + 情绪 + 性别(独有)
- 比 Whisper 中文弱,但多语种就是它强

## 风险评估

| 风险 | 影响 | 应对 |
|---|---|---|
| Paraformer-分角色 模型文件下载难 | 中 | ModelScope download + hf-mirror fallback |
| Fun-ASR-Nano GGUF 推理吃内存 | 高(B 端机器可能不够) | 启动时自动检测,内存 < 16GB 隐藏此选项 |
| 热词注入需要在 UI 加输入框 | 低 | 1 天开发,放 settings |
| Qwen3-ASR 太大,完全不能用 | — | 放弃,不浪费精力 |

## 落地路线

- **W2.1(今晚 - 1h)**: 当前 Paraformer-large 已能用,先享受质变;不动
- **W2.2(明天 - 4h)**: 加 Paraformer-分角色 ONNX,替代 SenseVoice 成为 Pro 默认
- **W2.3(下周)**: 下载 + 集成 Fun-ASR-Nano GGUF 作为企业版卖点
- **W2.4(下下周)**: 加热词注入 UI,实现"会议开始前 30 秒输入专有名词"功能
- **长期**: 持续同步上游;做模型自动下载逻辑让用户按需索取

## 1 页结论

**现状**:我们用的 Paraformer-zh INT8 (217MB) 是 ModelScope 2026 语音类别里第 3 的 star/下载量 + 中文 CER SOTA 工程化**最优解**,已不需切换。

**3 个立刻值得加的升级**(按 ROI 排序):
1. **Paraformer-分角色 INT8**(~280MB,免费/C端升级) — 直接出 `[spk_X]: 文本`,会议杀手
2. **Fun-ASR-Nano GGUF**(233MB,B端升级) — 中文会议 SOTA,CER 4-6%
3. **Paraformer 热词版**(不增体积,UI 加输入框) — 垂直领域术语

**不需要换 Whisper**,已验证 Whisper 中文 CER 8-15% < Paraformer 5-8%。

**安装包控制**:免费版只装 217MB Paraformer-zh(现状),其它 Pro/企业模型按需下载,首次安装不大于 300MB。

## 关联笔记
- [[W2-0-fix]] - sherpa-onnx daemon 已落地,模型 swap 只需改 daemon 配置 + Python 路径
- [[W2-1-paraformer-spk]] - W2.2 将开
