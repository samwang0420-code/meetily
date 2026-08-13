# §103 VAD buffer warn 噪音 + sherpa_asr.py duration_ms 命名 (2026-08-11)

## 触发

用户 8/11 重新导入 13430280252492828.mp4 (1:49:57 stereo mp4, 5.97GB), 
log 显示 §102 修复完全生效:
- decode 第一遍 145M samples / 3299s / 1ch (symphonia stereo downmix)
- §102 fallback triggered → ffmpeg → WAV (7s) → 581M / 6597s / 2ch ✅
- VAD 66 → splitter 537 segments
- ASR 537/537, avg confidence 0.90, 总耗时 ~25min

但 log 体积 20MB, 全是 warn 噪音:
```
$ grep -c "VAD.*buffer is large" /tmp/meetily_verify_102.log
144357
```

每次 30ms VAD process_chunk 都 warn 一次 (1M samples 阈值偏低), 旧设计就是 spam。

## 修复

### 1. VAD buffer warn 跨阈值只 warn 一次

`frontend/src-tauri/src/audio/vad.rs`:
- struct 加 `warned_about_buffer: bool` flag
- 构造时 init `false`
- `process_chunk` 阈值 1M → 9.6M samples (10 min at 16kHz)
- check `&& !self.warned_about_buffer`, 命中后 set true
- SpeechEnd 处 reset flag (line 275 附近, `current_speech.clear()` 之后)

```rust
// §103: VAD buffer warn 噪音 — 阈值提升到 10 min (9.6M samples), 跨阈值只 warn 一次
// 旧阈值 1M samples (62.5s) 偏低, 长录音正常超过, 日志被撑大到 20MB+
const VAD_BUFFER_WARN_THRESHOLD: usize = 9_600_000; // 10 min at 16kHz
if current_speech_size > VAD_BUFFER_WARN_THRESHOLD && !self.warned_about_buffer {
    warn!("VAD: Accumulated speech buffer is large: {} samples ({:.1}s) - possible memory issue (will not re-warn until SpeechEnd)",
          current_speech_size, current_speech_size as f64 / 16000.0);
    self.warned_about_buffer = true;
}
```

### 2. sherpa_asr.py duration_ms 命名误导

`frontend/src-tauri/scripts/sherpa_asr.py`:
- `duration_ms = int((time.time() - t0) * 1000)` — 实际是 ASR 总耗时 (含 VAD/IO/热词)
- JSON 返 `decode_ms` + `duration_ms` + `audio_seconds` 三字段, 但 decode_ms (纯推理) ≈ duration_ms
- 改名为 `total_ms`: `total_ms = int((time.time() - t0) * 1000)  # §103: renamed from duration_ms`
- JSON 字段同步: `"total_ms": total_ms,`

## 验证

```bash
# §103 修复后预期 (用户重启 binary + 重新导入同一文件):
grep -c "VAD.*buffer is large" /tmp/meetily_verify_103.log   # 预期 < 10 (旧: 144357)
grep "total_ms" /tmp/meetily_verify_103.log | head -3        # 改用 total_ms 字段
```

预期 buffer 最大累计 8.86M samples ≈ 553s (实测旧 log) → 仍跨 10min 阈值, 但只 warn 1 次。

## §37 硬闸门

- ✅ cargo check --lib: 0 errors (28 §18 warnings 不动)
- ✅ guard 121 → 124/124 PASS (3 §103 新锚点)
- ✅ cargo build --release: 1m30s, binary 10:23 72M
- ✅ sync_app_bundle.sh: §99.6 tauri bundle SHA 一致

## 已知边界

- 8.86M samples 是用户当前 1:49:57 录音实际峰值, 10min 阈值正好触发
- 极长录音 (>3h) 仍可能单次 cover, 但不会再 spam warn
- sherpa_asr.py 改名是 JSON 字段, 不影响前端 (decode_ms 仍存在, total_ms 仅多一个 log 字段)

## 关联

- §102 (symphonia stereo downmix fallback 基础)
- §37 (硬闸门)
- §56 (AGENTS.md 描述 ≠ 代码 commit, 此次写到 commit 同步)
