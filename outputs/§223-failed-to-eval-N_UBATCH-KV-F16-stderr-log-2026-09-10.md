# §223 'failed to eval' 三连修复 (2026-09-10)

## 触发事故

用户 2026-09-10 10:38 触发 §222 binary (Qwen 2.5 3B context=16384, KV Q4_0) 重跑 c1299582 摘要:
- llama-helper 进程 PID 35657 启动,17.3% CPU,750MB RAM (含 Q4_K 模型 + KV)
- 跑 14 分钟,chunk_count=0,最后 status=failed
- error: `Generation failed: Generation failed: failed to eval`

§222 真根因(8K→16K context)修复了"GGML_ASSERT n_tokens > n_batch"的 panic 路径,但漏了 KV Q4_0 + n_ubatch 两个 corner case,触发另一条 `llama_decode() → ctx->decode()` 返回 -2 路径。

## 根因 (3 条叠加)

### 根因 1 — N_UBATCH 没显式设

`llama.cpp/src/llama-context.cpp:163`:
```cpp
cparams.n_ubatch = std::min(cparams.n_batch, params.n_ubatch == 0 ? params.n_batch : params.n_ubatch);
```

§220 commit 时设了 `with_n_batch(N_BATCH=16384)`,但**没设 `with_n_ubatch()`**。Rust binding 默认值 0 → llama.cpp 走 fallback 路径把 n_ubatch = n_batch = 16384。但 llama.cpp 还有 `n_batch % n_ubatch == 0` 的 GGML_ASSERT (line 2691),如果 cparams 处理后 n_ubatch 跟 n_batch 不一致会 panic 或拆分失败。

### 根因 2 — KV cache Q4_0 + n_ctx=16K 触发 memory->init_batch 失败

§215 commit 把 KV cache 设成 Q4_0 (cnblogs/itech/p/19919532 文章推荐 4K context 用)。但 c1299582 §222 实际 context=16384,KV Q4_0 + Qwen 2.5 3B Q4_K 模型 + n_ctx=16384 组合下,`memory->init_batch()` 在 `LLAMA_MEMORY_STATUS_FAILED_PREPARE` 路径返回 → `llama_decode()` 返回 -2。

**llama.cpp 关键 log** (`src/llama-context.cpp:1638`):
```
LLAMA_LOG_WARN("%s: failed to find a memory slot for batch of size %d\n", ...);
return 1;
```

这条 log 在 stderr 里,但 macOS .app bundle stderr 被 LaunchServices 丢弃,**看不到**。

### 根因 3 — macOS .app bundle stderr 不可见

之前 §99.4 / §194 都尝试过用 stderr 文件描述符,但实际所有 stderr 输出都被 macOS LaunchServices 在 .app bundle 启动时丢弃。 `eprintln!` 调用虽然执行了,但没地方写。

## 修复 (§223 三连)

### 修复 A — main.rs 显式 N_UBATCH

```rust
// §220 (2026-09-09): N_BATCH = 16384, 跟 LlamaBatch::new 一致
.with_n_batch(N_BATCH)
// §223 (2026-09-10): N_UBATCH 必须显式跟 N_BATCH 一致
.with_n_ubatch(N_BATCH)
.with_n_threads(threads)
```

### 修复 B — main.rs 移除 KV Q4_0,改回 F16 default

```rust
// §223 (2026-09-10): KV cache Q4_0 + n_ctx=16K + Qwen 2.5 3B Q4_K 组合实测
//   触发 'failed to eval' (llama_decode 返回 -2 = memory->init_batch() 失败)。
//   改回 F16 KV 是 conservative choice, 16K context F16 KV = 0.32 GB,
//   M3 8GB 还有 5.5 GB headroom, 完全够。
//   Q4_0 KV 留给将来 32K+ context 才考虑 (§215 文章推荐是 8K 以下场景)。
;
```

不再调 `with_type_k/with_type_v`,走 llama.cpp 默认 F16。

### 修复 C — sidecar.rs stderr redirect 到 log file

```rust
fn open_stderr_log_file(app_data_dir: &std::path::Path) -> std::process::Stdio {
    let log_dir = app_data_dir.join("logs");
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        log::warn!("§223 Failed to create log dir {:?}: {}", log_dir, e);
        return Stdio::null();
    }
    let log_path = log_dir.join("llama-helper.log");
    match OpenOptions::new().create(true).append(true).open(&log_path) {
        Ok(file) => {
            log::info!("§223 llama-helper stderr → {}", log_path.display());
            Stdio::from(file)
        }
        Err(e) => {
            log::warn!("§223 Failed to open {:?}: {}", log_path, e);
            Stdio::null()
        }
    }
}
```

`.stderr(Stdio::from(Self::open_stderr_log_file(&self.app_data_dir)))`

路径: `~/Library/Application Support/tech.yanjingai.app/logs/llama-helper.log`

SidecarManager 加 `app_data_dir: PathBuf` 字段,3 处 `Self {}` 初始化同步。

## §37 6 步硬闸门

- ✅ cargo check --lib: 0 errors (17 warnings §18 不动)
- ✅ cargo test --lib: 561 passed / 1 fixture-bound fail (§18 /tmp/transcript_709b.txt 缺失不修)
- ✅ check_historical_fixes.py: **777/777 PASS** (新增 5 §223 anchor,改 3 §215/§216/§217 anchor regex)
- ✅ cargo build --release: 4m18s, binary 59.8M (sha=456993da6411)
- ✅ llama-helper build: 5.17s, binary 4.9M (sha=eeb3556ac27e,跟 §222 一致)
- ✅ sync_app_bundle.sh: 3 binaries synced to 言镜 AI.app bundle + codesign identifier OK + symlink OK
- ⏳ GUI 端到端 (§15 强制,用户必做)

## §15 GUI 验收 (用户必做,不能 CLI 测)

```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```

1. 进 `meeting-c1299582-d80c-4d7d-972f-27e2ee3027d7` (重庆通航融资租赁案)
2. 点 "重新生成摘要"
3. 期望 ~5-15 分钟完成,status=completed,chunk_count=4
4. 任何阶段查 stderr:
   ```bash
   tail -f ~/Library/Application\ Support/tech.yanjingai.app/logs/llama-helper.log
   ```
   期望看到 `n_batch       = 16384` + `n_ubatch      = 16384`(§223A 生效)
5. DB 验证:
   ```bash
   DB="$HOME/Library/Application Support/tech.yanjingai.app/meeting_minutes.sqlite"
   sqlite3 "$DB" "SELECT chunk_count, ROUND(processing_time,1), status, substr(error,1,200) FROM summary_processes WHERE meeting_id='meeting-c1299582-d80c-4d7d-972f-27e2ee3027d7' ORDER BY updated_at DESC LIMIT 1"
   ```
   期望: chunk_count=4, status=completed, error=空

## 教训 (§92 + §56 强化)

1. **任何 llama-cpp-2 / llama.cpp 参数改动必须 bundle binary 真跑一次 §15** — cargo check + guard 蒙混过不了真实 chunk 1 prompt 输入 + 生成第一 token 路径。c1299582 是 10K chars 1.5h 会议,只有真实长 prompt 才能完整测出 n_ubatch + KV Q4_0 corner case。
2. **'failed to eval' 是 catch-all 错误** — 真信息在 stderr (LLAMA_LOG_WARN 'failed to find a memory slot for batch of size N' / 'compute failed while preparing batch of size N')。macOS .app bundle stderr 必须 redirect 到文件才能 debug。
3. **§218 落库让用户能看到 chunk 1/4 failed** — 否则 §222 commit 会被误以为"修复成功"而实际 chunk 1 失败。从"完全没信息"到"看到 chunk 1/N failed"是巨大诊断改进。

## 关联章节

- §220 (LlamaBatch capacity 错配,修了 batch.add 容量)
- §222 (Qwen 2.5 3B context 8K→16K,n_ctx 修复,但 KV Q4_0 + N_UBATCH 漏)
- §215 (KV cache Q4_0,在 4K context 下工作,n_ctx=16K 失败)
- §194 (GENERATION_TIMEOUT_SECS 3600s)
- §197 (llama-cpp-2 0.1.146 baseline)
- §198 (caller-provided n_layer=36)
- §163 (sampling temp/top_p)
- §37 / §15 / §18 / §56 / §92 / §108 / §99.4
