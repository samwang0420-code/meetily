- `frontend/src/contexts/TranscriptContext.tsx` (lastDecodeMs + 400ms 渐隐)
- `frontend/src/components/TranscriptView.tsx` (decode_ms chip + 0.15s transition)

### 已知边界
- silent abort 可能是 macOS GUI app 没 launchctl session + Info.plist 配置启动 type, 待用户实际打开 app 验证
- streaming session 现在 1 个 record (lazy), 多 worker 并发不安全 (当前 `NUM_WORKERS=1` 不触发)
- `token != last_emitted_final` diff 逻辑在 daemon.py 实现, provider 层不重做

### 下一步候选 (本轮不做, 留给下一轮)
1. 加 Dobao 流式云端 provider 作"联网高质量模式" (差距真正补齐)
2. 把 Worker::Provider path 的 streaming session 移到全局 Mutex<HashMap<sid, Session>>
3. speaker diarization (多说话人)
4. ITN (数字/日期正则后处理)
5. 录制面板右下加 chip 显示 p95 delay (实时观感)

---

## 2026-07-12 · 紧急回滚 v0.6.12 streaming 改动(识别 0 段 bug fix)

**用户反馈 (7/12 14:00)**: "你看我最近的录音,根本没有实时识别出内容"

**DB 实证**:
```
meeting-f333... 2026-07-12T05:57:44|0 segments  ← 今天 13:55
meeting-33ce... 2026-07-11T07:22:11|0 segments  ← 7/11 15:21 (我当时在改 streaming)
meeting-1dff... 2026-07-10T10:25:49|3 segments  ← 7/10 18:22 ✓
meeting-54cb... 2026-07-10T09:49:01|1 segment   ← 7/10 17:47 ✓
```

**根因**: v0.6.12 (昨天提交) 我把 streaming 接入录音管线时埋的 bug:
- 我把 `worker.rs` 第 497 行的 `TranscriptionEngine::Sherpa =>` 分支 **改成 stub**: `warn() + return Err`,作为占位符
- 同时 `engine.rs` 把 sherpa 分支从 `Ok(Sherpa)` (unit variant) 改成 `Ok(Provider(SherpaProvider::new(model, app)))`
- **结果**: provider 配置 = sherpa_funasr_nano,worker 拿到 `TranscriptionEngine::Sherpa` unit variant → 命中我加的 stub → **永远 return Err → 0 段**
- 而 user 后台跑的就是 7/11 15:56 rebuild 的 binary (包含这个 bug)

**修复 (5 步)**:

1. **`worker.rs` line 497 Sherpa 分支**: 从 stub 改成 bak2 真分支:
   ```rust
   TranscriptionEngine::Sherpa => {
       let language = crate::get_language_preference_internal();
       let model_name = crate::api::api::api_get_transcript_config(...).await
           .ok().flatten().map(|c| c.model)
           .unwrap_or_else(|| "sense-voice-zh-int8".to_string());
       let provider = crate::audio::transcription::SherpaProvider::new(model_name);
       match provider.transcribe(speech_samples, language.clone()).await { ... }
   }
   ```
2. **`engine.rs`**: 回滚到 `Ok(TranscriptionEngine::Sherpa)` (unit variant),删 Arc/TranscriptionProvider import
3. **`sherpa_provider.rs`**: 重写回老 API (`new(model)`, 不接 `app_handle`),行为用 `transcribe_blocking` 整段(非 streaming)
4. **`sherpa_asr.py`** + **`sherpa_daemon.rs`**: 阈值 `chunk_threshold` 600,`silence_threshold` 1200(原 7/10 值)
5. 加 `use tauri::Manager` import + `TranscriptionProvider` import(transcribe 方法 trait bound)

**关键修正(回滚 bak2 后)**: bak2 写 `Some(language.clone())` 给 transcribe,但当前 provider trait 第 2 参数是 `Option<String>`, `language = get_language_preference_internal()` 已经是 Option<String>,**多包 Some 是错误的**。正确:`provider.transcribe(samples, language.clone())`

**未撤回的前端改动 (本轮保留,本轮新 binary 也无害)**:
- `transcriptService.ts`: decode_ms / buffer_age_ms 字段 (可选,没 streaming 也不会触发)
- `TranscriptContext.tsx`: lastDecodeMs state (none 不显示)
- `TranscriptView.tsx`: decode ms chip (optionally hidden)
- `worker.rs` 中 `TIMING_*` static + `get_streaming_timing_stats` tauri command (backend 不会调用,但暴露无害)
- `mod.rs` 加了 `get_streaming_timing_stats` re-export

**改动仅本轮内净增量**:
- 新增 static: TIMING_LAST_EMIT_MS, TIMING_RING_HEAD, TIMING_RING_FILLS, TIMING_RING_DECODE, TIMING_RING_BUFFER_MS + tauri command `get_streaming_timing_stats` (未使用但保留备将来 PR)
- sherpa_daemon.rs: 阈值恢复 600/1200
- sherpa_asr.py: 阈值恢复 600/1200, 删 `min_partial_interval_ms` 逻辑

**验证 (本轮)**:
- `cargo check`: 干净 (13 warnings, 0 errors)
- `tsc --noEmit`: 干净
- `cargo build --release`: 成功,二进制 14:08 时间戳 66MB
- **Python daemon 协议实测**:
  - list: 2 models ✓
  - transcribe_blocking 静音 1s: text='The.' confidence=0.92 duration_ms=684 (694ms 内出 1s 静音识别为 'The.')
  - transcribe_blocking 5s 模拟音频: text='The.' confidence=0.92 duration_ms=691 (0.76s 端到端)
- binary build 时间戳 14:08 (本轮刚生成的)

**运行验证**:
- 用户启动新 binary (14:08) → 录音 → 应该恢复正常识别(transcribe_blocking 整段,延迟 600-1500ms)
- 仍未接 streaming (用户感知"实时字幕" 仍然 0 段 partial 浮现,但 final 段会出文字)

**待办 (下一轮留 PR)**:
- 把 streaming 拆成单独 PR,**先在 dev 路径跑一周稳定后再合 main**,而不是像我这次直接打在 main binary
- 加一个 `MIN_REALTIME_TEST_DURATION` 校验,validate streaming 落库后看 transcript 段数 >=1
- 加 staging flag `STREAMING_ENABLED=true` 让 main path 默认关闭 streaming
