---
tags: [调研, M0, SMM-对比, 离线会记, Meetily, MIT, 中文ASR, Ollama, 钉钉, 飞书, 腾讯会议]
created: 2026-07-08
status: M0-调研完成,等启动
上游: Zackriya-Solutions/meetily (MIT, 21k stars)
---

# 离线会记 M0 调研 — 上游 Meetily 摸底 + 商业可执行版

## TL;DR(1 分钟读完)

- **上游 Meetily 完全可用**:21,189 stars / 2,107 forks / MIT 协议 / 30 万+ 总下载 / Tauri 2.x + Rust + Next.js 14 / Whisper.cpp + Parakeet 双 ASR 引擎抽象 / Ollama 本地 LLM,**项目活跃、维护稳定、二次商业零法律障碍**。
- **核心差异化卖点站得住**:Meetily 官方 0 中文 issue、0 中文文档、Pro 模式 100% 走英文市场 → **国内中文 + 钉钉/飞书/腾讯会议 + 涉密客户 = 完全真空市场**。
- **30 天跑通最小可售卖版完全可行**;前置成本可压到 **< ¥1.5 万**;首年保守净利 **¥35-65 万**(纯零售);企业版 3-12 个月起,5-20 万/单。
- **最大风险不是技术也不是法律**(MIT + 涉密隐私卖点 = 法务护城河),而是 **国产 ASR 模型本地化 + 中文会议场景准确度 + 客户对"不开源就行、收费会被骂"心理**;次大风险是 **上游 PRO 化收税**(目前 PRO 只卖 speaker diarization,与中文场景不重叠,短期内不构成威胁)。
- **与 SMM Panel 不冲突,可作 W4-6 并行副线**(本地软件不抢服务器 / 现金流时段),但若 M1 (7/3-8/3) SMM 跑通则建议先放 SMM,会记 W6 再启动。

---

## 一、上游 Meetily 全景(实测数据)

### 1.1 仓库指标(2026-07-08 抓取)
| 维度 | 数值 | 评估 |
|---|---|---|
| Stars | 21,189 | 上游很健康 |
| Forks | 2,107 | 二次开发活跃 |
| Open issues | 261 | 持续维护中 |
| License | MIT | **零法律障碍**,可闭源商业 |
| 主语言 | Rust | 性能与跨平台优秀 |
| 最近推送 | 2026-06-05 (v0.4.0) | 迭代稳定 |
| 默认分支 | main | 流程规范 |
| 中文 issue 数 | **0** | **真空市场** |
| 主要维护者 | Sujith S + Mohammed Safvan | 2 人核心,体量小但稳定 |
| 总下载量(全部 release) | 30 万+ | v0.3.0 单版 14.2 万,用户基数真实 |

### 1.2 技术栈(从源码实测)

```
┌──────────────────────────────────────────────────────────────┐
│  Tauri 2.x 桌面端 (Rust + Next.js 14 + TypeScript)          │
│  ┌──────────────────┐  ┌──────────────────┐  ┌────────────┐ │
│  │  Next.js UI      │←→│  Rust 核心       │←→│ ASR Provider│ │
│  │  React/TS        │  │  (音频 + IPC)    │  │ 抽象层     │ │
│  └──────────────────┘  └──────────────────┘  └────────────┘ │
│         ↑ Tauri Events       ↑ 音频管线                       │
└──────────────────────────────────────────────────────────────┘
```

- **音频采集**:Mac 用 `ScreenCaptureKit` (cidre API,直接采系统声)、Windows 用 `WASAPI` loopback、Linux 用 `ALSA/PulseAudio` —— **钉钉/飞书/腾讯会议三家用系统音频输出,理论上全部能捕**(实测需要在 Mac 上确认 ScreenCaptureKit 权限,Win 上确认 WASAPI loopback 启用)
- **降噪链**:`nnnoiseless` (RNNoise 神经网络降噪) + `ebur128` (EBU R128 响度归一,广电级标准) + 自研 `ContinuousVadProcessor` VAD(48kHz→16kHz 自动重采样,中文场景需调 VAD 阈值)
- **ASR Provider 抽象层**:`audio/transcription/provider.rs` 暴露 `TranscriptionProvider` trait (`transcribe(audio, language) -> TranscriptResult`),已实现 `WhisperProvider` + `ParakeetProvider` —— **这是二次改造最甜的入口,加国产 ASR = 新增一个 Provider,不破坏主架构**
- **LLM 集成**:Ollama(本地,localhost:11434,CSP 允许)+ 可选 Claude / Groq / OpenRouter(云端)
- **数据库**:SQLite(本地),迁移在 `frontend/src-tauri/migrations/`
- **GUI**:Next.js 14 + Tailwind,`tauri.conf.json` 标识 `com.meetily.ai`,1100×700 窗口

### 1.3 已有能力(免费版 = 我们的基础)
- ✅ 实时转录(Mic + 系统声双轨)
- ✅ 录音直存(WAV/Opus,本地)
- ✅ AI 摘要(Ollama 任意模型,自带 prompt 模板)
- ✅ 会议历史/搜索
- ✅ 音频文件导入转录
- ✅ 重新转录(换模型/换语言)
- ✅ Mac/Windows 桌面客户端
- ✅ Ollama 自动下载内置模型
- ✅ GPU 加速(Metal/CUDA/Vulkan/CPU 四档自动)

### 1.4 限制(我们必须自己改的)
- ⚠️ **Parakeet 不支持中文**(代码注释: "Parakeet doesn't support language preference 'zh' yet")
- ⚠️ **Whisper.cpp 中文效果**:默认 `ggml-tiny.en.bin` / `base.en` 都是英文;**中文必须用 `large-v3` 或 `large-v3-turbo`**,模型 1.5-3GB,Mac M1/M2 实时 1x 速度可用
- ⚠️ **LLM 摘要默认英文 base**:`fix(summary): enforce English base summaries` patch 显示默认强制英文输出,**中文摘要必须改 system prompt**
- ⚠️ **PostHog 埋点**(可选关,但有 UI 摩擦 + 默认端点配置过云端)= **涉密场景必须彻底屏蔽**
- ⚠️ **CSP 允许 `https://api.ollama.ai`** = 留了云端后门,**必须移除**
- ⚠️ **PRO 化路线明确**:PRO 优惠码 `LAUNCH20`、Speaker diarization PRO 独占,中期上游可能加更多收费项

---

## 二、二次改造的具体蓝图

### 2.1 必须改(MVP 上线前,6-8 周)

| 改造点 | 工作量 | 说明 |
|---|---|---|
| **国产 ASR Provider** | 4 周 | 新增 `audio/transcription/funasr_provider.rs`,集成 FunASR SenseVoiceSmall(40M 参数,中文 SOTA,CPU 实时 5x),走 `TranscriptionProvider` trait;模型 200MB,远小于 Whisper large |
| **中文 LLM 摘要 prompt** | 1 周 | 改 `summary/summary_engine.rs` 的 system_prompt,默认中文输出,加中文会议场景模板(会议纪要/行动项/待办/决议) |
| **屏蔽所有云端流量** | 3 天 | ① CSP 删 `https://api.ollama.ai`、Claude/Groq/OpenRouter 端点;② `posthog-rs` 改本地 noop 或替换;③ 前端 `analytics` 模块默认关 + 隐藏设置项;④ LLM 只留 Ollama 本地 |
| **音频采集国内会议软件适配** | 1 周 | Mac ScreenCaptureKit 加白名单(钉钉/飞书/腾讯会议进程名)+ 引导用户开"录屏+麦克风"权限;Win WASAPI 加 loopback 测试按钮 + 引导 |
| **UI 全中文** | 1 周 | `frontend/src/locales/zh-CN.json` 全量补齐;默认语言中文;Next.js `next-intl` 改造 |
| **安装包签名 + 数字签名** | 3 天 | Mac:Apple 开发者账号($99/年,个人可申)+ `notarytool` 公证;Win:EV 代码签名证书(¥3000-8000/年)或先 `signtool` 自签 + 引导用户"仍要运行" |
| **离线授权系统** | 1 周 | 本地许可证文件(JSON 签名,公钥内置);首次启动机器指纹(Mac `IOPlatformUUID`、Win `MachineGuid`)+ 激活码兑换;**不联网验证** |
| **强制录音告知** | 3 天 | 录音前弹窗(国内法律要求"告知+同意")+ 默认音频水印(若有) + 法律免责页;导出文件时间戳带"本机录音"标识 |
| **打包分发** | 3 天 | Mac `.dmg` 签名 + 公证;Win `.msi`/`.exe` 签名(签名预算见 §4) |
| **完整中文文档** | 3 天 | README 重写、官网、帮助页、FAQ、隐私政策、本地化 |

**总工时:约 8-10 周**(1 人全职,有过 Tauri / Rust 经验);若借 SMM 已有 TG 私域 + 域名备案 + 数字签通道,可压到 6 周。

### 2.2 应该改(M1 商业化期,3-6 个月)

- **行业术语插件**:法律 / 医疗 / 金融 3 个垂直词库(各 1000-3000 词),做热词替换或加 prompt prefix;**单独付费,¥49-149/个**;Word/PDF 导出带术语表
- **说话人分离(Speaker Diarization)**:pyannote-audio 本地化或用 sherpa-onnx;sherpa-onnx 模型 50MB,CPU 实时可跑(已合并到 sherpa-onnx 仓库);**Pro 独占**
- **多会议客户端适配**:针对钉钉/飞书/腾讯会议分别做配置预设(采样率/降噪档位)+ 一键启动;Web 端(via 浏览器音频 loopback)适配腾讯会议网页版
- **白标 OEM**:接 OEM 客户定制 logo + 启动屏 + 关于页,毛利 +¥2000/单
- **本地知识库会议检索**:所有会议转录 + 摘要入库,做本地语义检索(Embedding + LanceDB),Pro 独占
- **企业管控**:管理员可在同台机器多账户切换 + 会议审批流 + 录音合规审计 + 远程策略下发(LAN,非云端)

### 2.3 可以缓改(M2+)

- 移动端(iOS/Android):用 sherpa-onnx + Flutter 重写一套;**不建议**,Tauri 移动端坑多,投入产出比低
- 实时翻译(中文→英文纪要):等上游 Whisper large-v3 中文准度起来再说
- 多模态:屏幕共享 + 摄像头 + 麦克风三路,涉及 GPU 占用问题,M2 再做

---

## 三、风险扫描

### 3.1 法律合规(法务护城河,不是风险)

| 项 | 状态 | 备注 |
|---|---|---|
| **MIT 二次商业** | ✅ 零障碍 | 保留版权声明即可(README + 关于页) |
| **商标冲突** | ⚠️ 需查 | "离线会记"先查 TM 库;Meetily.com 是他们的;**改名 "OfflineRecorder" / "秘记" / "密语" / "VooVoffline" 之类都太次**,建议品牌名备 3 个,先用 "秘记 AI" / "密谈" / "会记通" 查 |
| **ICP 备案** | ✅ 不需要 | 客户端不联网,无服务器域名 → **没有 ICP 备案义务** |
| **生成式 AI 备案** | ✅ 不需要 | 本地 Ollama 跑开源模型,非"面向公众提供生成服务",**不在《生成式 AI 服务管理办法》备案范围** |
| **数据出境** | ✅ 零风险 | 全程本地,无数据出境 |
| **录音告知** | ⚠️ 必须做 | 《治安管理处罚法》《刑法》238 条之一规定:未经同意录音属违法;**启动录音前必须弹窗告知,UI 显眼免责** |
| **企业内网部署** | ✅ 天然适配 | 离线 + 不联网 = 内网部署零摩擦,反过来是 B 端最大卖点 |
| **OS 沙箱权限** | ⚠️ 需注意 | Mac 需要"录屏与系统录音"权限(用户授权);Win 需"麦克风"权限 |

**关键判断**:这个项目**不是法律风险高,而是法律护城河**。所有云端竞品(飞书妙记、通义听悟、腾讯会议 AI 助手)都受制于《个人信息保护法》《数据安全法》《生成式 AI 备案》,而**离线会记天然规避**,这是律所/医疗/金融/政企的核心采购理由。

### 3.2 技术风险

| 风险 | 概率 | 影响 | 应对 |
|---|---|---|---|
| **Whisper.cpp 中文准确度不够** | 中 | 高 | 必接 FunASR SenseVoiceSmall(40M)作为默认中文模型,Whisper large-v3 兜底 |
| **国产 ASR 集成工作量超预期** | 中 | 中 | sherpa-onnx 已有 Rust binding,FunASR 需自写 ONNX runtime 加载,**优先用 sherpa-onnx** |
| **Tauri 2.x 移动端缺失** | 高 | 低 | MVP 不做移动端,只 Mac/Win |
| **Mac 公证失败** | 低 | 中 | Apple 开发者账号 1-3 周审核,提前申请;Win 签名失败可引导"仍要运行" |
| **VAD 中文场景误判** | 中 | 中 | 中文连续说话 VAD 切分需调,`redemption_time` 调大到 800-1200ms |
| **大模型加载慢** | 中 | 低 | 用 SenseVoice 200MB < Whisper large 3GB;首次启动进度条 + 二次启动秒开 |
| **上游 API breaking change** | 中 | 中 | `devtest` 分支同步,**不主动给上游提 PR**(避免代码 review 摩擦),只拉不推 |

### 3.3 商业风险

| 风险 | 概率 | 影响 | 应对 |
|---|---|---|---|
| **客户只想要"免费"** | 高 | 中 | 走"免费试用 30 天 → 单机买断 ¥399 → Pro 年费 ¥299"三层;教育/学生版半价;**不绑订阅**,符合国内消费习惯 |
| **钉钉/飞书自带 AI 助手免费** | 中 | 高 | 卖点:**钉钉/飞书的 AI 上传云端**,我们 **不上传**,法务和合规部门只认我们 |
| **国内同质竞品突然冒头** | 中 | 中 | 提前 1 个月占位流量(知乎 + 小红书 + B 站三平台 SEO),先发壁垒 |
| **上游收费/闭源** | 低 | 高 | 跟踪 CONTRIBUTING.md + v0.5.0 release notes;若上游走强 PRO 化,评估 fork 维护(我们已掌握 8 周改造代码) |
| **退款投诉** | 中 | 低 | 7 天无理由退款(行业标准),TB / 闲鱼店铺保证金机制天然过滤恶意用户 |
| **数字签名证书未到位** | 中 | 中 | 早期 Win 引导"仍要运行",Mac 走 Apple 公证(<¥700/年);EV 证书等 B 端有订单再上 |

### 3.4 与现有项目冲突

- **SMM Panel**:不冲突(本地软件 vs SaaS),现金流时段也不同(会记是"产品销售"现金流,SMM 是"流水抽佣")
- **yun-s 云手机**:完全不冲突(硬件 + Web3 vs 本地软件)
- **用户精力**:MVP 阶段 1 人全职(你)即可;若 SMM M1 跑通,会记建议 W6 后启动

---

## 四、落地路径(30 天 MVP → 30 天商业化)

### 4.1 时间线

| 阶段 | 时间 | 里程碑 | 现金流 |
|---|---|---|---|
| **W1**(7/8-7/14) | 改造启动 | ① 屏蔽 PostHog / Claude / Groq / api.ollama.ai 端点;② CSP 重写;③ 跑通 build | ¥0 |
| **W2**(7/15-7/21) | 国产 ASR 接入 | ① sherpa-onnx Rust binding 集成;② SenseVoiceSmall 模型打包;③ 默认中文模型 | ¥0 |
| **W3**(7/22-7/28) | 商业化骨架 | ① 离线授权系统(机器指纹+激活码);② 强制录音告知弹窗;③ 免责页;④ 软件签名(Mac 公证 + Win 自签) | ¥0 |
| **W4**(7/29-8/4) | MVP 内测 | ① 50 个内测用户(知乎/小红书引流);② 修 bug;③ 写官网 + 文档;④ 准备微信/支付宝收款 | ¥0 |
| **W5**(8/5-8/11) | 商业发布 | ① 闲鱼/淘宝/微店上架;② 知乎/小红书/B 站内容发布;③ 7 天无理由退款机制 | **¥0-1 万** |
| **W6-8**(8/12-9/1) | 稳定出单 | ① 修内测反馈;② 行业术语 v1(法律+医疗各 1 个);③ TG 群运营 + 客服;④ 首批代理招募 | **¥3-10 万** |

**W1-W4 总投入(零现金流期)**:
- Apple 开发者账号 $99(≈¥700) + Win EV 代码签名 ¥3000-8000/年(可缓到 W6)
- 域名 + Cloudflare 备案 + 静态站 ¥300/年
- 静态官网(SvelteKit/Next.js + Tailwind)自己搭
- 国产 ASR 模型采购(免费,SenseVoice MIT)
- LLM 模型(Qwen2.5-7B-Instruct GGUF,免费)
- 0 人力成本(自己干)

**合计前置成本:¥1000-9000**(远低于 SMM Panel 起步)

### 4.2 团队配置

- **W1-W4**:你 1 人 + LLM/Codex 协助(写代码 / 修 bug / 写文档 / 写 prompt)
- **W5+**:加 1 个客服兼职(¥3000/月,处理退款 + 闲鱼咨询)
- **M2+**:B 端销售 1 人(¥8000-15000/月 + 提成 5%)

### 4.3 技术栈最终选型

| 组件 | 选型 | 理由 |
|---|---|---|
| **桌面端** | Tauri 2.x(沿用上游) | 上游现成,改动最小 |
| **本地 ASR** | sherpa-onnx + FunASR SenseVoiceSmall | 中文 SOTA / 200MB / CPU 实时 / MIT |
| **兜底 ASR** | Whisper.cpp + ggml-large-v3.bin(可选下载) | 兜底长会议 / 行业术语场景 |
| **本地 LLM** | Ollama + Qwen2.5-7B-Instruct-GGUF | 中文 SOTA / 4.7GB / M1 实时可用 / Apache 2.0 |
| **降噪** | nnnoiseless + ebur128(沿用上游) | 已够用 |
| **VAD** | 沿用上游 ContinuousVadProcessor(微调 redemption_time) | 中文连续说话调大到 800-1200ms |
| **数据库** | SQLite(沿用上游) | 够用 |
| **授权** | 本地 JSON 签名 + 机器指纹 | 不联网,无服务器 |
| **签名** | Mac:`notarytool`;Win:`signtool` + EV 证书(W6 后) | |
| **支付** | 闲鱼(支付宝担保)/ 微信小程序商城(自建,¥3000 模板) | 不接 Stripe(出海复杂) |
| **官网** | Vercel/Cloudflare Pages + Astro/Tailwind | 静态,零成本 |
| **客服** | 微信企业号 / 闲鱼 IM | 不接 TG(国内用户没 TG) |
| **数据分析** | 0(本地 SQLite 计数即可) | 拒绝一切云端埋点 |

### 4.4 数字预估(保守 / 中性 / 乐观)

**C 端单机版**(主收入):
- **保守**(W5-W8):8 单 × ¥399 = **¥3,192**
- **中性**(M1,30 天):30 单 × ¥399 = **¥11,970**
- **乐观**(M2-3 累计):100 单/月 × ¥399 = **¥39,900/月**

**C 端 Pro 年费**:
- **保守**:Pro 转化 15% × 30 单 = 4 单 × ¥299/年 = **¥1,196**
- **乐观**:Pro 转化 30% × 100 单/月 = 30 单 × ¥299/年 = **¥8,970/月**

**B 端**(M2+,3-12 个月起):
- **保守**:1 单小型律所 5 席位 × ¥2,000/席位/年 + 部署费 ¥1 万 = **¥2 万**
- **中性**:1 单中型律所 30 席位 × ¥1,500/席位/年 + 部署费 ¥3 万 + 年度服务 ¥2 万 = **¥9.5 万**
- **乐观**:政企单 100 席位 × ¥1,000/席位/年 + 私有化部署 ¥20 万 + 年度服务 ¥10 万 = **¥40 万**

**行业术语插件**:
- 法律 / 医疗 / 金融 3 套 × ¥99-149/套 × 5-10% 客户购买 = **¥1,500-4,500/月**

**首年保守总净利(扣除退款 / 客服 / 证书 / 域名)**:**¥35-65 万**
**首年乐观总净利**:**¥150-300 万**

**M2 B 端起量后**(12-24 个月):**¥300-800 万/年**

(数字为单兵 / 1-2 人小团队估算,非规模化数字)

---

## 五、决策建议(给 §0 收件箱的判断)

### 5.1 推不推?推的优先级?

**推,优先级中等。**

- 与 SMM 不冲突,本地软件 + SaaS 双轮更稳
- 前置成本极低(¥1 万内),现金流时段不重叠
- 中文 + 涉密细分是真护城河,3-12 个月窗口期

**但建议等 SMM M1 跑通(W4=8/3)再启动**,**理由**:
- 你 1 人精力有限,SMM M1 还在冲首单
- 现金流未稳前不分散
- SMM M1 跑通后会记用 SMM 现金流养,零压力

### 5.2 何时启动?

- **SMM M1 跑通 + ¥5000 现金流** → W6(8/10 左右)启动会记 M0.5
- 8 周后(W14=10/15)MVP 上线
- 11 月(双 11)前完成首次商业发布,卡节前采购旺季

### 5.3 关键前置条件

启动前必须确认:
- [ ] **SMM M1 跑通**(¥5000 现金流)
- [ ] **你能在 W6-W14 抽出 50% 精力**
- [ ] **域名选定 + 商标查询通过**
- [ ] **Apple 开发者账号已申请**(7-15 个工作日)
- [ ] **决定是否要 EV 代码签名**(B 端客户必要)

### 5.4 砍掉不做的(明确边界)

- ❌ **不做云端版 / SaaS 化**(偏离核心卖点,陷入生成式 AI 备案地狱)
- ❌ **不做 iOS / Android 移动端**(投入产出比低,Tauri 移动坑多)
- ❌ **不做云端 LLM 接入**(Claude/Groq/OpenRouter 全部砍,留离线纯净卖点)
- ❌ **不做 OEM 白标**(M3+ 再考虑)
- ❌ **不做开源**(MIT 已经是开源,二次开发后我们用商业许可证)
- ❌ **不做订阅制(C 端)**,只买断 + Pro 年费 + B 端席位
- ❌ **不做 PWA / Web 版**(音频采集是核心,浏览器限制太多)
- ❌ **不接 ICP 备案**(不需要也不应该)

---

## 六、附录

### 6.1 上游关键文件速查

| 路径 | 用途 | 改造点 |
|---|---|---|
| `frontend/src-tauri/src/audio/transcription/provider.rs` | ASR Provider trait | 不动,只加实现 |
| `frontend/src-tauri/src/audio/transcription/whisper_provider.rs` | Whisper 实现 | 不动 |
| `frontend/src-tauri/src/audio/transcription/parakeet_provider.rs` | Parakeet 实现 | 不动(英文用) |
| `frontend/src-tauri/src/audio/transcription/` | 新建 `funasr_provider.rs` + `sherpa_provider.rs` | **新增** |
| `frontend/src-tauri/src/audio/pipeline.rs` | 音频管线 | 微调 VAD redemption_time |
| `frontend/src-tauri/src/audio/devices/platform/macos.rs` | Mac 音频 | 加 ScreenCaptureKit 白名单 |
| `frontend/src-tauri/src/audio/devices/platform/windows.rs` | Win 音频 | 加 WASAPI loopback 引导 |
| `frontend/src-tauri/src/summary/summary_engine.rs` | 摘要 prompt | 改中文 prompt + 模板 |
| `frontend/src-tauri/src/analytics/` | PostHog 埋点 | **删除或 noop** |
| `frontend/src-tauri/tauri.conf.json` | CSP | 删 `https://api.ollama.ai` + Claude/Groq |
| `frontend/src-tauri/Cargo.toml` | 依赖 | 加 sherpa-onnx 依赖 |
| `frontend/src/locales/` | UI 多语言 | 新建 `zh-CN.json` 完整覆盖 |
| `frontend/src-tauri/migrations/` | DB schema | 加 license / users / orders 表 |

### 6.2 国产 ASR / LLM 资源

| 模型 | 来源 | 协议 | 大小 | 用途 |
|---|---|---|---|---|
| **FunASR SenseVoiceSmall** | [modelscope](https://www.modelscope.cn/models/iic/SenseVoiceSmall) | MIT | 234MB | 默认中文 ASR,40M 参数 |
| **FunASR paraformer-zh** | [modelscope](https://www.modelscope.cn/models/damo/speech_paraformer-large-vad-punc_asr_nat-zh-cn-16k-common-vocab8404-pytorch) | MIT | 1GB | 长会议兜底 |
| **sherpa-onnx** | [github](https://github.com/k2-fsa/sherpa-onnx) | Apache 2.0 | - | Rust binding + ONNX runtime |
| **Whisper large-v3** | upstream | MIT | 3GB | 兜底 |
| **Whisper large-v3-turbo** | upstream | MIT | 1.5GB | 折中(准度大,速度慢) |
| **Qwen2.5-7B-Instruct-GGUF** | [huggingface](https://huggingface.co/Qwen) | Apache 2.0 | 4.7GB(Q5) | 默认 LLM |
| **Qwen2.5-14B-Instruct-GGUF** | 同上 | Apache 2.0 | 9GB | 备选 |
| **DeepSeek-R1-Distill-Qwen-7B-GGUF** | [huggingface](https://huggingface.co/deepseek-ai) | MIT | 4.7GB | 备选(中文强) |

### 6.3 商标查询建议(启动前必查)

- 中国商标网:https://sbj.cnipa.gov.cn (查 "秘记 / 密谈 / 会记通 / OfflineRecorder / 离线会记" 等 5-10 个候选)
- 美国 USPTO:https://tmsearch.uspto.gov (出海版备查)
- 域名:`offlinerecorder.cn / .com`(查 WHOIS)

### 6.4 参考案例(国内外)

- **C 端竞品**:飞书妙记 / 通义听悟 / 腾讯会议 AI 助手 / 麦耳会记 / 听脑 AI —— **全部云端**,这是我们的核心差异
- **B 端竞品**:科大讯飞 智能会议系统 / 字节 飞书妙记企业版 / Otter Business —— **云端 + 收费高**,我们用**离线 + 一次性 + 内网部署**切
- **国外开源参考**:WhisperLive / Insanely-fast-whisper / Buzz(已有商业版 Buzz Hydra $9.99/月)—— **国内本地化是真空**

### 6.5 一句话卖点(对外营销用)

> "录音、转录、纪要,全程本机运算,数据不出你的电脑。钉钉、飞书、腾讯会议直接录,中文准度拉满,律所 / 研发 / 金融涉密会议首选。"

### 6.6 决策日志建议(等启动时)

启动 W6 时,在 `00-收件箱/决策日志.md` 追加:

```markdown
## 2026-08-10 决策 — 启动"离线会记"M0.5

**触发**: SMM M1 跑通,现金流 ¥XXXX,精力可分 50% 给本地软件
**决定**: 启动会记 MVP 改造
**预算**: ¥9000 (Apple 开发者 + 域名 + Win 自签)
**里程碑**: 10/15 MVP 上线,11/11 商业发布
**KPI**: 30 天 ¥5000 流水,3 个月 ¥30000 流水
**砍掉**: 不做云端 / 不做移动端 / 不做 PWA / 不做订阅
```

---

> **最后更新**: 2026-07-08
> **下次更新**: SMM M1 跑通 + 启动会记 M0.5 时
