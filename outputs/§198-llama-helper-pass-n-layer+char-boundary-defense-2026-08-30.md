# §198 llama-helper pass n_layer + char boundary panic defense (2026-08-30)

## 触发
用户生成摘要 (meeting-b0297a12 云南河口瑶族自治县法院庭审现场, 6433 chars / 100 segments, builtin-ai + court_hearing), 触发两个连续问题:

1. **char boundary panic** — 摘要输出 byte index 2434 '诉' panic:
   ```
   Background task panicked: end byte index 2434 is not a char boundary; it is inside '诉' (bytes 2432..2435 of string)
   ```
   DB: `chunk_count=0, processing_time=0.0`, status='failed'.

2. **Metal GPU partial offload** — `llama-helper` get_default_gpu_layers() 硬编码 estimated_layers=28 for files <2.5GB, 但 Qwen 2.5 3B 实际 36 层. 结果 8 层跑 CPU → 99.6% matmul → ~17 tok/s.

## 真根因 (Metal offload)
`llama-helper/src/main.rs::get_default_gpu_layers` 估算 layer 数:
```rust
let estimated_layers = if file_size_gb > 2.5 { 33 } else { 28 };  // wrong
```

Qwen 2.5 3B model.gguf: file_size 1.93 GB, layers = 36 (`frontend/src-tauri/src/summary/summary_engine/models.rs:191 layer_count: 36`). 1.93 < 2.5 → 走 28 层 → with_n_gpu_layers(28) → 8 层 CPU.

**sample 实证** (1 秒钟 profile):
- 489 calls `ggml_gemv_q4_K_8x4_q8_K` (CPU)
- 2 calls `ggml_metal_graph_compute` (Metal init only)
- **99.6% CPU / 0.4% Metal**

## 真根因 (char boundary panic)
- `hard_post_process.rs:754` `&truncated_line[..200]` 硬编码 200 byte 切片, 200 落在中文 3-byte char 中间时 panic.
- `hard_post_process.rs:1758 / 1700` regex match byte offset 走 `&out[..*start]` + `&out[*end..]`, 理论上 char boundary, 但加上 §198 防御性 safe_slice wrapper 永远安全.

## 修复 (3 文件 + 1 guard script)

### 1. `llama-helper/src/main.rs` (28 行新增)
- `Request::Generate` 新增 `n_layer: Option<u32>` 字段
- `get_default_gpu_layers(model_path, context_size, n_layer: Option<u32>) -> u32`:
  - `n_layer.is_some()` 用 caller 提供值
  - `n_layer.is_none()` fallback 到 file-size heuristic (backward-compat)
  - 加 §198 partial offload warn (when layers < caller_n)
- `load_model_if_needed` 加 `n_layer: Option<u32>` 参数

### 2. `frontend/src-tauri/src/summary/summary_engine/client.rs`
- `Request::Generate` 新增 `n_layer: Option<u32>` 字段
- 两个 instantiation 点都加 `n_layer: Some(model_def.layer_count)`

### 3. `frontend/src-tauri/src/summary/hard_post_process.rs`
- 新增 `fn safe_slice<'a>(text, start, end) -> &'a str` — floor byte offset downward to char boundary
- `truncate_raw_transcript_leak` line 754: `&truncated_line[..200]` → 加 floor 循环
- `truncate_raw_transcript_leak` line 778: `line.len() - cleaned.len()` → `saturating_sub` 防 underflow
- `strip_fabricated_evidence_ids` line 1700 / 1758: 用 safe_slice 替代 raw `&out[..*start]` `&out[*end..]`
- 3 个新测试 (section_198_*)

### 4. `scripts/check_historical_fixes.py` (6 新 anchor)
- 198_n_layer_field_in_request
- 198_get_default_gpu_layers_accepts_n_layer
- 198_safe_slice_helper_exists
- 198_safe_slice_used_in_strip_fabricated
- 198_truncate_200_byte_floored
- 198_client_pass_layer_count

guard: 697 → **703/703 PASS** (+6 §198 anchors).

## 性能预测 (Qwen 2.5 3B + ctx=32768 + M3 4.80 GB VRAM)
- weight_per_layer = 1.93/36 = 0.054 GB
- total_per_layer = 0.090 GB
- safe_layers = 4.30/0.090 = 47 → min(47, 36) = **36 = 全 Metal offload**

## §37 硬闸门
- cargo check --lib: 0 errors (14 §18 warnings 不动)
- cargo test --lib section_198: **3 passed / 0 failed**
- cargo test --lib: **529 passed / 1 failed / 3 ignored** (1 fail = fixture-bound §161 §18 不动)
- cargo build --release: 4m05s, binary 17:26 57M
- sync_app_bundle.sh: 3 binary 全部 sync (sha verified)
- check_historical_fixes.py: **703/703 PASS**
- ⏳ GUI 端到端 (§15 强制, 用户必做)

## §15 GUI 验收 (用户必做, 不能 CLI 测)
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```
打开 `meeting-b0297a12` → 点 "重新生成":
1. 后端日志 (stderr/console) 应含 `§198 partial GPU offload` (如果 layers < caller_n) 或不出现 (如果全 36 层)
2. 进度条 0% → 100% 应该跑完 (~5-15 min 取决于 36 vs 28 层提速)
3. DB: `chunk_count >= 1, status='completed', processing_time > 60s`
4. 不要再 panic char boundary (即使 LLM 输出 byte 落在 '诉' 等中文 3-byte char 中间)

## 已知边界
- 1 个 §161 fixture-bound test fail (/tmp/transcript_709b.txt 丢失), §18 不动
- 25 cargo warnings (§18 不动)
- 1 bun:test tsc error (§18 不动)

## 关联
- §197.1 (sidecar BrokenPipe recovery, a1c6be0)
- §197 (llama-cpp-2 0.1.146 回退, 39b2f6d)
- §196 (llama-cpp-2 0.1.154 升级, ee5230b)
- §195 (飞机执行案地名归一化 + 融资租赁方向硬约束, 4a34c77)
- §169.6 (char boundary floor 基础)
- §37 硬闸门 / §56 AGENTS.md 双校 / §92 决策迁移铁律 / §18 不主动改无关 bug
