# §200 摘要 byte boundary panic 全防御 + panic location 写 DB

**触发**: 2026-08-31 11:25 用户触发 meeting-b0297a12 regenerate, 7 分 15 秒后 panic:
```
Background task panicked: end byte index 2176 is not a char boundary;
it is inside '发' (bytes 2175..2178 of string)
```

**根因**: §169.6 (commit 167b5a4) 只修了 `chunk_text` + `replace_range` 两处 byte slice, 但 LLM 输出后处理流水线还有 7 处未防御 byte slice. panic 字符串 byte 2176 是 '发' (U+53D1), 不在 transcript (transcript byte 2176 是 '哪' U+54EA), 也不在 result markdown (md_cn 3498 bytes 太短, md_en 4185 bytes byte 2176 是 '日'), 必须是 LLM 输出 markdown 中某处.

**修复 7 处 byte slice 全加 is_char_boundary floor**:

1. `processor.rs:850 clean_llm_markdown_output` - markdown fence extract (`trimmed[prefix.len()..trimmed.len()-SUFFIX.len()]`)
2. `service.rs:88 strip_leading_title` - markdown H1 strip
3. `hard_post_process.rs:199 extract_chinese_ngrams` - ngram 切片
4. `hard_post_process.rs:882 normalize_bullet_key` - 去 `==⚠️xxx⚠️==` 高亮标记
5. `hard_post_process.rs:765 truncate_raw_transcript_leak` - line 切片
6. `fact_guard.rs:402` `WEIGHT_AFTER_LARGE_RE.is_match(&summary[m.start()..])`
7. `fact_guard.rs:736` `&summary[dm.end()..window_end]` (DEFENSE_HEADER_RE + WITNESS_KIN_RE)

**诊断增强 (commands.rs catch_unwind)**:
- `std::panic::Location::caller()` 自动捕获 panic 抛出点的 file:line
- panic 写 DB 格式: `"Background task panicked at {file}:{line} — {msg}"`
- 下次 panic 自动定位抛出点, 不需要 git log 反查 (之前 §169.6 panic 修了 chunk_text 但 panic 还在别处, 必须 repro 才能找到真凶)

**§37 6 步硬闸门**:
- ✅ cargo check --lib: 0 errors
- ✅ cargo test --lib: 540 passed / 1 failed (1 fixture-bound §161, §18 不动)
- ✅ cargo build --release: 6m 59s, binary SHA `efcba024b554`
- ✅ check_historical_fixes.py: 712 → **717/717 PASS** (+5 §200 anchors)
- ✅ sync_app_bundle.sh: 3 binary 全 sync (言镜 AI + llama-helper + ffmpeg)
- ⏳ GUI 端到端 (§15 强制, 用户必做)

**commit**: `7758960 fix(§200): 摘要生成 byte boundary panic 全防御 + panic location 写 DB`
**binary**: `target/release/meetily` SHA `efcba024` (含 §200)

**§15 GUI 验收 (用户必做)**:
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 重生成 meeting-b0297a12 摘要
# 期望:
#   1. 不再 panic byte boundary (任何路径都过 is_char_boundary floor)
#   2. 如果还 panic, error 字段会含 file:line (例如 "panicked at hard_post_process.rs:782 — ...")
#      → 自动定位抛出点
```

**关联**:
- §169.6 (2026-08-24): 修了 chunk_text + replace_range, 没覆盖其他 byte slice 路径
- §198 (2026-08-30): byte 2434 '诉' panic 修复, safe_slice helper 引入
- §18 / §37 / §56 / §92 / §93 / §108
