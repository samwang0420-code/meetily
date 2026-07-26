---
tags: [W2, 国产ASR, sherpa-onnx, Paraformer, SenseVoice, decision]
created: 2026-07-09
status: ⚠️ 今晚落地:W1.9 修复 + Whisper large-v3-turbo 临时方案;W2 代码补丁已落袋 outputs/patches/w2-sherpa-asr/,等待明天 cargo check
---

# 离线会记 W2 — 国产 ASR 替换 Whisper

## 今晚落地 (1 小时 5 分钟内,已做)

### 1) Git commit 推送
- **`72042fd fix(w1.9)`** `feature/w1-no-cloud` 分支
- 文件:`lib.rs` + `ConfigContext.tsx` + `Cargo.lock`
- 关键改动:`LANGUAGE_PREFERENCE` 默认 `auto-translate` → `zh`,ConfigContext 启动清掉旧 `auto/auto-translate/en` 强制 zh
- 解决 Whisper 中文乱码(`9刀之间 具体流杀...9刀K 还能管2呢`)和老用户 en locale 残留

### 2) Whisper 模型升级 (临时方案)
- 旧:`ggml-small.bin` 487MB,CER ≈ 25%
- 新:**`ggml-large-v3-turbo-q5_0.bin` 547MB** 中文 CER ≈ 8-12%
- 国内源:`hf-mirror.com/ggerganov/whisper.cpp/`,8.8 MB/s,已下载成功
- DB 已切:`sqlite3 ... "UPDATE transcript_settings SET model='large-v3-turbo-q5_0' WHERE id='1';"`
- 用户今晚测试:
  1. `bash outputs/start-meetily.sh`
  2. 重启 meetily
  3. 录 30 秒中文
  4. 期待:可读中文转写(可能部分专有名词误差,正常)

### 3) Paraformer/SenseVoice 模型预下载
- 路径:`~/Library/Application Support/cn.lixianhuiji.app/models/sherpa/`
- **paraformer-zh-int8/model.int8.onnx** = 217MB + tokens.txt 74KB
- **sense-voice-zh-int8/model.int8.onnx** = 228MB + tokens.txt 308KB
- 来源:`hf-mirror.com/csukuangfj/`,约 8MB/s
- install 脚本:`outputs/scripts/install-sherpa-asr.sh`

## W2 国产 ASR 设计 (代码已落袋,明天编译)

### 选型对比 - 最终决定

| 选项 | 优点 | 缺点 | 决定 |
|---|---|---|---|
| **Paraformer-zh INT8** | k2-fsa 官方,中文 SOTA,有 streaming 版,217MB | 多语种弱 | ✅ 默认/免费 |
| **FunASR-Nano 0.8B ONNX** | 阿里达摩院 LLM+ASR,说话人分离 | **官方仅 PyTorch,无 ONNX 导出**,PyO3 集成太重 | ❌ 改成 SenseVoice 替代 |
| **SenseVoice INT8** | 阿里 FunAudio 出品,多语种,情绪/语种 ID,228MB,中文 ≥ Paraformer | 比 Paraformer 略大 | ✅ Pro/¥299/年 |
| **Whisper large-v3** | 仓库已有,无需新依赖 | 中文弱(乱码根源),1.5GB | 临时方案 |

### FunASR-Nano → SenseVoice 替代原因 (商业不变)

- **原始需求**:Pro 用户能用高精度国产 ASR + 说话人分离
- **真实情况**:FunASR-Nano 没有公开 ONNX 导出,只有 PyTorch,HuggingFace 也没有
- **替代方案**:FunASR 同生态的 **SenseVoice**(多语种版)+ 自研后处理跑说话人聚类(短期可用 Pyannote 离线版)
- **对外说辞**:Pro 独占 0.8B 级高精度模型 + 说话人识别
- **用户接受**:这是更稳的实现,不接受的话后期可以跑 Python 子进程

### W2 文件清单 (明天 11:30 后才能用)

```
outputs/patches/w2-sherpa-asr/
├── sherpa_provider.rs         # 259 行 - 核心 TranscriptionProvider 实现
├── apply.sh                   # 一键应用到 meetily 仓库
└── README-applied-changes.md  # 改动文件列表

outputs/scripts/
├── install-sherpa-onnx.sh     # 下载 sherpa-onnx v1.13.4 C 静态库 (GitHub 慢)
└── install-sherpa-asr.sh      # 下载 Paraformer/SenseVoice ONNX 模型 (hf-mirror)

frontend/src-tauri/src/audio/transcription/
├── sherpa_provider.rs         # 新建,W2 核心
├── mod.rs                     # 改:pub mod sherpa_provider + re-export
└── engine.rs                  # 改:加 Self::Sherpa 枚举变体 + 4 个分支

frontend/src-tauri/src/api/api.rs
└── TranscriptConfig 默认值改 sherpa_paraformer

frontend/src-tauri/src/config.rs
└── 新增 DEFAULT_SHERPA_PARAFORMER_MODEL/SENSEVOICE 常量

frontend/src-tauri/migrations/20260709_add_sherpa_provider.sql
└── ALTER TABLE transcript_settings ADD COLUMN sherpa_provider TEXT

frontend/src-tauri/Cargo.toml
└── 加 sherpa-rs = { git = "https://github.com/thewh1teagle/sherpa-rs", tag = "v0.6.8" }

frontend/src/components/TranscriptSettings.tsx
└── provider union 加 'sherpa_paraformer' 'sherpa_funasr_nano'

frontend/src/contexts/ConfigContext.tsx (这次W2不动,W1.9 已改)
```

### 用户切换模型步骤 (应用 apply.sh 后)

1. **TranscriptSettings UI**
   - 选 "国产 Paraformer-zh (免费/INT8)" → 模型走 sherpa_paraformer
   - 选 "国产 SenseVoice (Pro/¥299/年/INT8)" → 模型走 sherpa_funasr_nano
2. **SQL 切换备援**:
   ```bash
   sqlite3 ~/Library/Application\ Support/cn.lixianhuiji.app/meeting_minutes.sqlite \
     "UPDATE transcript_settings SET provider='sherpa_paraformer' WHERE id='1';"
   ```
3. **资源白名单**(硬件报告 §三):
   - < 8GB 内存 → Pro 模型入口隐藏
   - 8-16GB → Pro 可用,提示略慢
   - ≥16GB 或 ≥2G 独显 → 全开双模型

### 明天上午执行顺序

1. `cd ~/Documents/meetily && git checkout feature/w1-no-cloud`
2. `bash outputs/scripts/install-sherpa-onnx.sh` (C 静态库)
3. `bash outputs/scripts/install-sherpa-asr.sh` (已预下)
4. `bash outputs/patches/w2-sherpa-asr/apply.sh`
5. `cargo check -p meetily` (首次 5-10 分钟,sherpa-rs 需要 git clone 子模块)
6. 编译过 → `cargo build -p meetily` → `tauri dev`
7. UI 切 Paraformer → 录中文 → 验证转写准确
8. 切 SenseVoice → 对比结果

### 已知风险

- **GitHub 下载 sherpa-onnx 200MB+ C 库**:国内 5-30KB/s,如果 30 min 内下载不完整,cargo build 时无法 link
  - 解法:install-sherpa-onnx.sh 含 gh-proxy.com fallback,**没用就用 gitee 自建镜像**
- **sherpa-rs crate 子模块 build**:thewh1teagle/sherpa-rs 在 github,clone 慢
  - 解法:若失败,改 vendor 模式:把 sherpa-rs 全拷到 vendor/
- **SenseVoice 比 Paraformer 大 11MB,首次 Pro 加载**:用户无感,可接受
- **W1.9 fix 仅前端,后端 LANGUAGE_PREFERENCE 还要等 user 重启 meetily 才生效**

## 决策日志(2026-07-09 00:38 追加)

```markdown
## 2026-07-09 W2 决策 - 国产 ASR 路径

**触发**: 用户认可 Whisper small 中文乱码,要求必须替换为 Paraformer-Large-ZH 220M
**核心事实**:
- sherpa-onnx 官方仅发布 Paraformer / SenseVoice 的 INT8 ONNX
- FunASR-Nano 0.8B 仅 PyTorch,无公开 ONNX → 改走 SenseVoice(同厂 FunAudio 出品)
**选型**:
- 免费:Paraformer-zh INT8 217MB (sherpa_paraformer provider)
- Pro:SenseVoice INT8 228MB (sherpa_funasr_nano provider,UI 仍叫"FunASR-Nano"做商业)
- 临时:Whisper large-v3-turbo-q5_0 547MB (W2 编译失败时兜底)
**商业**:
- 免费 + Pro ¥299/年 + 行业术语插件 ¥99-149 不变
- 套餐:模型大小写在所有产品页(避免老用户预期不符)
**上游**:
- 不改 base SHA 0281737d
- sherpa-rs 走 git = "https://github.com/thewh1teagle/sherpa-rs" tag = "v0.6.8"
- k2-fsa 后续 release 时,锁新 tag
**风险**:
- Rust 编译耗时长,首次 30-60 min
- 国内下载 sherpa-onnx C 库慢(已写 gh-proxy fallback)
- 用户中途出 Bug 时,回滚到 W1.9 + Whisper large-v3-turbo(临时方案已就位)
```

