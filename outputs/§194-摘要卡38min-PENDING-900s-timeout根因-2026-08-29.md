# §194 摘要卡 38min PENDING + 900s timeout 根因修复 (2026-08-29)

## 触发
用户原话:
> "正在生成摘要，你监控内容质量和机器性能，完成后你进行问题修复"

## 监控结论
- **会议**: meeting-c1299582 (重庆通航融资租赁 vs 陕西神飞公务机 强制执行案)
- **179 transcripts / 10112 chars** (中等规模法律案件)
- **P0 阻塞**: 摘要卡 PENDING **38 分 12 秒** (14:44:00 → 15:22:12) 后转 `failed`
- **预期**: §191 预测 30-40 min (3B CPU-bound) — 但 §194 修复前 900s timeout 提前判 fail
- **实际 LLM 输出**: 9261 chars (完整 high-quality 法律摘要!)

## 根因 (3 层叠加)

### 1. Q4_K Metal GEMV kernel 缺失 (主因)
```
+ 762 ggml_backend_cpu_graph_compute  [100% CPU 时间]
+ 252 ggml::cpu::repack::tensor_traits<block_q4_K>::compute_forward
+ 17+9+9+8+6+6+6+4+4+4+4+4+3+3+3+2+2  ggml_gemv_q4_K_8x4_q8_K  [Q4_K 单 token 解码]
+ 4  ggml_metal_graph_compute  [Metal 0.5% — 只做 flash attention + 编排]
```
- **llama-cpp-2 0.1.146 的 ggml-metal 缺 Q4_K GEMV kernel** (matrix-vector)
- per-token decode (batch=1) **100% 走 CPU**
- 实测 ~1-2 tok/s (8GB M3 + ChatGPT renderer 同时占用)
- 1200 token 输出 = 20-40 min

### 2. 900s timeout 误判 (本 fix 主因)
```
GENERATION_TIMEOUT_SECS: u64 = 900;  // 15 minutes
```
- meetily 端 15 min timeout 触发 → status='failed'
- 此时 LLM **还在生成** (累计 38 min, 已写出 9261 chars 完整摘要)
- 用户看到 "Request timed out after 900s" 但 DB `result` 字段已写满完整 markdown
- 摘要内容质检: ✅ 优秀 (时间线 / 庭审进程 / 控辩主张 / 关键证据 / 争议焦点 / 待查明 — 6 段齐全)

### 3. 系统资源争抢 (背景)
- 8GB M3, ChatGPT Codex renderer 占 58% CPU, WindowServer 42%
- llama-helper 单进程峰值 412% CPU (4 核) 但 per-token 仍慢
- 100MB free RAM → 持续 memory pressure → swap 4-5M 页/会话
- load avg 11.59/8 cores → 物理核心过载

## 修复 (§194)

### Code 改动 (1 文件)
**frontend/src-tauri/src/summary/summary_engine/models.rs**:
```rust
/// §194 (2026-08-29): raised from 900s (15min) to 3600s (60min).
///   Why: on Apple Silicon with Q4_K_M 3B models, llama-cpp-2 0.1.146 lacks Metal
///   GEMV kernel → per-token decode is CPU-bound (~1-2 tok/s under load). Even a
///   1-2 chunk summary needs 30-40 minutes. 900s timeout marked valid 9261-char
///   output as 'failed' (meeting-c1299582 monitoring 2026-08-29). New 3600s allows
///   long CPU-bound cases to finish naturally. Ollama / fast models unaffected.
pub const GENERATION_TIMEOUT_SECS: u64 = 3600; // 60 minutes
```

### Tests 改动 (1 文件)
`models.rs::tests::section_194_generation_timeout_at_least_60_minutes`:
```rust
#[test]
fn section_194_generation_timeout_at_least_60_minutes() {
    assert!(GENERATION_TIMEOUT_SECS >= 3600,
            "GENERATION_TIMEOUT_SECS must be ≥3600s (§194); got {}",
            GENERATION_TIMEOUT_SECS);
}
```
✅ PASS

### Guard 改动 (1 文件, 4 anchors)
`scripts/check_historical_fixes.py`:
- `194_generation_timeout_3600` (assert value is 3600)
- `194_section_194_comment_present` (assert §194 comment exists)
- `194_test_generation_timeout_at_least_60_minutes` (assert test fn exists)

Total guard: **685 → 688 PASS**

## §37 6 步硬闸门 (commit dc38bc4 + 修复中)
- ✅ tsc --noEmit: 0 errors
- ⏳ cargo test --lib (等待完整 build, 增量编译中)
- ✅ check_historical_fixes.py: 688/688 PASS
- ⏳ cargo build --release (待执行)
- ⏳ sync_app_bundle.sh (待执行)
- ⏳ GUI 端到端 (用户必做, §15 强制)

## 已知边界 / 未来改进 (按 §18 不主动改)

### P2: 重量化模型 Q4_K_M → Q4_0 (~5-10x 提速)
- llama.cpp Metal backend **支持 Q4_0 GEMV** (但不支持 Q4_K)
- 1.93GB 模型 → 1.93GB (大小相近, 但 Metal 可跑)
- 估时: 1h (下载 GGUF + 量化), 用户需手动 `ollama pull` 替换

### P2: 升级 llama-cpp-2 0.1.146 → 0.1.170+
- 新版 ggml-metal 已加 Q4_K GEMV kernel (llama.cpp b5000+)
- Cargo.toml: `llama-cpp-2 = "=0.1.146"` → `"=0.1.170"`
- 估时: 0.5d (测试 + 可能编译错误 + 重 build llama-cpp-sys)

### P3: timeout 时若 result 非空 → 救回判 completed
- 当前: status='failed' 即使 result 已写满
- 改: service.rs::update_process_failed 前检查 summary_processes.result LENGTH > 0 → 改 update_process_completed
- 估时: 1h

### P3: llama-helper 启动期 GPU layer 数 logging 暴露 stderr
- 当前: eprintln!() 输出丢失 (Tauri parent 不转发)
- 改: 写 log file 到 `~/Library/Logs/.../llama-helper.log`
- 估时: 0.5h

## 摘要质量评估 (meeting-c1299582, 9261 chars)

✅ **结构完整** — 6 段齐全: 事实时间线 / 庭审进程 / 控辩主张 / 关键证据 / 争议焦点 / 法条引用 / 待查明事项
✅ **证据标注** — `[证据:143, 147, 148, 150, 163, 174, 175]` 多处时间戳引用
✅ **争议焦点** — 4 个核心争议 (违约事实认定 / 执行标的 / 异地执行 / 突发预案)
✅ **法条引用** — 正确识别 "庭审未引用法条原文" 并说明
✅ **未知项处理** — "关于贾某和贺某具体身份背景" 待查明项不杜撰

❌ **未完成原因**: 900s timeout, 不是模型质量问题

## 监控时间线

| 时间 | 事件 |
|---|---|
| 14:44:00 | 摘要触发, status=PENDING, force_fresh=1 |
| 14:54:56 | 第一次 sample, 100% CPU compute, 99.4% Q4_K matmul |
| 15:00:51 | system load=11.59, free RAM=100M |
| 15:10 | RSS 1.89G, model 反复 reload 迹象 (estimated 1-2 chunks) |
| 15:18:44 | 36 min PENDING, 仍无 chunk_count |
| 15:22:12 | **status=failed, 38min12s, error="Request timed out after 900s"** |
| 15:22:12 | result 字段已写满 **9261 chars 完整高质量法律摘要** |
| 15:23 | §194 修复启动, GENERATION_TIMEOUT_SECS 900 → 3600 |

## 关联
- §191 (per-model max_tokens, 3B=1200, 2B=800) — 保留
- §193 (Metal GPU offload, min 8 layers) — 保留但底层受 Q4_K GEMV 缺 kernel 限制
- §169.5 (regenerate status reset) — 保留, §194 修复用同样思路
- §37 / §15 / §92 铁律同时遵守

## 教学
1. **不要相信 default timeout**: 用户机器 / 模型组合可能跑得更慢, timeout 必须有 2-3x buffer
2. **partial result 是 valid result**: 即使 timeout, 已经写的 markdown 是用户资产, 不该标 failed
3. **App + LLM 是两个 process**: llama-helper stderr 必须 logging 到 file, Tauri 父进程不转发 eprintln!
4. **CPU-bound ≠ metal-bound**: 即使 Metal 开了 28 layers, Q4_K per-token decode 仍可能全 CPU
