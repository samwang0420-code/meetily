# §224 Metal buffer OOM — N_UBATCH 跟 N_BATCH 拆开 (2026-09-10)

## 触发事故

§223 binary (`sha=314a949e`,含 N_UBATCH=16384+KV F16+stderr redirect) 用户 11:35 触发:
- stderr log 显示 §223A 生效 (`n_ubatch=16384` 正确设置)
- llama-helper PID 8910 跑了 ~2 分钟,chunk 1 完成,chunk 2/4 失败
- error: `Sidecar closed stdout (process may have crashed)`

## 真根因

stderr log 末尾:

```
llama_context: n_ubatch      = 16384
llama_context: KV buffer size = 576.00 MiB (16384 cells, 36 layers)
sched_reserve: worst-case: n_tokens = 16384, n_seqs = 1
ggml_metal_buffer_init: error: failed to allocate buffer, size = 9496.00 MiB
```

**问题链**:
1. `n_ubatch=16384` 让 `sched_reserve worst-case n_tokens = n_ubatch = 16384`
2. Metal compute buffer 必须装下 n_ubatch tokens 对应的 graph
3. M3 8GB unified memory,**9.3 GB Metal buffer 分配失败**
4. llama-helper abort → SidecarManager 报 "Sidecar closed stdout"

**为什么 §223 错了**: 我把 N_UBATCH 跟 N_BATCH 一样设 16384(逻辑思维:"对称美"),但两者职责不同:
- `n_batch` = logical max, 决定 `LlamaBatch::new` token 数组容量 (caller side)
- `n_ubatch` = physical max, 决定 single-graph Metal compute buffer 大小 (callee side)

**§223A 是基于错误假设**:`n_batch % n_ubatch == 0` 必须成立 (GGML_ASSERT line 2691),但把两个都设成 16384 是合法的(16384 % 16384 = 0),只是物理 buffer 装不下。

## 修复

`llama-helper/src/main.rs`:
```rust
// §224 (2026-09-10): N_BATCH=16384 logical, N_UBATCH=512 physical
const N_BATCH: u32 = 16384;
const N_UBATCH: u32 = 512;

.with_n_batch(N_BATCH)   // logical capacity
.with_n_ubatch(N_UBATCH)  // physical graph size
```

`n_ubatch=512` 是 llama.cpp default (line 2891 `n_ubatch = 512`),H100/A100/M3 unified memory 8GB 都装得下。

`LlamaBatch::new(N_BATCH=16384, 1)` 保留 — 让 16K prompt 一次写入 batch token 数组 (§220 修复),不被 llama-helper Rust binding 容量限制。

## 验证

预期 stderr log:
```
llama_context: n_batch       = 16384
llama_context: n_ubatch      = 512
llama_context: KV buffer size = 576.00 MiB (16384 cells, 36 layers)
sched_reserve: max_nodes = 3480
ggml_metal_buffer_init: success (~2 GB)
```

## §37 6 步硬闸门 (commit fa595e2)

- ✅ cargo check --lib: 0 errors (17 §18 warnings 不动)
- ✅ cargo test --lib: 561 passed / 1 fixture-bound fail (§18)
- ✅ check_historical_fixes.py: **779/779 PASS** (+2 §224 anchor)
- ✅ llama-helper build: 4.88s, binary 4.9M (sha=ac96a68207c3)
- ✅ bundle llama-helper sha=`9148becff8343e8ac4edf39324b482c1c61c558c15271d5c578cf22e4366967d`
- ✅ sync_app_bundle.sh OK
- ⏳ GUI 端到端 (§15 强制,用户必做)

## §15 GUI 验收 (用户必做)

```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 进 c1299582 → 重新生成摘要

# 同时验证 stderr:
tail -100 ~/Library/Application\ Support/tech.yanjingai.app/logs/llama-helper.log | grep -E "n_batch|n_ubatch"
# 期望: n_batch=16384, n_ubatch=512
# 期望不再出现 'ggml_metal_buffer_init: error'
```

## 教训 (§92 + §56 强化,第 4 次)

1. **n_batch (logical) vs n_ubatch (physical) 区别必须明确** — 不能为"对称美"两个设一样
2. **Metal unified memory 8GB 是硬上限** — buffer > 7 GB 必失败
3. **cargo cache 可能跳过 build** — 之前 §222→§223 升级 cargo build 报告 OK 但 binary 没变(可能是 cargo fingerprint cache 错),必须 touch 强制重 build + sync 后**必须重启进程**(进程 in-memory 仍是旧 binary)
4. **stderr redirect 是 §223C 救命的功能** — 没有 §223C 我永远看不到 `ggml_metal_buffer_init: failed to allocate buffer, size = 9496.00 MiB` 这条关键信息,只能 blind debug

## 关联

- §223 (N_UBATCH=N_BATCH=16384 错,触发了本节 Metal OOM)
- §222 (n_ctx=16384 真根因,保留)
- §220 (N_BATCH=16384 logical,保留)
- §218 (chunk error 落 DB,让用户能看到 chunk 2/4 failed)
- §215 (KV Q4_0 在 n_ctx=16K 不稳定,改回 F16)
- §37 / §15 / §18 / §56 / §92

## 第 4 次失败时间线

| 章节 | 触发 | 错误信息 | 真根因 |
|---|---|---|---|
| §218 | 2026-09-09 09:00 | "No chunks were processed successfully" | chunk error 被吞 |
| §219 | 2026-09-09 12:43 | "Failed to add token to batch" | chunk_size 2400 + n_batch 8192 不足 |
| §220 | 2026-09-09 14:04 | "Failed to add token to batch" | LlamaBatch::new 容量错配 |
| §222 | 2026-09-09 14:05 | "Failed to add token to batch" | n_ctx 8192 < prompt 9074 tokens |
| §223 | 2026-09-10 10:38 | "failed to eval" | KV Q4_0 + n_ctx=16K 不稳定 + N_UBATCH 缺失 |
| §224 | 2026-09-10 11:35 | "Sidecar closed stdout" | N_UBATCH=16384 → Metal buffer OOM |
