# §222 Qwen 2.5 3B context 8K→16K — llama.cpp causal_attn 静默 cap n_batch (2026-09-09)

## 触发

§220 (commit 2eac721, 14:09) 修 LlamaBatch capacity 错配:
```rust
const N_BATCH: u32 = 16384;
.with_n_batch(N_BATCH)                  // context 单批 decode 上限
let batch_size = N_BATCH as usize;       // LlamaBatch token 数组容量
let mut batch = LlamaBatch::new(batch_size, 1);
```

§220 跑 bundle llama-helper 直接打 chunk 2 实际 prompt (~9000 tokens):

```bash
echo '{"type":"generate","id":"test","prompt":"...","max_tokens":50,...,"context_size":8192}' | llama-helper
# 输出:
# 📝 Tokenized prompt: 9074 tokens
# llama-context.cpp:1599: GGML_ASSERT(n_tokens_all <= cparams.n_batch) failed
```

**§220 修不够, 真根因是 llama.cpp causal_attn 静默 cap n_batch = min(n_ctx, params.n_batch)**。

## 根因 (3 层)

### Layer 1: llama.cpp n_batch 静默 cap
`/Users/wangwei/.cargo/registry/src/.../llama-cpp-sys-2-0.1.146/llama.cpp/src/llama-context.cpp:161`:

```cpp
// with causal attention, the batch size is limited by the context size
cparams.n_batch = cparams.causal_attn ? std::min(cparams.n_ctx, params.n_batch) : params.n_batch;
```

**Qwen 2.5 (causal attention) 触发 `min(n_ctx, params.n_batch)` 分支**:
- n_ctx = 8192 (来自 with_n_ctx(8192))
- params.n_batch = 16384 (来自 with_n_batch(16384))
- cparams.n_batch = **min(8192, 16384) = 8192** ← 静默 cap!

### Layer 2: batch.add() 检查用 LlamaBatch.allocated
```rust
if self.allocated < usize::try_from(self.n_tokens() + 1).expect(...) {
    return Err(BatchAddError::InsufficientSpace(self.allocated));
}
```

`LlamaBatch::new(16384, 1)` 设 `allocated = 16384`. batch.add() 调用 9074 次, 全部成功 (allocated 16384 够大).

### Layer 3: llama_decode 内部 GGML_ASSERT 触发
```cpp
GGML_ASSERT(n_tokens_all <= cparams.n_batch);
```

n_tokens_all = 9074 (实际 token 数), cparams.n_batch = 8192 (cap 后). 
**9074 > 8192 → GGML_ASSERT 失败 → panic**.

注意**不是** `Result::Err`, 是 panic! Rust async panic 被 tokio::spawn catch, 转 JoinError → "No chunks were processed successfully".

## §222 修复 — n_ctx 8192 → 16384

`frontend/src-tauri/src/summary/summary_engine/models.rs:213`:
```diff
-context_size: 8192, // §217: §215 4K 不够 10K-char meeting...
+context_size: 16384, // §222: §217 8K 不够 chunk 2 prompt 9074 tokens
+   // §220 修 batch_size=16384 但 llama.cpp causal_attn 静默 cap n_batch = min(n_ctx, params.n_batch)
+   // 必须 n_ctx >= prompt tokens. 16K 装得下 chunk (max ~9K) + 800 output = ~10K
+   // KV Q4_0 额外 0.08 GB (0.16 GB total), 8GB 仍 ok
```

测试更新:
```diff
-assert_eq!(qwen_3b.context_size, 8192); // §217
+assert_eq!(qwen_3b.context_size, 16384); // §222: 8K 不够 chunk 2 prompt 9074 tokens
```

## 验证 (实测)

llama-helper binary §222 修复后, bundle llama-helper 跑 c1299582 实际 chunk 2 prompt:

```
✅ Model loaded successfully
llama_context: n_batch       = 16384   ← §222 修后 = 16384 (不再是 8192)
📝 Tokenized prompt: 9074 tokens
🔄 Starting generation (max_tokens: 50)
✓ Reached max_tokens limit
📊 Generation Statistics:
   • Prompt tokens: 9074
   • Output tokens: 50
   • Prompt processing: 63.55s
   • Generation time: 58.67s
   • Total time: 122.22s
   • Speed: 0.85 tokens/sec
{"type":"done","text":" 以下是根据提供的转录片段提取的证据记录：\n\n- **...**"}
```

**成功生成! n_batch=16384, prompt 9074 < n_batch, decode 通过 GGML_ASSERT**.

## 内存影响 (M3 8GB)

KV cache Q4_0:
- n_ctx=8192: KV = 8192 * 2 (KV heads) * 128 (head_dim) * 36 (layers) / 2 (Q4_0 = 4-bit) = **75 MB**
- n_ctx=16384: KV = 16384 * 2 * 128 * 36 / 2 = **150 MB** (+75 MB)

M3 8GB:
- 模型 weights: 1.80 GB
- KV Q4_0 (16K): 0.15 GB
- app + system: ~1.5 GB
- llama-helper runtime: ~0.5 GB
- buffer: **~4 GB 可用**, 安全

## §37 6 步硬闸门

- ✅ cargo check --lib: 0 errors
- ✅ cargo test --lib models: PASS (context_size=16384 assertion)
- ✅ cargo build --release: 5m 27s (full rebuild after §218 chunk_text change)
- ✅ check_historical_fixes.py: **773/773 PASS** (+2 §222, 修 4 个 §215/§217 anchor regex)
- ✅ sync_app_bundle.sh: bundle llama-helper sha=eeb3556ac27e
- ⏳ GUI 端到端 (§15 强制, 用户必做)

## binary 状态

| 位置 | sha | mtime | size |
|---|---|---|---|
| `target/release/llama-helper` (src) | (待 shasum) | 14:50 | 4.9M |
| `target/release/言镜 AI.app/.../llama-helper` (bundle) | eeb3556ac27e | 14:52 | 4.9M |

## §15 GUI 验收 (用户必做)

```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 进 c1299582 → 重新生成摘要
# 期望: 30-50 min 完成 (n_batch=16384, chunk 2 prompt 9074 < 16384), 
4 status='completed', chunk_count=4
```

DB 验证:
```bash
sqlite3 "$HOME/Library/Application Support/tech.yanjingai.app/meeting_minutes.sqlite" \
  "SELECT chunk_count, ROUND(processing_time, 1) FROM summary_processes 
   WHERE meeting_id LIKE '%c1299582%' ORDER BY updated_at DESC LIMIT 1"
# 期望: chunk_count=4, processing_time > 1000 (大摘要)
```

## 铁律 (任何 v0.X 演进适用)

1. **llama.cpp causal_attn 静默 cap n_batch** — `with_n_batch(N)` 在 causal 模型下只生效到 n_ctx
2. **n_ctx 必须 >= max_chunk_prompt_tokens** — 7 system const + chunk_text + template + max_output
3. **每次改 n_ctx / n_batch 必须实测 prompt token 数** — 不能估算
4. **Rust panic ≠ Result::Err** — async panic 被 tokio catch 转 JoinError, 错误信息丢失
5. **§15 端到端强制** — cargo check + guard 不够, 必须 bundle binary 真跑一次
6. **§217 估的 6776 system token 是错的** — 实际 7058 tokens, 加上 chunk_text 1800 接近 9000
7. **修改 §X 描述前必读源码** — 避免凭直觉改错地方 (§220 batch_size 修了 batch 容量, 但没解决 n_batch cap)

## 已知边界 (按 §18 不动)

- 1 个 cargo warning (unused doc comment) 不动
- 17 个 warning (Q4_0 vs Q4_K 注释, §18 不动)
- tok/s 0.85 慢 (M3 CPU 推理 3B Q4_K + 9074 prompt 输入慢, 后续 §X 优化)

## 关联

- §220 (LlamaBatch capacity 错配, 修了 batch.add 容量不够)
- §217 (Qwen 2.5 3B context 8K, 当时估错了 prompt token 数)
- §215 (Qwen 2.5 3B context 32K→4K, KV Q4_0)
- §219 (chunk_size 2400→1800 + n_batch 8K→16K)
- §218 (chunk error 写 DB, 让用户能看到真实根因)
- §37 (硬闸门) / §15 (GUI 端到端) / §56 (AGENTS.md 双校) / §92 (决策迁移铁律)
- [[222-context-8K到16K-llama.cpp-causal-attn-cap]] (Obsidian)

## commit

```bash
git add frontend/src-tauri/src/summary/summary_engine/models.rs \
        scripts/check_historical_fixes.py \
        outputs/§222-context-8K到16K-llama.cpp-causal-attn-cap-2026-09-09.md
git -c user.email=codex@local -c user.name=codex commit -m "fix(§222): Qwen 2.5 3B context 8K→16K — llama.cpp causal_attn 静默 cap n_batch

§220 修了 LlamaBatch::new(16384, 1) batch.add 容量,但 llama.cpp causal_attn 
路径 cparams.n_batch = min(n_ctx, params.n_batch) 静默 cap 到 8192.
chunk 2 prompt 9074 tokens > 8192 → GGML_ASSERT(n_tokens_all <= cparams.n_batch)
panic → tokio::spawn catch → JoinError → 'No chunks were processed successfully'.

修: context_size 8192 → 16384, cparams.n_batch = min(16384, 16384) = 16384.
实测 bundle llama-helper n_batch=16384, prompt 9074 < 16384, 生成成功 (50 tokens).
KV Q4_0 8K→16K 增加 75 MB, M3 8GB 仍 ok.

§37 闸门:
- guard 773/773 PASS (+2 §222, 修 §215/§217 anchor regex 跟新值对齐)
- bundle llama-helper sha=eeb3556ac27e
- sync_app_bundle.sh 通过 _sync_copy.py 真同步

教训 (§220 强化): 改 n_batch / batch_size 必须实测 cparams.n_batch 实际值,
不能信 with_n_batch(N) 参数. llama.cpp causal_attn 路径会静默 cap 到 n_ctx."
```
