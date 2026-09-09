# §220 LlamaBatch capacity 错配 — batch_size 必须 = n_batch (2026-09-09)

## 触发

§219 修复后用户 14:04 重跑 c1299582 摘要:

```
sqlite3 ... SELECT chunk_count, ROUND(processing_time,1), status, substr(error,1,300) FROM summary_processes ...
failed|0|0.0|Multi-level summarization failed: chunk 2/4 failed: Generation failed: Generation failed: Failed to add token to batch
```

**chunk 2/4 failed (之前 2/3)** — §219A chunk_size 2400→1800 让 10,112 chars 切成 4 chunks 而不是 3,数量对了。但 **错误完全一样**: Failed to add token to batch.

## §219B 真根因 — LlamaBatch capacity ≠ n_batch

§219B 我做的修改:
```rust
.with_n_batch(16384)  // context 单批 decode 上限 = 16K tokens
...
let batch_size = self.context_size as usize;  // 8192
let mut batch = LlamaBatch::new(batch_size, 1);  // capacity = 8192
```

**关键误解**: `with_n_batch(N)` 是 context 的物理 batch size (decode 时单次最多 N tokens),**不是** `LlamaBatch` 的 token 数组容量!

llama-cpp-2 0.1.146 实现:
```rust
pub fn new(n_tokens: usize, n_seq_max: i32) -> Self {
    let batch = unsafe { llama_batch_init(n_tokens_i32, 0, n_seq_max) };
    // llama_batch_init 分配 batch.token / batch.pos / batch.logits 数组, 大小 = n_tokens
}
```

而 `batch.add()` 的容量检查:
```rust
if self.allocated < n_tokens + 1 {  // self.allocated = n_tokens 参数
    return Err(BatchAddError::InsufficientSpace(self.allocated));
}
```

**所以**:
- `with_n_batch(16384)` → context decode 单批可处理 16K,但 **batch 对象本身 token 数组只有 8192 容量**
- batch.add 到 8193 仍 InsufficientSpace
- chunk 2 prompt = system (6776) + chunk (1800) + template (200) ≈ **8776 tokens > 8192 batch capacity**

**§219B 改错了地方**,容量错配!

## §220 修复 — LlamaBatch::new 必须用 N_BATCH

`llama-helper/src/main.rs`:
```rust
// §220 (2026-09-09): batch_size 必须 >= n_batch
const N_BATCH: u32 = 16384;
let ctx_params = LlamaContextParams::default()
    .with_n_ctx(Some(NonZeroU32::new(self.context_size).context("Invalid ctx size")?))
    .with_n_batch(N_BATCH)  // context 单批上限
    ...
    .with_type_k(KvCacheType::Q4_0)
    .with_type_v(KvCacheType::Q4_0);

let mut ctx = model.new_context(&self.backend, ctx_params)?;
...
// §220: LlamaBatch::new 第一个参数 = batch token 数组容量, 必须 >= n_batch
let batch_size = N_BATCH as usize;  // 16384, 跟 with_n_batch 一致
let mut batch = LlamaBatch::new(batch_size, 1);
```

**核心原则**: `LlamaBatch::new(n, ...)` 的 n 跟 `with_n_batch(N)` 的 N 必须满足 `n >= N`,否则 batch.add() 在 n_tokens > capacity 时 InsufficientSpace。

## §37 6 步硬闸门

- ✅ cargo check (llama-helper): 0 errors (1 §18 warning)
- ✅ check_historical_fixes.py: **768/768 PASS** (+2 §220 anchor, -1 §219 anchor)
- ✅ llama-helper build --release --features metal: 5.68s, binary 4.9M **sha e39e38434a87**
- ✅ sync_app_bundle.sh: §108 llama-helper + §98 codesign 全 sync (bundle binary 已更新)
- ⏳ frontend binary 没改 (不需要重 build)
- ⏳ GUI 端到端 (§15 强制, 用户必做)

## 铁律 (任何 v0.X 演进适用)

1. **`LlamaBatch::new(n, ...)` 的 n 跟 `with_n_batch(N)` 必须 n >= N** — 这是 llama-cpp-2 0.1.146 强约束, 不满足报 InsufficientSpace
2. **`with_n_batch(N)` 只是 context 单批上限** — 不影响 batch 对象本身 token 数组容量
3. **改 n_batch 必须同时改 batch_size** — 用同一常量 N_BATCH 避免漂移
4. **n_ctx 跟 N_BATCH 解耦** — N_BATCH 可以 > n_ctx (batch 容量),但 n_ctx < N_BATCH 时 decode 仍受 n_ctx 限制
5. **§219B 错就错在只改 with_n_batch 没改 batch_size** — §219B §219C 必须同日落地, 我偷懒只改了 §219B, 留 §220 修复

## 用户必做 (§15 GUI 验收)

```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```

1. 打开 meeting-c1299582 → 点 "重新生成"
2. 期望: 进度条跑 3-5 min, 4 chunks × ~30s ≈ 2 min 完成
3. DB 验证 (应 completed):
   ```bash
   DB="$HOME/Library/Application Support/tech.yanjingai.app/meeting_minutes.sqlite"
   sqlite3 "$DB" "SELECT chunk_count, ROUND(processing_time,1), status, substr(error,1,200) FROM summary_processes WHERE meeting_id='meeting-c1299582-d80c-4d7d-972f-27e2ee3027d7' ORDER BY updated_at DESC LIMIT 1"
   ```
   期望: `chunk_count=4, status=completed, error=NULL`

## 关联

- §219 (chunk_size 2400→1800 + n_batch 8K→16K, §219B 错配容量)
- §218 (chunk error 写 DB, 让真实根因全程可见)
- §215 / §216 / §217 (上一轮 context 优化)
- §197 (llama-cpp-2 0.1.146 baseline)
- §190 (qwen2.5:3b 默认)
- §163 (sampling temp/top_p)
- §194 (GENERATION_TIMEOUT_SECS 60min)
- §92 (决策迁移铁律)
- §15 / §37 / §18 / §56
- Obsidian: `~/Documents/Obsidian Vault/项目/3-离线会记/§220-...md`
- outputs: `outputs/§220-...md`

## commit

```
fix(§220): LlamaBatch capacity 错配 — batch_size 必须 = n_batch = 16384
```

## 教训 (§56 强化)

§219B commit 时只验证 `cargo check pass + guard pass`,**没端到端跑一次 c1299582 摘要**。
§15 铁律强制: **改 llama-helper 的 n_batch / batch_size 必须真跑摘要一次**, 不能 cargo check 蒙混。
§218 落库让用户能看到 chunk 2/4 failed, 否则我可能 §219B 就 close 掉了 (chunk 数量变了但错误还是 batch, 应该立刻怀疑 §219B 没改对地方)。
