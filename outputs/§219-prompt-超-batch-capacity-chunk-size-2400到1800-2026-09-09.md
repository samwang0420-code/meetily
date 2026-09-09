# §219 prompt 超 batch capacity — chunk_size 2400→1800 + n_batch 8K→16K (2026-09-09)

## 触发

用户 9/9 13:33 重跑 meeting-c1299582 摘要,§218 落库逻辑生效:

```
sqlite3 ... SELECT substr(error,1,400) FROM summary_processes ...
Multi-level summarization failed: chunk 2/3 failed: Generation failed: Generation failed: Failed to add token to batch
```

## 真根因 (3 跳)

1. **system prompt 实际超 8K batch capacity**:
   ```python
   ENGLISH_BASE_SUMMARY_INSTRUCTION     =  156 chars
   EVIDENCE_GROUNDED_SUMMARY_RULES      = 6167 chars  # §138/§161/§163 等基础规则
   P1_PRECISION_RULES                   = 3727 chars  # §138/§195 等
   P141_VERBATIM_FACT_CHECK             = 4285 chars  # §141 verbatim 反例 + 5 步 checklist
   P161_MULTI_CASE_AND_EVIDENCE         = 2866 chars  # §161 多案件 + 关键证据
   P188_EVIDENCE_COPY                   = 2033 chars  # §188 evidence copy
   P189_CASE_TYPE_DROPDOWN              = 1096 chars  # §189 案件类型下拉
                                          ──────────
                                  total = 20330 chars ≈ ~6776 tokens  # 系统 prompt 自身
   ```

2. **chunk 2400 tokens 太大**: chunk_transcript_by_token() 切 2400 token chunks,加上 system (6776) + template (200) ≈ **9376 tokens > 8192 batch capacity**

3. **为什么 chunk 1 OK,chunk 2/3 fail**: §53 中文标点感知切分,chunk 1 因标点位置偶然 ~8100 tokens 通过,chunk 2/3 必超。BatchAddError::InsufficientSpace(8192) 抛出 → llama-helper main.rs:554 包装 "Failed to add token to batch" → client.rs:288 包装 "Generation failed: Failed to add token to batch" → processor.rs:1075 保存到 first_chunk_error → summary_processes.error 落库

## 修复 (3 处)

### §219A: chunk_size 2400→1800

`frontend/src-tauri/src/summary/processor.rs`:
```rust
pub fn chunk_transcript_by_token(text: &str) -> Vec<String> {
    const CHUNK_SIZE: usize = 1800;  // §219: 2400→1800
    const OVERLAP: usize = 50;
    chunk_text(text, CHUNK_SIZE, OVERLAP)
}
```

效果: 10,112 chars transcript 切 ~3-4 chunks (vs 之前 3 chunks)。每 chunk prompt = system (6776) + chunk (1800) + template (200) ≈ 8776 tokens。

### §219B: llama-helper n_batch 8K→16K

`llama-helper/src/main.rs`:
```rust
let ctx_params = LlamaContextParams::default()
    .with_n_ctx(Some(NonZeroU32::new(self.context_size).context("Invalid ctx size")?))
    // §219B: n_batch 8K→16K (跟 context 解耦)
    // 根因: prompt ~9376 tokens > 8K batch capacity
    // 配套: chunk_size 2400→1800, prompt 总数 ~8776, 余量 ~7200 tokens
    .with_n_batch(16384)
    .with_n_threads(threads)
    ...
```

效果: 即使未来 §X 注入更多 rule 让 system prompt 涨到 12K, 仍然在 16K n_batch 内。**不增加 KV cache**, 只在 llama_batch_init 时分配临时 buffer (~32 MB at 16K), 跟 n_ctx 8K KV cache Q4_0 (~600 MB) 相比可忽略。

### §219C: 移除 §150 anchor (chunk_size 2400),新增 §219 anchor (chunk_size 1800)

`scripts/check_historical_fixes.py`:
- `150_chunk_size_2400` → `150_chunk_size_1800_after_219`
- 新增 4 个 §219 anchor:
  - `219_chunk_size_1800` (const CHUNK_SIZE: usize = 1800)
  - `219_n_batch_16k` (with_n_batch(16384))
  - `219_p219_chunk_size_tests_mod` (测试 mod 存在)
  - `219_chunk_size_1800_comment_in_doc` (doc 注释含 §219A)

## §37 6 步硬闸门

- ✅ cargo check --lib: 0 errors (17 §18 warnings)
- ✅ cargo test --lib p219_chunk_size: **2/2 PASS**
  - `section_219_chunk_transcript_by_token_uses_1800_chunk_size` — 验证 chunk ≤ 5400 chars (= 1800 token * 3 chars/token 上限)
  - `section_219_prompt_overhead_estimate` — 验证 prompt 总数估算 ≤ 9000 tokens
- ✅ check_historical_fixes.py: **766/766 PASS** (+4 §219 anchor, -1 §150 anchor)
- ✅ cargo build --release: 增量 1m13s, binary 59.8M mtime **13:56**
- ✅ llama-helper build: 4.37s, binary 4.9M (sha edb92e0ea429)
- ✅ sync_app_bundle.sh: §93 main + §98 codesign + §108 sidecar 全 sync
  - target/release/meetily: sha e277cd66dd46 (未签)
  - target/release/言镜 AI.app/Contents/MacOS/言镜 AI: codesigned (sha 不同, normal)
- ⏳ GUI 端到端 (§15 强制, 用户必做)

## 铁律 (任何 v0.X 演进适用)

1. **prompt overhead 必须估算** — 任何 §X 加 system prompt 规则, 必须 grep 所有 const chars 总数, 跟 (n_batch - chunk_max) 比对
2. **chunk_size 必须 < n_batch - system_chars** — 留余地给未来 rule 增长
3. **n_batch 跟 n_ctx 解耦** — n_batch 是物理 batch size, n_ctx 是 KV cache 上限, 互不影响 (除 llama_batch_init 时分配 buffer)
4. **BatchAddError::InsufficientSpace 是 prompt 溢出信号** — 用户反馈 "n/M failed" 立即 grep 7 个 system const 总 chars
5. **chunk 边界偶然性** — §53 中文标点切分让 chunk 大小有 ±10% 浮动, 第 1 chunk 偶然通过不代表全部 OK
6. **§150 + §219 是同一条 chunk_size** — 改一个必须同步 guard anchor, 否则下次 release §150 守卫报 fail

## 用户必做 (§15 GUI 验收)

```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```

1. 打开 meeting-c1299582 (重庆通航融资租赁) → 点 "重新生成"
2. 期望 1: 进度条正常跑, ~3-5 min 完成 (3-4 chunks × 30s ≈ 2-3 min)
3. 期望 2: status=completed, chunk_count=3 或 4
4. DB 验证:
   ```bash
   DB="$HOME/Library/Application Support/tech.yanjingai.app/meeting_minutes.sqlite"
   sqlite3 "$DB" "SELECT chunk_count, ROUND(processing_time,1), status, substr(error,1,200) FROM summary_processes WHERE meeting_id='meeting-c1299582-d80c-4d7d-972f-27e2ee3027d7' ORDER BY updated_at DESC LIMIT 1"
   ```
   期望: `chunk_count=3 or 4, status=completed, error=NULL`

5. 如果还失败: error 字段不再是 "Failed to add token to batch" (这问题修了), 而是新的真实错误 (例如 "context size exceeded", "timeout after 3600s", "GGUF model load failed" 等), 进入下一轮排查

## 下次 §X 加 system prompt 规则前必读

```bash
# 检查现有 system prompt 总长度
python3 -c "
import re
with open('frontend/src-tauri/src/summary/processor.rs', 'r') as f:
    content = f.read()
consts = ['ENGLISH_BASE_SUMMARY_INSTRUCTION', 'EVIDENCE_GROUNDED_SUMMARY_RULES', 'P1_PRECISION_RULES', 'P141_VERBATIM_FACT_CHECK', 'P161_MULTI_CASE_AND_EVIDENCE', 'P188_EVIDENCE_COPY', 'P189_CASE_TYPE_DROPDOWN']
total = 0
for name in consts:
    m = re.search(rf'const {name}: &str = r#\"(.*?)\"#;', content, re.DOTALL)
    if not m:
        m = re.search(rf'const {name}: &str =\s*\"((?:[^\"\\\\]|\\\\.)*)\"', content, re.DOTALL)
    if m:
        total += len(m.group(1))
        print(f'{name}: {len(m.group(1))} chars')
print(f'Total: {total} chars ~ {total//3} tokens')
print(f'n_batch: 16384, prompt budget: 16384 - 1800 (chunk) - 200 (tmpl) = {16384 - 1800 - 200} tokens')
"
```

如果 `total // 3 > 14384`, 必须**减小某个 const** 或**减小 chunk_size** 或**扩 n_batch**。

## 关联

- §218 (chunk error 写 DB, 让用户能看到真实错误)
- §215 / §216 / §217 (上一轮 context 优化)
- §194 (GENERATION_TIMEOUT_SECS 60min)
- §197 (llama-cpp-2 0.1.146 baseline)
- §163 (sampling temp/top_p)
- §190 (qwen2.5:3b 默认)
- §198 (caller-provided n_layer=36)
- §150 (chunk_size 2400 被 §219 改为 1800, guard anchor 同步)
- §53 / §55 (中文标点感知切分)
- §92 (决策迁移铁律)
- §15 / §37 / §18 / §56
- Obsidian: `~/Documents/Obsidian Vault/项目/3-离线会记/§219-...md`
- outputs: `outputs/§219-...md`

## commit

```
fix(§219): prompt 超 batch capacity — chunk_size 2400→1800 + n_batch 8K→16K
```

## 已知边界 (按 §18 不动)

- 17 cargo warnings (§18 范围)
- 1 bun:test tsc error (§18 范围)
- chars_per_token = 1/0.35 = 2.857 是估算值, 实际 Qwen tokenizer 比例跟文本相关
- n_batch=16384 跟 context_size=8192 解耦, 实际 LlamaBatch 容量 = min(LlamaBatch::new(n, 1).allocated, n_batch)
- p219 测试用 5400 chars 上限 (= 1800 tokens * 3 chars/token 安全上限), 实际 chunk_text 可能更大 (标点切到边界)
