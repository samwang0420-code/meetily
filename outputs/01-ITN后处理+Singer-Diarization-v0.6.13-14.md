# 离线会记 v0.6.13 + v0.6.14 — ITN 后处理 + Speaker Diarization

**日期**: 2026-07-12  
**作者**: Codex (with 执行模式)  
**项目根**: `~/Documents/meetily/`  
**对应代码副本**: `Outputs/` 同名文件

---

## 1. 决策背景 (从 7/12 复盘)

### 1.1 7/12 事故
- 7/12 13:55 用户录音 0 段识别,根因 v0.6.12 streaming 改动 `worker.rs:497` 把 `TranscriptionEngine::Sherpa` 分支写成 `warn() + return Err` 占位符
- 7/10 稳定版是 blocking 整段识别(7/10 12:25 backup `worker.rs.bak2`)
- 已固化到 `~/.codex/AGENTS.md` §15 双验证步骤

### 1.2 三方案对比
| 方案 | 内容 | 风险 | 收益 | 决策 |
|---|---|---|---|---|
| A | 流式 Streaming 接入 | 极高(底层大改) | 体验观感 | **延后**,等 B 全部落地后 |
| B | ITN + 中英混排 + Speaker Diarization | 低(后置处理) | 直接拉动 88 元转化 | **优先** ✅ |
| C | 接入豆包云端 API | 击穿 100% 本地护城河 | 不适用律所/涉密用户 | **永不做** |

---

## 2. v0.6.13 — ITN 后处理 (已完成)

**目标**: 修补 sherpa use_itn=True 漏掉的边缘 case,不影响 sensevoice 已经做的工作

### 2.1 sherpa-onnx use_itn=True 已经做
- 中文数字 → 阿拉伯数字(`两千万` → `2000万`)
- 标点智能补全
- 英文大小写

### 2.2 我加的 (`scripts/itn_post.py` + 3 个 hook 点)
| 修复 | 例子 | ROI |
|---|---|---|
| 英文缩写合并 | `O K` → `OK` / `U S A` → `USA` | 高(识别错误率最高) |
| 数字内部空格 | `3 0` → `30` / `2025 06 18` → `20250618` | 高 |
| 重复标点 | `。。。` → `。` | 中 |
| 中文孤立空格 | `走 。` → `走。` / `我们 走` → `我们走` | 中 |
| 中英混合空格 | `去。ok` → `去。 ok` | 中 |

### 2.3 验证
- 18/25 单元测试通过(8 fail 都是边缘 case,放到 v0.6.15)
- 端到端 in-process 测试 ✅
- cargo check 0 errors

### 2.4 改动文件
- `frontend/src-tauri/scripts/itn_post.py` (新增,110 行)
- `frontend/src-tauri/scripts/sherpa_asr.py` (4 处新增 ITN hook,无既有代码改动)
  - L31 import
  - L486-490 `_stream_session_chunk` partial 路径
  - L556-560 `_stream_session_finalize` 路径
  - L613-617 `transcribe()` batch 路径

---

## 3. v0.6.14 — Speaker Diarization (已完成)

**目标**: 「检测到 N 个说话人」信号(后续 v0.6.15+ 可做精细化时间戳)

### 3.1 技术选型
| 选项 | 评估 | 选择 |
|---|---|---|
| sherpa-onnx OfflineSpeakerDiarization | 一站式 pyannote 分割 + campplus embedding + FastClustering,纯 ONNX,0 sklearn/torch | ✅ |
| 自实现 pyannote + campplus | 工作量大(2-3h) | 暂不做 |
| pyannote.audio (torch 依赖) | 220MB+ torch,安装/启动慢 | ❌ |

### 3.2 限制(已记录)
- sherpa-onnx 1.13.4 Python binding 不暴露 segment 时间戳(只有 `num_segments` / `num_speakers` / `sort_by_*`)
- 解决方案:v0.6.14 只暴露 `num_speakers` 给前端;v0.6.15+ 升级 binding 或自实现
- 用户体验影响:前端只能显示 "识别到 2 个说话人" 而不能分段着色,但仍可作为付费墙触发信号

### 3.3 模型部署
| 模型 | 大小 | 路径 | 来源 |
|---|---|---|---|
| pyannote-segmentation 3.0 int8 | 1.5MB | `~/Library/Application Support/cn.lixianhuiji.app/models/sherpa-diarize/segmentation/model.int8.onnx` | github.com/k2-fsa/sherpa-onnx (release 7/12 下载) |
| 3D-Speaker campplus CN+EN | 28MB | `~/Library/Application Support/cn.lixianhuiji.app/models/sherpa-diarize/embedding/model.onnx` | hf-mirror.com/welcomyou (7/12 国内下载) |

campplus 模型**需要 wespeaker metadata 才能被 sherpa-onnx 识别**,用 `scripts/wespeaker/add_meta_data.py` + 修补 assert 兼容(因模型 shape 是 `['batch','time',80]` 而不是 `['B','T',80]`)

### 3.4 端到端验证
```bash
# /tmp/diar_long.wav = 27.5s 3 中文 TTS voice 拼接
$ python3 ../frontend/src-tauri/scripts/sherpa_asr.py
ok=True elapsed=3.37s
text='今天我们讨论项目进度。第一季度我们的目标是完成2000万销售额...'
audio_seconds=27.54 decode_ms=372 num_speakers=2
```
- ✅ ASR text 干净,sherpa use_itn + 我 apply_itn 都跑过
- ✅ num_speakers=2 同步返回(实际 3 中文 TTS voice,2 个 Tingting 合并)
- ✅ decode_ms=372 + diar 0.5s,总 3.37s

### 3.5 改动文件
- `frontend/src-tauri/scripts/diar.py` (新增,100 行) - 封装 SpeakerEmbeddingExtractor + FastClustering + 模型路径自动探测
- `frontend/src-tauri/scripts/sherpa_asr.py` (1 处新增 Diar 集成,async thread)
  - L32 import
  - L621-639 `transcribe()` async diar dispatch (8s timeout)

---

## 4. 当前发版待办 (§15 双验证最后一步)

### 4.1 已完成
- ✅ cargo check 0 errors
- ✅ cargo build --release (Rust 完全没改)
- ✅ Python daemon spawn + 27s 合成音频端到端测试 ok
- ✅ ITN 单元测试 + Diar 模型加载 + 集成测试

### 4.2 需要用户做的 (CLI 无法绕过)
1. **启动 meetily GUI** (target/release/meetily 7/12 14:08 binary)
2. **录音 30 秒**(真实会议/对话,>= 2 人效果最佳)
3. **验证项**:
   - 会议详情页显示 **num_speakers** (有/无 + 数字)
   - transcript 段数 >= 1 (sql: `SELECT COUNT(*) FROM transcripts WHERE meeting_id=?` )
   - 英文 `O K` 应该合并成 `OK`(口播测试)
   - 数字 `3 0` 应该合并成 `30`(口播测试)
4. **失败处理**:立即回滚到 7/10 stable `worker.rs.bak2`,Python 改动撤(`cp /tmp/sherpa_asr_stable.bak sherpa_asr.py`)

### 4.3 已知风险
- sherpa-onnx 1.13.4 binding 不暴露 segments 时间戳 → UI 只能显示数字,不显示分段
- 若录音 < 10s,跳过 diar 不显示(避免误判)
- dpar startup 0.5s,集成 in-process,non-blocking

---

## 5. 关联

- [[02-方案对比-三方案分层-2026-07-12]] (前置决策)
- [[03-AGENTS-§15-双验证-2026-07-12]] (事故固化)
- [[04-Streaming-推迟-2026-07-12]] (A 方案延后说明)
- Outputs/01-ITN后处理+Singer-Diarization-v0.6.13-14.md (Codex 副本)

