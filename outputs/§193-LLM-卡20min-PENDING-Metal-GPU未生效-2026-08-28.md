# §193 LLM 摘要卡 20min PENDING + Metal GPU 未生效 (2026-08-28)

## 触发
用户原话:
> "正在生成摘要，你监控内容质量和机器性能，完成后你进行问题修复"

## 监控结论
- **P0 阻塞**: 任务 `meeting-8ce922f9` (高压触电致人损害责任纠纷案件 regenerate) 卡 PENDING 21.5 分钟
- **预期**: §191 预测 1-2 分钟 (Metal GPU 28 layers full offload)
- **实际**: 1.4 tok/s, 完全 CPU decode, Metal GPU 只占 2.7%

## 根因 (sample 实测)

```
+ 1387 LlamaContext::decode                      [100% in decode]
+ 1375 ggml_backend_cpu_graph_compute           [99% CPU 后端]
+ 985  ggml_graph_compute_thread
+ 758  ggml_compute_forward_flash_attn_ext      [Flash Attn 在 CPU!]
+ 549  block_q4_K::compute_forward              [Q4_K matmul 在 CPU]
+ 55   com.apple.Metal.CommandQueueDispatch     [Metal 只占 2.7%]
```

## 为什么 Metal 没生效

`llama-helper/src/main.rs::calculate_gpu_layers` (line 205-272) 的启发式:

```rust
let kv_per_1k_gb = if file_size_gb > 2.5 { 0.25 } else { 0.12 };
let total_kv_gb = (context_size as f32 / 1000.0) * kv_per_1k_gb;
```

`models.rs::qwen2.5:3b.context_size = 32768` (§190 立), Q4_K_M 1.93GB:

- 估算 `total_kv_gb = 32.768 * 0.12 = 3.93 GB` (按 dense 模型估)
- 但 **Qwen 2.5 3B 用 GQA** (Grouped Query Attention), KV cache 实际 ~0.04 GB/1K (1/3 of dense)
- `safe_vram = 4.0 - 0.5 = 3.5 GB` < `total_kv_gb (3.93 GB)` → 算出的 layers 不可信
- Metal 路径几乎不走 → 全 CPU decode

**`detect_metal_vram` 也有问题**: 用 `hw.memsize * 0.6` (即 8GB × 0.6 = 4.8GB), 但 Apple Silicon 统一内存, M3 8GB 机器 GPU 能用 ~3.5GB 已经是极限.

## 性能对比

| 场景 | tok/s | 33 分钟录音预期时间 |
|---|---|---|
| §191 预测 (Metal 28 layers) | 30+ | ~1-2 min |
| 实际 (1.4 tok/s CPU) | **1.4** | **~20+ min** |

**20 倍 regression**.

## 同时发现的 3 个次要问题

1. **`end_time` 未重置** (§169.5 修复不完整) — 残留 `2026-08-28 12:03:57`, 应该是 NULL
2. **`/tmp/run_build.sh` 孤儿 cargo build** 反复触发 — 8/28 10:25 创建, 14:50 + 14:58 跑过两次, 父 PID 1 (init orphan). 已 mv 到 `.DELETED`
3. **`/tmp/build_rel3.log`** 持续写入 — 已清理

## 提议修复

### Fix 1 (§193 P0): 重新校准 KV cache 估算
```rust
// llama-helper/src/main.rs::calculate_gpu_layers
let kv_per_1k_gb = if file_size_gb > 5.0 {
    0.25  // Llama 7B+, dense
} else if file_size_gb > 2.5 {
    0.15  // 中型 dense
} else {
    0.04  // 1B-3B GQA (Qwen 2.5 3B 用 GQA!)
};
```

### Fix 2 (§193 P0): 强制最小 GPU layer
```rust
// 即使算出来 0, 也至少放 8 layers 在 Metal
// Metal 即使 1 layer 也比 CPU 快
let layers = safe_layers.max(8).min(model_layers);
```

### Fix 3 (§193 P0): end_time 重置 (§169.5 补充)
```rust
// database/repositories/summary.rs::create_or_reset_process
UPDATE summary_processes SET
    status = 'PENDING', result = NULL, end_time = NULL,
    chunk_count = 0, processing_time = 0.0,
    updated_at = ?
WHERE meeting_id = ?
```

### Fix 4 (§193 P1): /tmp 残留 build script 清理 + 锁
- 删 `/tmp/*.sh` 残留 build script (run_build.sh, run_187.sh, run_test.sh, run_cargo.sh)
- 删 `/tmp/build_*.log` `/tmp/build_*.done`
- 加 cron 守护禁止 /tmp/run_*.sh 反复触发

## 当前状态

- LLM 还在跑 (21.5 min PENDING, 1.4 tok/s, CPU 570%)
- DB 未更新 (chunk_count=1, result='', updated_at=14:51:34)
- §191 commit 在位 (binary mtime 13:07-14:50), Metal framework linked, 但实际 Metal GPU 路径不走

## 关联
- §191 (Metal GPU offload 立了但未生效)
- §169.5 (status 重置, 漏了 end_time)
- §190 (qwen2.5:3b context_size=32768 触发估算超限)
- §18 (云端 API 永不接入)
- §37 (硬闸门)
- §15 (GUI 端到端验证, 用户必做)
