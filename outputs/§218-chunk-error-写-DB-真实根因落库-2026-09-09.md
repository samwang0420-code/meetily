# §218 chunk error 写 DB — 真实根因落库 (2026-09-09)

## 触发

用户 9/9 重跑 meeting-c1299582-d80c-4d7d-972f-27e2ee3027d7 (43 min / 179 transcripts / 10,290 chars):

```
sqlite3 "$DB" "SELECT status, chunk_count, substr(error,1,250) FROM summary_processes WHERE meeting_id='meeting-c1299582-...' ORDER BY updated_at DESC LIMIT 1"
failed|0|Multi-level summarization failed: No chunks were processed successfully.
```

§215 (context 32K→4K + KV cache Q4_0) + §216 (token_piece buffer 64) + §217 (context 4K→8K) 都做了,用户重启 app + 重跑仍失败,但 DB error 信息永远是空泛的 "No chunks were processed successfully"。

## 真根因

`processor.rs:1075` Map 阶段 chunk error 被吞:

```rust
Ok((i, Err(e))) => {
    if e.contains("cancelled") { cancel_error = Some(e); break; }
    error!("Failed processing chunk {}/{}: {}", i + 1, num_chunks, e);
    // ↑ 只 log, 不保存
}
```

最终 `if chunk_summaries.is_empty()` 路径返回硬编码字符串,跟真实 LLM 错误完全无关。

用户每次重跑后只能看到 "No chunks processed",排查路径 0:
- 不知道是 llama-helper 启动失败
- 不知道是 stdin/stdout pipe 断
- 不知道是 prompt+chunk 拼起来超 context (虽然 §217 改 8K)
- 不知道是 GGUF 模型加载失败
- 不知道是 timeout 截断

## 修复 (3 处)

`frontend/src-tauri/src/summary/processor.rs`:

### 1. 加 first_chunk_error 变量
```rust
let mut cancel_error: Option<String> = None;
// §218: 收集第一个 chunk 错误, 在所有 chunk 都失败时把真实根因落 DB
// 之前只 log 不保存, DB summary_processes.error 永远是 "No chunks were processed successfully"
// 用户重启后只能看到空泛错误, 排查路径 0
let mut first_chunk_error: Option<String> = None;
let mut chunk_summaries: Vec<Option<String>> = vec![None; chunks.len()];
```

### 2. Err 分支保存第一个错误
```rust
Ok((i, Err(e))) => {
    if e.contains("cancelled") {
        cancel_error = Some(e);
        break;
    }
    error!("Failed processing chunk {}/{}: {}", i + 1, num_chunks, e);
    if first_chunk_error.is_none() {
        first_chunk_error = Some(format!(
            "chunk {}/{} failed: {}",
            i + 1,
            num_chunks,
            e
        ));
    }
}
```

设计决策: 只保存**第一个**错误。多个 chunk 失败时,第一个通常是 LLM 真实错误(超时 / pipe 断 / context 超),后续错误通常是连锁(sibling chunks 等 LLM 占用)。这样保持排查路径稳定。

### 3. 最终 Err 用真实信息
```rust
if chunk_summaries.is_empty() {
    let detail = first_chunk_error
        .as_deref()
        .unwrap_or("No chunks were processed successfully");
    return Err(format!("Multi-level summarization failed: {}", detail));
}
```

fallback 保留原始 "No chunks processed" 文案 — 万一 Err 路径完全没进(例如 join_error),用户至少看到熟悉的提示。

## §37 6 步硬闸门

- ✅ cargo check --lib: 0 errors (17 §18 warnings)
- ✅ cargo test --lib p218_chunk_error: **3/3 PASS**
  - `section_218_first_chunk_error_format_includes_chunk_index` — 验证 chunk 编号 + 真实 error
  - `section_218_error_takes_first_when_multiple_chunks_fail` — 验证多 chunk 失败时只保留第一个 (跳过 cancelled)
  - `section_218_fallback_error_when_no_chunk_error_captured` — 验证 fallback 文案
- ✅ check_historical_fixes.py: **762/762 PASS** (+4 §218 anchor)
- ✅ cargo build --release: **9m33s** (全 build, §211 target/ 清理后), binary 59.8M mtime 13:28
- ✅ sync_app_bundle.sh: §93 main + §98 codesign + §99.6 tauri bundle + §108 sidecar 全部 sync
  - `target/release/言镜 AI.app/Contents/MacOS/言镜 AI` = codesigned, sha f7342088d9a4
  - `target/release/meetily` (未签源) = e551bf872cf5
  - sha 不同是 codesign --force --deep --sign - 改 binary 内容的正常结果
- ⏳ GUI 端到端 (§15 强制, 用户必做)

## 铁律 (任何 v0.X 演进适用)

1. **Map 阶段 chunk error 必须落 DB** — 永远不让用户看到 "No chunks were processed successfully" 这种空泛错误
2. **保留第一个错误,跳过 cancelled** — 第一个 chunk 错误最有信息量,后续通常是连锁
3. **fallback 文案必须保留** — 旧用户 / 老 session 兼容性
4. **§218 类型修复必须加单测** — p218_chunk_error_written_tests 3 个 case 永久守护
5. **下游 handler 不需要改** — Err path 透传上游, `update_process_failed` 已经把 error 写 DB (§152)

## 用户必做 (§15 GUI 验收)

```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```

1. 打开 meeting-c1299582 (重庆通航融资租赁 vs 陕西神飞) → 点 "重新生成"
2. 期望 1: 进度条正常跑 (§217 8K context 容得下 10K-char meeting prompt+chunk+output)
3. 期望 2 (如果还是失败): DB error 字段不再是空泛 "No chunks processed",而是含 `chunk N/M failed: <真实 LLM/llama-helper 错误>`
4. DB 验证:
   ```bash
   DB="$HOME/Library/Application Support/tech.yanjingai.app/meeting_minutes.sqlite"
   sqlite3 "$DB" "SELECT chunk_count, ROUND(processing_time,1), status, substr(error,1,300) FROM summary_processes WHERE meeting_id='meeting-c1299582-d80c-4d7d-972f-27e2ee3027d7' ORDER BY updated_at DESC LIMIT 1"
   ```

## 可能的真实根因 (从 §218 error 推测,优先级)

| 真实 error | 推测根因 | 修复 |
|---|---|---|
| `Failed to write request to stdin` | llama-helper 重启或 pipe 断 | §197 llama-cpp-2 0.1.146 baseline |
| `context size exceeded` | prompt+chunk+output > 8K | §217 已修; 如果还不够,继续扩到 16K |
| `model not found` | Qwen2.5-3B GGUF 未下载 | 模型自动下载触发 |
| `timeout after 3600s` | 推理 1h 未完成 | §194 timeout 已扩到 60min, 4 chunk × 137s ≈ 9min |
| `cancelled` | 用户主动取消 | UI 主动触发 |

如果 §218 落库后真实 error 是 llama-helper stdin pipe 问题,需要 §219 进一步修。如果还是 timeout,需要 §217 8K 不够,继续扩展。

## 关联

- §215 / §216 / §217 (上一轮优化 context + KV cache)
- §194 (GENERATION_TIMEOUT_SECS 60min)
- §197 (llama-cpp-2 0.1.146 baseline)
- §163 (sampling temp/top_p)
- §190 (qwen2.5:3b 默认)
- §198 (caller-provided n_layer=36)
- §92 (决策迁移铁律, 代码 + AGENTS.md + Obsidian + Codex outputs 四处同日落)
- §15 (GUI 验收强制)
- §37 (硬闸门)
- §18 (不主动改无关 bug)
- §56 (AGENTS.md §X 描述 ≠ 代码 commit, 这次 code + outputs + AGENTS.md 一次到位)

## commit

```
fix(§218): chunk error 写 DB — 真实根因落库, 不再被吞
```

## 已知边界 (按 §18 不动)

- 25 cargo warnings (§18 范围)
- 1 bun:test tsc error (§18 范围)
- p218 单测用 `format!("chunk {}/{} failed: {}", ...)` 模拟, 真实 chunk 错误信息来自 `llama-helper` 进程交互
- first_chunk_error 只保存第一个错误, 多 chunk 失败时后续错误不保存 — 设计选择, 排查路径优先看第一个
