# §51 Map 受控并发 — commit 19f00dc (2026-08-03)

> 用户截图重新生成 a09de61d 用 10 分钟, 截图反馈"太慢了你看中间出了什么问题".
> 真根因: Map 阶段串行, 9 chunk × 67s/chunk = 10 分钟.
> 移植自离线会记/ perf/summary-map-concurrency 分支 commit e950608 (2026-07-23).

## 1. 慢的真根因

DB 实测 (8/3 01:32-01:42):
- meeting-a09de61d 重新生成
- chunk_count = 9 (Map-Reduce 走通)
- processing_time = 606.1s = **10 分钟 6 秒**
- start_time = 01:32:14 / end_time = 01:42:20

代码层根因 (主仓库 meetily/ frontend/src-tauri/src/summary/processor.rs:541):
```rust
for (i, chunk) in chunks.iter().enumerate() {
    let user_prompt_chunk = build_chunk_summary_user_prompt(chunk, output_language);
    match generate_summary(...).await {  // ← 每个 chunk 顺序 await
```
**Map 阶段完全串行**, 9 chunk 一个一个跑 LLM.

## 2. e950608 历史 commit

2026-07-23 Codex 在离线会记/ fresh clone 上做了 e950608:
- "perf(summary): Map 受控并发 (默认 2 路) + chunk_text O(n²)→O(n)"
- 30 分钟会议 (~3000 token, 2 chunks) Map wall-time 由串行 Sum 17.9s → Max 6.1s

**但这个 commit 从未合并到主仓库 meetily/** —— AGENTS.md §37 升级 v0.8 漏了.

## 3. 移植方案

两个仓库 processor.rs 已分叉, 不能 git apply / cherry-pick. 手写核心:

### 3.1 chunk_text O(n²) → O(n)

```rust
// 原版 (O(n²))
let chars: Vec<char> = text.chars().collect();
let start_byte: usize = chars[..start_char].iter().map(|c| c.len_utf8()).sum();
// 新版 (O(n))
let mut char_byte_offsets: Vec<usize> = Vec::with_capacity(text.len() / 3 + 1);
char_byte_offsets.push(0);
for c in text.chars() {
    char_byte_offsets.push(char_byte_offsets.last().unwrap() + c.len_utf8());
}
let start_byte = char_byte_offsets[start_char];
```

### 3.2 Map 阶段 FuturesUnordered 受控并发

```rust
let map_concurrency: usize = std::env::var("MEETILY_MAP_CONCURRENCY")
    .ok().and_then(|s| s.parse().ok()).unwrap_or(2).max(1);

// owned data 跨 spawn 边界
let client_arc: Arc<reqwest::Client> = Arc::new(client.clone());
let chunks_arc: Arc<Vec<String>> = Arc::new(chunks.clone());
let chunk_results: Arc<Mutex<Vec<Option<String>>>> = Arc::new(Mutex::new(vec![None; num_chunks]));
let mut inflight: FuturesUnordered<tokio::task::JoinHandle<()>> = FuturesUnordered::new();
let mut next_to_spawn = 0;

while next_to_spawn < num_chunks || !inflight.is_empty() {
    if let Some(token) = cancellation_token {
        if token.is_cancelled() {
            return Err("Summary generation was cancelled".to_string());
        }
    }
    while next_to_spawn < num_chunks && inflight.len() < map_concurrency {
        let i = next_to_spawn;
        next_to_spawn += 1;
        // ... owned clone ...
        inflight.push(tokio::spawn(async move { ... }));
    }
    if !inflight.is_empty() {
        inflight.next().await;  // wait one
        // 进度回调
    } else { break; }
}
let chunk_summaries: Vec<String> = {
    let results = chunk_results.lock().unwrap();
    results.iter().filter_map(|x| x.clone()).collect()
};
```

## 4. 性能预估

按 e950608 实测比例:
- 9 chunk × 67s 串行 (10 min) →
- **2 路并发 (默认)**: ~5 分钟 (67s × ⌈9/2⌉)
- **4 路**: ~2.5 分钟
- **8 路**: ~1.3 分钟 (CPU 充足)

默认 2 路是安全选择, 用户可 env MEETILY_MAP_CONCURRENCY=4 调高.

## 5. §37 硬闸门

| 步骤 | 结果 |
|---|---|
| npx tsc --noEmit | 0 errors (1 个 §18 bun:test 不动) |
| npx next build | (未跑, 仅后端改动, 无 frontend rebuild 必要) |
| cargo build --release | 1m44s ✓ (25 个 §18 warning 不动) |
| check_historical_fixes.py | **37/37 PASS** (34 → 37, 加 3 个 §51 锚点) |

## 6. binary

`/Users/wangwei/Documents/meetily/target/release/meetily` 67.83 MB
mtime 2026-08-03 10:00
tag v0.8.5 指向新 commit 19f00dc

## 7. §15 GUI 验收 (用户必做)

1. killall meetily && open /Users/wangwei/Documents/meetily/target/release/meetily
2. 打开 a09de61d 或新录 60 分钟会议
3. 点"重新生成", 看处理时间是否降到 5 分钟左右 (2 路并发)
4. 想更快: 启动前 `export MEETILY_MAP_CONCURRENCY=4`
5. 验证摘要 result 内容不变 (Map 阶段输出顺序按原 chunks 顺序拼接)

如果性能没改善, 立刻 `git reset --hard 1b52011 && cargo build --release` 回退.

## 8. 关联

- [[49-v0.8.4-77min-录音+超时诊断-2026-08-01]]
- [[49-fix-v0.8.5-完整-2026-08-02]] (commit 1b52011)
- [[49-fix-v0.8.5-§51-并发-2026-08-03]] (本文件)
- §15 §25 §37 §49 §51
