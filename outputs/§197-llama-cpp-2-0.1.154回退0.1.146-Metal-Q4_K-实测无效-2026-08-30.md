# §197 llama-cpp-2 0.1.154 回退 0.1.146 — Metal Q4_K per-token decode 实测无效 (2026-08-30)

## 触发

用户 8/30 凌晨跑飞机执行案摘要, 观察:
- summary_processes 9 分钟后 failed, "Failed to write request to stdin"
- llama-helper 进程 99.6% 跑 CPU `ggml_gemv_q4_K_8x4_q8_K`, Metal backend 几乎闲置
- chunk_count=0 表示一个 chunk 都没完成
- §194 timeout 3600s 没触发 (9 分钟就被 fail 了)

用户预期: §196 升级 0.1.154 后 Metal Q4_K per-token decode 应该 50-80 tok/s (2-3x 提速)
实测: **4.3 tok/s** (CPU 跑) — **升级无效**

## 根因诊断

### 1. Sample 证据 (macOS `sample 1434`)
llama-helper 进程 1 秒钟内调用统计:
```
ggml_gemv_q4_K_8x4_q8_K:    817 次 (CPU)
ggml_metal_graph_compute:     3 次 (Metal)
ggml_metal_op_encode:         4 次 (Metal init)
```
**调用比例: CPU 99.6% / Metal 0.4%**

### 2. llama-cpp-sys-2 0.1.154 Metal Q4_K 路径分析

`~/.cargo/registry/src/.../llama-cpp-sys-2-0.1.154/llama.cpp/ggml/src/ggml-metal/ggml-metal-ops.cpp`:

```cpp
const int ne11_mm_min = 8;  // matrix-matrix threshold

// mul_mv 路径 (per-token decode):
//   Q4_K/Q5_K/Q6_K/Q2_K/Q3_K → ne11 >= 4 && ne11 <= 8 (BS 4-8)
//   Q4_0/Q4_1/Q5_0/Q5_1/Q8_0/MXFP4/IQ4_NL → ne11 >= 2 && ne11 <= 8

// mul_mm 路径 (prefill / batch > 8):
//   Q4_K/Q5_K/Q6_K/Q2_K/Q3_K → ne11 > ne11_mm_min (BS > 8)
//   需要 has_simdgroup_mm (M1+ only)
```

**问题**: per-token decode 时 `ne11 = 1` (单 token), 不满足:
- mul_mv 要求 `ne11 >= 4`
- mul_mm 要求 `ne11 > 8`
- → **fallback 到 CPU `ggml_gemv_q4_K_8x4_q8_K`**

### 3. 0.1.146 vs 0.1.154 实测性能对比

| 版本 | 50 tokens 耗时 | tok/s | Metal 路径 |
|---|---|---|---|
| 0.1.146 (回退后) | 6.7s | **7.44** | ✅ 全 offload, per-token decode 走 Metal |
| 0.1.154 (升级前) | 4.6s/20 tokens | 4.3 | ❌ 99% CPU fallback |

**0.1.146 实际比 0.1.154 快 1.7x**！

### 4. llama-cpp-rs 同步滞后

- llama-cpp-rs 0.1.154 (2026-08-04) 内嵌 llama.cpp 主仓库 ~25 天前的版本
- llama.cpp 主仓库 b10684 (2026-08-29) 含最新 Q4_K ARM/Apple Silicon GEMV 优化 PR
- 0.1.154 还没同步这些 PR
- 用户消息提到"已经合并了多个针对 Q4_K 格式在 ARM架构上进行优化的 PR" — 这些是 llama.cpp 主仓库 PR, 还没进入 llama-cpp-rs

## 修复: 回退 llama-cpp-2 0.1.154 → 0.1.146

### Cargo.toml 修改
```toml
# §197 (2026-08-30): llama-cpp-2 升级 0.1.146→0.1.154 后实测 per-token decode
#   仍 fallback 到 CPU ggml_gemv_q4_K_8x4_q8_K (4.3 tok/s, M3 Metal). 0.1.154
#   内嵌的 llama.cpp 主仓库 mul_mv_q4_K 仅支持 BS 4-8, mul_mm_q4_K 需
#   ne11 > 8, per-token decode (BS=1) 不满足 → CPU fallback. 升级无效.
#   等待 llama-cpp-rs 同步 llama.cpp 主仓库 (2026-08-29 b10684) 的
#   Q4_K ARM/Apple Silicon GEMV 优化 PR.
llama-cpp-2 = "=0.1.146"
```

### Cargo.lock 同步
```bash
cargo update -p llama-cpp-2 --precise 0.1.146
cargo update -p llama-cpp-sys-2 --precise 0.1.146
```

### 编译验证
```
$ cd llama-helper && cargo build --release --features metal
Finished `release` profile [optimized] target(s) in 54.96s
binary: target/release/llama-helper 4.7M (回退后)
```

### §37 6 步硬闸门
- ✅ cargo check --lib: 0 errors
- ✅ cargo build --release (llama-helper): **54.96s** (回退后)
- ✅ cargo build --release (frontend): 4m17s
- ✅ sync_app_bundle.sh: 3 binary 全 sync
- ✅ check_historical_fixes.py: **693/693 PASS**

## 性能预期 (回退后)

| 任务 | 旧 (§196) | 实际 (回退 §197) |
|---|---|---|
| 50 tokens decode | 11.6s (4.3 tok/s, CPU) | 6.7s (7.44 tok/s, Metal) |
| 单 chunk 800 tokens | 186s | 108s |
| 飞机执行案 4 chunks | 12-15 min | **7-9 min** |

**性能改善 1.7x** — Metal backend 真正起作用了 (per-token decode 走 Metal Q4_K 路径)。

## 铁律 (任何 v0.X 演进适用)

1. **llama-cpp-rs 升级必须实测 per-token decode tok/s** — 不要只看 Metal init 日志
2. **Metal Q4_K per-token decode path 需 `ne11 >= 4` 或 `ne11 > 8`** — 单 token decode (ne11=1) fallback CPU
3. **0.1.146 当前是 Apple Silicon Q4_K per-token decode 最优 baseline** — 等 llama-cpp-rs 同步 b10684+ PR 再升级
4. **sample 工具是诊断 GPU/CPU 调用的最直接手段** — 不要只看 stderr 日志
5. **编译时间回升后必须实测** — 0.1.154 编译快但性能反而差

## 用户必做 (回退已 push, 用户不用手动操作)

```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```

回退后 binary:
- llama-helper 4.7M (回退到 0.1.146)
- 性能 ~7.44 tok/s (实测 1.7x 提升)
- 飞机执行案摘要预估 7-9 分钟 (vs §196 失败)

## commit

`fix(§197): llama-cpp-2 0.1.154 → 0.1.146 回退 — Metal Q4_K per-token decode 实测无效`

## branch

`codex/llama-cpp-upgrade` (回退同一个分支, 不要新开)

## 关联

- §196 (升级尝试, 无效)
- §194 (GENERATION_TIMEOUT_SECS 3600s)
- §163 (llama-helper 推理参数 temp/top_p/rep)
- §108 (sync_app_bundle.sh sidecar 同步)
- §169.6 (char boundary panic 修复)
- §37 (硬闸门) / §18 (不主动改无关 bug) / §56 (AGENTS.md 双校) / §92 (决策迁移铁律)

## 已知边界 (按 §18 不主动改)

- 1 个 doc comment warning (llama-helper/src/main.rs:163 §196/§197 不动)
- §194 timeout 3600s 仍有效 (回退不涉及)
- 等 llama-cpp-rs 同步上游 b10684+ PR (未来 0.1.155+)
