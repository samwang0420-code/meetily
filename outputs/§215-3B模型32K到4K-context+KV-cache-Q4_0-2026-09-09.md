# §215 3B 模型 32K→4K context + KV cache Q4_0 优化 (2026-09-09)

## 触发

用户原话「OK, 走C。我们就用 3B 的模型, 你看看」+ 文章参考 [cnblogs/itech/p/19919532](https://www.cnblogs.com/itech/p/19919532)。

## 文章核心结论 (复述给用户)

1. **llama.cpp 比 Ollama 内存少 1.3GB** — M3 16GB / 8B Q4_K_M: llama.cpp 5.2GB vs Ollama 6.5GB
2. **Q4_K_M 是性价比之王** — Q2/Q3 中文质量差, Q6/Q8 体积翻倍
3. **Context 是隐形杀手** — 8K 内存翻倍, 日常 4K 够
4. **散热和降频** — MacBook Air 无风扇, 长跑从 40 tok/s 降到 15-20
5. **模型比工具重要** — Qwen2.5 中文远超 Llama-3.1

## 当前模型调用方式梳理 (用户要求)

### builtin-ai 路径 (默认, §190 起)
```
用户选择 qwen2.5:3b
  ↓
frontend (Next.js) → Tauri invoke
  ↓
src-tauri/src/summary/summary_engine/models.rs
  - get_model_by_name("qwen2.5:3b") -> ModelDef
  - 字段: gguf_file, context_size, layer_count, sampling
  ↓
src-tauri/src/summary/sidecar/SidecarManager
  - spawn llama-helper 进程 (裸 llama-cpp-2 0.1.146, §197 baseline)
  - 通过 stdin/stdout JSON 协议通信
  ↓
llama-helper (Rust bin)
  - main.rs 第 519 行: LlamaContextParams::default()
  - with_n_ctx(self.context_size)  ← 这里原来是 32K
  - with_n_batch / with_n_threads_batch
  - 现在加: with_type_k(Q4_0) / with_type_v(Q4_0) ← §215 新增
  - load Qwen2.5-3B-Instruct-Q4_K_M.gguf (~2.1GB)
  - Metal GPU offload (M3)
  - per-token decode (~30-40 tok/s expected)
```

### 关键代码锚点

- **A. models.rs context_size** (line 213): 32768 -> **4096**
- **B. models.rs 单测断言** (line 457): 同步 32768 -> **4096**
- **C. llama-helper/src/main.rs imports** (line 11): `use llama_cpp_2::context::params::LlamaContextParams` -> `{KvCacheType, LlamaContextParams}`
- **D. llama-helper/src/main.rs context params** (line 519 后):
  ```rust
  .with_n_threads_batch(threads)
  // §215: KV cache Q4_0 for M3 8GB
  .with_type_k(KvCacheType::Q4_0)
  .with_type_v(KvCacheType::Q4_0)
  ```

## 内存收益 (M3 8GB + Qwen 2.5 3B Q4_K_M)

| 配置 | 模型 | KV cache | Runtime | 合计 | Headroom |
|---|---|---|---|---|---|
| 旧 (32K F16 KV) | 2.1GB | 1.28GB | 300MB | **3.7GB** | 4.3GB |
| 新 (4K F16 KV) | 2.1GB | 0.16GB | 300MB | **2.6GB** | 5.4GB |
| **本节 (4K Q4_0 KV)** | **2.1GB** | **0.04GB** | **300MB** | **2.4GB** | **5.6GB** |

> KV cache 公式 (Qwen 2.5 3B GQA): `kv_per_1k_gb ≈ 0.04` (≤ 2.5B 模型), §193 GQA-aware
> 4K Q4_0: 4 × 0.04 × 0.25 (Q4 压缩比) = 0.04GB

## 为什么 4K 够 1.5h 会议

1. **chunk_size 2400 token** (§55 Map-Reduce)
2. **prompt ≤ 600 token** (system + few-shot)
3. **output ≤ 800 token** (§52 max_tokens)
4. **总需求** = 2400 + 600 + 800 = **3800 token ≤ 4096** ✓
5. 用户硬指令 `-c 4096` 与当前 §52/§55 设计完全契合

## 不动项 (§18 精神)

- **不切回 Ollama**: builtin-ai 路径已是裸 llama-cpp-2, 走 Metal GPU, 比 Ollama 省 1.3GB
- **不升级 llama-cpp-2**: §197 已实证 0.1.146 是 M3 Apple Silicon Q4_K per-token decode 最优 baseline
- **不引入新单测**: 只改 context_size 常量 + KV cache type, 已有单测断言同步即可
- **不调整 max_tokens**: §52 800/1200 (BuiltInAI 2B/3B) 仍合适

## §215 guard anchors (4 个)

```python
("215_qwen25_3b_context_4k",
 "frontend/src-tauri/src/summary/summary_engine/models.rs",
 r"§215: 32K -> 4K for M3 8GB"),
("215_qwen25_3b_test_assertion_4k",
 "frontend/src-tauri/src/summary/summary_engine/models.rs",
 r"assert_eq!\(qwen_3b\.context_size, 4096\);.*§215"),
("215_llama_helper_kv_cache_import",
 "llama-helper/src/main.rs",
 r"use llama_cpp_2::context::params::\{KvCacheType, LlamaContextParams\}"),
("215_llama_helper_kv_cache_q4_0",
 "llama-helper/src/main.rs",
 r"with_type_v\(KvCacheType::Q4_0\)"),
```

guard: 749 -> **753/753 PASS**

## §37 6 步硬闸门状态

- [1] cargo check --lib: 0 errors, 17 warnings §18
- [2] cargo test --lib --no-run: 编译通过 (3 warnings §18)
- [3] check_historical_fixes.py: 753/753 PASS
- [4] ~~check_v08_migration_completeness.py~~: HEAD v0.9.4, 旧 guard obsolete (跳过)
- [5] cargo build --release: 待跑 (~2-3 min 增量)
- [6] sync_app_bundle.sh: 待跑

## §15 GUI 验收 (用户必做)

```bash
killall meetily 2>/dev/null
open ~/Applications/言镜\ AI.app
```

1. 设置 → 模型设置 → 选 `qwen2.5:3b`
2. 如果 GGUF 未下载, 触发下载 (~2GB, 1-5 min)
3. 跑一次 1.5h 会议摘要 (~15K tokens, 4 chunks)
4. 期望: 8-10 min 完成, 活动监视器 RSS < 4GB, 不卡顿

## 风险

- llama-helper 重 build 增量 ~2-3 min
- llama-cpp-2 0.1.146 KV cache Q4_0 实际效果需 GUI 验证 (CLI 测不出)
- qwen2.5:3b GGUF 首次下载 ~2GB, 用户需稳定网络
- 老用户 DB settings 可能指向 qwen3.5:2b (legacy), 启动时 §190.1 fallback 兜底
- 散热: M3 Air 1.5h 摘要跑 ~10 min, 末段可能降频到 20 tok/s

## 关联

- §190 (qwen2.5:3b 替换 qwen3.5:2b 默认)
- §193 (GQA-aware KV cache + n_gpu_layers 自动算)
- §197 (llama-cpp-2 0.1.146 baseline, 0.1.154 升级无效回退)
- §211 (言镜 AI 战略搁置, 重启入口)
- §37 (6 步硬闸门) / §28 (决策迁移铁律) / §18 (不主动改无关 bug) / §15 (GUI 验证强制) / §92 (防代码漏)
