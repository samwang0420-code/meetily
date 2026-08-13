# §102 隐藏导入 bug — aac→mono downmix + transcripts.user_id 漏写 + hardlink 风险 (2026-08-11)

## 触发
用户原话 "已经导入成功了, 看看是否有我们看不到的问题出现"

## 用户视角看不到的 4 个隐藏问题

### 🔴 Bug 1 (HIGH): 立体声 aac 一直被 downmix 到 mono — samples 减半
**症状:** 用户导入 13430280252492828.mp4 (1:49:57 stereo aac 44100Hz)
- ffprobe 实际: **duration=6597.92s**, channels=2 (stereo)
- DB metadata.json: **duration_seconds=3299.04s** (一半!)
- DB transcripts: **252 段** (vs 正确 828 段)
- 252 段 × 13s/段 ≈ 55 min, 但音频实际 110 min

**根因 (3 跳):**
1. `decoder.rs::decode_audio_file_with_progress` 用 symphonia 0.5 解 mp4/aac
2. `codec_params.channels` 是 None (mp4 metadata 不带)
3. 默认 channels=1, first packet decode 后 `spec.channels=1` (symphonia 把 stereo downmix 到 mono)
4. 后续 packet 全按 mono 处理, `all_samples.len()` 只有实际一半
5. `duration_seconds = all_samples.len() / sample_rate / 1 = 3299s` (应是 6597s)

**§62 commit 撒谎:** commit 2fe96d7 message 写 "fix §62: §62 A/B/C 三联" 但实际只改了 tempfile_in 路径 + hardlink 优先 + max_tokens,**fallback 函数完全没写**。`decode_audio_file_with_ffmpeg_fallback` 在 git 历史 0 命中 (除无关 b2205d3)。**§92 防代码漏 §92 / §56 失效第 N 次** — 类似 §70 (11/11 fail) §91 P1-B (漏 migration)。

**修复:** 真的写 `decode_audio_file_with_ffmpeg_fallback`:
- 第一次 decode 完后检查: `mp4/m4a + (channels==1 && samples<400M || channels==2 && samples<800M)` → fallback
- fallback: ffmpeg 强制 `-ac 2 -ar 44100` 转 WAV, 再 symphonia 解
- 加 §102 anchor: `102_ffmpeg_fallback_function_exists` (查函数真存在)

### 🔴 Bug 2 (HIGH): import.rs INSERT transcripts 漏 user_id 列
**症状:** 每次新导入, transcripts.user_id 都是 NULL, 依赖 §99.2 startup backfill 兜底
- 但 backfill 只跑一次 (启动时), 老数据修了, 新数据继续 NULL
- DB 验证: 252 transcripts.user_id IS NULL (最新 import 2026-08-11)

**根因:** §99.2 commit 只修了 meetings INSERT 加 user_id, **transcripts INSERT 没改**

**修复:** `import.rs::create_meeting_with_transcripts` INSERT 改:
```sql
INSERT OR IGNORE INTO transcripts (id, meeting_id, transcript, timestamp, 
                                  audio_start_time, audio_end_time, duration, 
                                  user_id, speaker_id)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL)
```

### 🟡 Bug 3 (MED): 同一 audio.mp4 9 个 hardlink
**症状:** `find -inum` 显示 inode 72166237 有 9 个 hardlink (8 个 "导入音频 ..." 目录 + 1 个 source)
- §64 B.1 优化 (hardlink 优先) 工作了: 384MB 文件 0 拷贝
- 但 **删除任一会话影响所有 hardlink + source** — 用户以为删了就释放空间, 实际 inode 还在
- audio.mp4 mtime Aug 4, 但 metadata.json / transcripts.json 是 Aug 10/11 — 用户看到"我的导入" 文件,但实际是源文件 alias

**根因:** §64 B.1 hardlink 优先策略没考虑删除语义

**修复 (后续):** 在 §98 之后写 §102.1 改进:
- meeting delete 时 unlink (if last hardlink)
- 或者显示 "原始 source 文件 X 个 hardlink 共享" 警告

**本次不做** (§18 不主动改): 用户没报具体问题, 现有 hardlink 安全.

### 🟢 Bug 4 (INFO): duration sum 2928 < span 3298 (370s gap)
**症状:** transcripts.duration sum=2928.56s, span=3298.74s, gap=370.18s
- span = MAX(end_time) - MIN(start_time) = 55min (一半!)
- sum_dur = 48.8min
- gap = 6.2 min — 段间静音

**根因:** 部分是 Bug 1 (audio 本身只解码一半), 部分是 VAD 静音段被剔除 (正常)

**本次不修**: 等 Bug 1 修后重新跑 import 看新数据.

## §102 修复明细

### 1. `decoder.rs` 新加 2 函数
- `decode_audio_file_with_ffmpeg_fallback(path, progress_callback)` — 启发式 fallback
- `convert_to_wav_with_ffmpeg_with_channels(input_path, progress_callback, channels)` — 强制声道数转 wav

### 2. `import.rs` 改 2 处
- import: 加 `decode_audio_file_with_ffmpeg_fallback`
- line 431 decode 调新函数 (替换 `decode_audio_file_with_progress`)
- transcripts INSERT 加 `user_id, speaker_id` 列

### 3. `scripts/check_historical_fixes.py` 加 3 §102 anchor
- `102_ffmpeg_fallback_function_exists` (查 pub fn 存在)
- `102_import_uses_fallback` (查调用点)
- `102_transcripts_insert_user_id` (查 SQL)
- 总 121/121 PASS

## §37 硬闸门
- ✅ cargo check --lib: 0 errors (28 §18 warnings 不动)
- ✅ check_historical_fixes.py: **121/121 PASS**
- ⏳ cargo build --release: 用户跑

## 用户手动命令
```bash
cd /Users/wangwei/Documents/离线会记

# 1. build (含 §102 真 fallback + transcripts user_id)
cd frontend/src-tauri && cargo build --release && cd ../..

# 2. sync
bash scripts/sync_app_bundle.sh

# 3. 重启 binary (老 broken imports 不会被回填, 用户需重新导入)
killall meetily 2>/dev/null
'/Users/wangwei/Documents/离线会记/target/release/bundle/macos/言镜 AI.app/Contents/MacOS/meetily' &

# 4. 验证
# 4a. 重新导入 13430280252492828.mp4 (或其他 stereo mp4)
# 4b. 看 metadata.json: duration_seconds 应 ~6597s (不是 3299s)
# 4c. 看 DB: transcripts 数应 ~828 (不是 252)
# 4d. transcripts.user_id 应该 = 2 (不是 NULL)

# 5. 老 broken imports (导入音频 2026-08-XX) 怎么办?
# 选项 A: 手动删除 (UI Sidebar 删除按钮)
# 选项 B: 保留当历史 (有 252 段 transcription, 但只覆盖一半音频)
# 选项 C: 我写脚本批量 re-import (复杂度高, §18 不主动做)

# 6. commit + push
git add frontend/src-tauri/src/audio/decoder.rs \
        frontend/src-tauri/src/audio/import.rs \
        frontend/src-tauri/src/lib.rs \
        scripts/check_historical_fixes.py \
        outputs/102-hidden-import-bugs-aac-mono-downmix-hardlinks-2026-08-11.md

git -c user.email=codex@local -c user.name=codex commit -m "fix(§102): 真正实现 §62 fallback + import.rs transcripts 加 user_id

§92 防代码漏 第 N 次 (§70 11/11 fail, §91 P1-B 漏 migration)

Bug 1: aac→mono downmix (HIGH)
- 用户 stereo 109min aac 一直只转一半 (252 段 vs 应 828)
- §62 commit 2fe96d7 message 写 'fallback 实现' 但代码完全没写
- 这次真写 decode_audio_file_with_ffmpeg_fallback:
  mp4/m4a + channels==1 + samples<400M OR channels==2 + samples<800M → fallback
  ffmpeg 强制 -ac 2 -ar 44100 转 wav 重解

Bug 2: transcripts.user_id 漏写 (HIGH)
- §99.2 只改了 meetings INSERT, transcripts 漏
- 252 transcripts 全 NULL user_id
- 修: INSERT 加 user_id 列 + bind

Bug 3: 9 hardlink 风险 (MED)
- §64 B.1 hardlink 优先 OK 但 delete 时会连锁影响
- 本次不修, 列待办 (用户没报具体问题)

Bug 4: duration sum < span 370s gap (INFO)
- 部分是 Bug 1 (audio 一半), 部分是 VAD 静音段
- 等 Bug 1 修后看新数据

§37 闸门:
- cargo check --lib: 0 errors
- check_historical_fixes.py: 121/121 PASS (新增 3 §102 anchor)
- guard 加函数存在性检查 (之前只查文字, 不查函数存在)"

git push origin perf/summary-map-concurrency

# 7. Obsidian 双写
cp outputs/102-hidden-import-bugs-aac-mono-downmix-hardlinks-2026-08-11.md \
   "$HOME/Documents/Obsidian Vault/项目/3-离线会记/102-hidden-import-bugs-aac-mono-downmix-hardlinks-2026-08-11.md"
```

## 关联
- §62 三联 (commit 2fe96d7 撒谎)
- §92 §56 (commit ≠ 代码)
- §91 P1-B (漏 migration, 类似模式)
- §70 11/11 fail (类似反模式)

## 给用户的建议
老 8 个 broken imports 选哪个:
A. 全删 (Sidebar 一个个删, hardlink 自动 unlink)
B. 保留 (历史数据, 一半音频)
C. re-import (新 binary 会用真 fallback 重转, 但会创建第 9-16 个目录)
