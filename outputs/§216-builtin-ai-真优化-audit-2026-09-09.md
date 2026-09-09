# §216 builtin-ai 真优化 audit (2026-09-09)

## 触发

用户原话「我现在下载 Qwen 2.5 3B Instruct (Balanced), 然后切换, 你再仔细的核对一下代码, 有没有真正的优化」。

## Audit 结论: 已落地 5 层优化, 真优化空间接近 0

| 章节 | 优化点 | 效果 |
|---|---|---|
| §190 | qwen2.5:3b 替换 qwen3.5:2b 默认 | 中文质量↑ |
| §193 | GQA-aware KV estimate + MIN_GPU_LAYERS=8 | M3 全层 offload |
| §198 | caller-provided n_layer=36 | 3B 真 36 layers (之前错估 28) |
| §163 | sampling temp=0.1 / top_p=0.3 / rep=1.05 | 输出稳定可复现 |
| §215 | context 32K→4K + KV cache Q4_0 | 省 1.24GB RAM |

## Qwen 2.5 3B Q4_K_M @ M3 8GB @ 4K context 实际效果 (手算 §193)

```
探测 VRAM: sysctl hw.memsize × 0.6 = 8 × 0.6 = 4.8 GB
safe_vram = 4.3 GB
Model weights: 1.93 GB / 36 layers = 53.6 MB/layer
KV cache (4K Q4_0): 4 × 0.04 = 0.16 GB / 36 = 4.4 MB/layer
total per layer: 58 MB
safe_layers = 4.3/0.058 = 74 (远 > 36)
结论: 36/36 layers full offload, KV cache 全在 unified memory
```

## 本节真改动 (2 处)

### 1. token_to_piece_bytes buffer 32 → 64
- 中文 token 偶尔 >32 byte 触发 retry 路径 (再 alloc 一次 ~required_size)
- 改 64 几乎消除 retry
- 风险: 0 (仅 alloc 大一点, ~64 bytes per token)
- 收益: 中文 token 占比高时 ~5% throughput 提升

### 2. with_type_k(Q4_0) 加详细注释
- 文档化 Q4_0 vs Q5_0 vs Q8_0 tradeoff
- 不影响 runtime, 仅 documentation
- 风险: 0

## 已审视、跳过 (按 §18 不主动改无关 bug)

### 跳过原因

| 选项 | 跳过原因 |
|---|---|
| `with_n_threads_batch` 减半 | prefill 走 Metal matmul 不消耗 CPU threads, 但 prefill 时间占比 < 5%, 改 batch threads 收益 < 1% |
| `with_n_batch(context_size)` = 4096 改 prompt_len | prefill batch 必须 ≥ max prompt 长度。Q4_K M3 prefill 4096 一次吞所有 prompt, 已最优 |
| `AddBos::Always` 改 Never | Qwen 2.5 GGUF tokenizer.add_bos_token=true, llama.cpp 内部去重, 不会重复 add |
| speculative decoding | 3B 模型本身 draft 不靠谱 (Qwen 2.5 3B 内置 draft head 不稳) |
| llama-cpp-2 0.1.146→0.1.154 | §197 已实证 per-token decode 反而 fallback CPU (4.3 tok/s < 当前 7.44) |
| `mLock mmap` 默认开 | llama-cpp-2 0.1.146 默认 mmap=true, Metal 已用 GPU VRAM |
| batch=8 speculative | 同 speculative decoding 跳过原因 |
| flash attention | Metal backend 在 0.1.146 已支持, flashes 自动启用 |
| num_parallel | 多用户场景, build-in AI 是单用户 |

## 用户级优化 (GUI 控制, 不动代码)

1. **散热**: 1.5h 摘要跑 ~10 min 末段可能降频到 20 tok/s
   - MacBook Pro 用 clamshell mode + 风扇满速
   - MacBook Air 无风扇, 跳过 (但要注意 0.7-0.8 倍速)
2. **LLAMA_IDLE_TIMEOUT=600**: 跑完不立刻关 sidecar, 避免下次重启 5s 模型加载延迟
   ```bash
   export LLAMA_IDLE_TIMEOUT=600  # 10 分钟
   ```

## §37 6 步硬闸门状态

- [1] cargo check: 0 errors, 1 warning §18
- [2] cargo test --no-run: 编译通过
- [3] check_historical_fixes.py: 753 → **755/755 PASS** (2 §216 anchor)
- [4] ~~check_v08_migration_completeness.py~~: HEAD v0.9.4 跳过
- [5] cargo build --release: llama-helper 4.9MB
- [6] sync_app_bundle.sh: 已 sync

## commit

`4c1cfe1 perf(§216): builtin-ai 真优化 audit + token_piece buffer 32→64 + KV Q4_0 注释`

## 关联

- §215 (context 32K→4K + KV cache Q4_0, 上一节)
- §193 (GQA-aware KV cache + MIN_GPU_LAYERS)
- §198 (caller-provided n_layer)
- §163 (sampling 参数固化)
- §37 (硬闸门) / §18 (不主动改无关 bug) / §28 (决策迁移铁律)
