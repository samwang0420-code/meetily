# Self-review findings: codex/accuracy-experiment

(Pre-subagent working set; will be consolidated with subagent findings.)

## Critical

### C1. useSummaryGeneration.ts — duplicate `fetchAllTranscripts` + silent empty-array overwrite
**File**: `frontend/src/hooks/meeting-details/useSummaryGeneration.ts`
**Issue**: `handleGenerateSummary` calls `await fetchAllTranscripts(meeting.id)` TWICE in the same flow (lines 555 and 574). The first call's result is assigned to `allTranscripts` inside the try block and only validated for emptiness (`if (!allTranscripts.length)` returns error early). After early-return validation passes, control falls out of the try block, then `const allTranscripts = await fetchAllTranscripts(meeting.id);` runs AGAIN on line 574. If the second fetch returns empty for any reason (transient DB error, race with another writer, pagination), `allTranscripts` silently becomes `[]` and the build/POST path proceeds with empty transcripts, sending `text: ""` to the backend — backend then either rejects or generates an empty summary.
**Why it matters**: User-visible failure mode is intermittent: first generate may work, second/third attempts may silently send empty transcripts. Also doubles DB load.
**Fix**: Remove the second fetch — reuse the variable from the first call. The `if (!allTranscripts.length)` check should sit OUTSIDE the inner try block, not inside it.

### C2. service.rs — meeting title extracted AFTER `==⚠️…⚠️==` highlight wrapping
**File**: `frontend/src-tauri/src/summary/service.rs`
**Issue**: `process_transcript_background` calls `extract_meeting_name_from_markdown(&final_markdown)` on line 742 AFTER `final_markdown = highlight_unexpected_facts(&final_markdown, &fact_report)` on line 734. When fact_guard flags a fabricated number/date/name, that token gets wrapped in `==⚠️X⚠️==`. If the fabricated token is inside the title (`# 2024年5月29日 会议`), the extracted title becomes `==⚠️2024年5月29日⚠️== 会议` and gets saved to the `meetings` table as the canonical title — polluting DB with markup.
**Why it matters**: Real user-visible bug. A user opening a meeting whose title contains a fabricated date the fact guard caught will see `==⚠️2024年5月29日⚠️== 会议` as the meeting title.
**Fix**: Extract the meeting name BEFORE `highlight_unexpected_facts` runs, e.g. between line 702 (normalize) and line 734 (highlight).

## Major

### M1. processor.rs — `normalize_section_key` keeps digits, contradicting its own docstring
**File**: `frontend/src-tauri/src/summary/processor.rs`
**Issue**: `normalize_section_key` (lines 746-754) docstring says it removes digits ("去数字"), but the implementation is `.filter(|c| c.is_alphanumeric() || (*c as u32) > 0x4E00)` — `is_alphanumeric()` includes digits 0-9. So timestamps like `[08:17]` and `[8:17]` produce keys `0817` vs `817` and won't be dedup'd. The §138 P0.1 dedup fails for any section whose dedup identity differs only by leading zeros or digit punctuation.
**Why it matters**: Real dedup regression on sections that cite mm:ss timestamps — every chunk of a 90-min transcript that quotes `[00:12]`-style markers produces a distinct "duplicate" section, defeating the dedup goal.
**Fix**: Add `!c.is_ascii_digit()` to the filter, or split out and drop digits: `.filter(|c| c.is_alphabetic() || (*c as u32) > 0x4E00)`.

### M2. hard_post_process.rs — `replace_with_chinese_boundary` only blocks when next char equals `wrong[last]`
**File**: `frontend/src-tauri/src/summary/hard_post_process.rs`
**Issue**: The function (lines 71-100) documents intent as `(?<![一-龥])X(?![一-龥])` (both lookbehind AND lookahead), but only implements a narrow `next == wrong[last]` lookahead. So "李富强" + "国" (next char CJK but ≠ "强") still gets replaced → "李福强" + "国". A 4-char name like "李富强国" would be wrongly rewritten to "李福强国". Also, the lookbehind is entirely missing — a preceding CJK char (e.g. "被告人李富强") still triggers replacement, which the documented regex would forbid.
**Why it matters**: For the default mapping the impact is small (no common 4-char names in the dict), but the function is the foundation for user-configurable mappings in Settings — any user who adds a 2-3 char wrong word will see false-positive replacements when the wrong word is preceded by ANY CJK char or followed by a CJK char ≠ `wrong[last]`.
**Fix**: Implement both `prev_is_cjk` and `next_is_cjk` checks. The current intent comment "错词 + 后一字符是汉字 且 与 wrong[last] 相同 → 阻止" is too narrow.

### M3. fact_guard.rs — `wrap_summary_as_multi_case_array` injects raw defendant names into JSON via `format!`
**File**: `frontend/src-tauri/src/summary/fact_guard.rs`
**Issue**: `wrap_summary_as_multi_case_array` (lines ~1788+) builds JSON via `format!(concat!(... "{first}" ... "{second}" ...))` where `first` and `second` are raw Chinese defendant name strings. While most ASR-produced names are safe, ANY name containing `"`, `\`, control chars, or even `\u2028`/`\u2029` line separators will produce malformed JSON that the frontend will fail to `JSON.parse`. The function uses `serde_json::to_string` for `content` and `warning` (safe) but NOT for `first`/`second`.
**Why it matters**: Edge-case but real. ASR errors like "被告人说"\" (rare but possible) would crash frontend JSON.parse, leaving the user on a broken page.
**Fix**: Use `serde_json::to_string(&first_defendant).unwrap_or(...)` for both name fields, same as for `content_json` and `warning_json`.

### M4. processor.rs — `extract_canonical_names` only handles 3-char names
**File**: `frontend/src-tauri/src/summary/fact_guard.rs`
**Issue**: `extract_canonical_names` (lines 1033-1075) iterates `while i + 3 <= chars.len()` and always inserts a 3-char slice — never inserts 4-char names. Compound surnames like "欧阳明" (4 chars) are silently dropped from canonical-name sets, so:
  - `normalize_name_drift` cannot find canonical matches for 4-char summary names
  - `detect_name_drift` only sees 3-char names
**Why it matters**: Any user with a transcript mentioning a 4-char (compound-surname) person will have name-drift detection fail silently — fabricated 4-char names won't be flagged.
**Fix**: After `seen.insert(three)`, also try inserting `chars[i..i+4]` if `i+4 <= chars.len()` and `chars[i+3]` is a CJK char (compound surname pattern). Or extend the surname table to include compound surnames like "欧阳", "司马", "诸葛", etc. and handle them as 2-char surname + 2-char given.

### M5. fact_guard.rs — `detect_attribution_confusion` 模式 3 is dead code
**File**: `frontend/src-tauri/src/summary/fact_guard.rs`
**Issue**: Lines 669-683 iterate `court_decision_fake` patterns and have an EMPTY body — they just consume the pattern and check `transcript.contains(...)` but never `out.push(...)`. Comment says "this is handled by detect_fabricated_verdict". The function literally does nothing in mode 3.
**Why it matters**: Dead code that misleads future maintainers into thinking mode 3 is implemented. Low immediate impact.
**Fix**: Either remove mode 3 entirely, or actually emit an issue. The comment correctly notes the overlap with `detect_fabricated_verdict`.

### M6. SummaryPanel.tsx — `detectMultiCaseSummary` called twice per render, re-parses JSON twice
**File**: `frontend/src/components/MeetingDetails/SummaryPanel.tsx`
**Issue**: Lines 698 and 703 both call `detectMultiCaseSummary(aiSummary)` — each call does `JSON.parse(rawMarkdown.trimStart())`. Two redundant parses per render.
**Why it matters**: Wasted CPU on every render of a multi-case summary, and worse, the two parses are not atomic — if `aiSummary` somehow mutates between them (it shouldn't but theoretically), they could disagree.
**Fix**: Compute once: `const multi = detectMultiCaseSummary(aiSummary); if (multi.isMultiCase) { ... {multi.caseCount} ... }`.

### M7. SummaryPanel.tsx — dead `summaryResponse && (...)` legacy render block
**File**: `frontend/src/components/MeetingDetails/SummaryPanel.tsx`
**Issue**: Lines 730-769 render a fixed bottom panel with `summaryResponse.summary.key_points.blocks.map(...)`. The component already renders `BlockNoteSummaryView` (line 772) which handles all modern formats. This legacy block predates the BlockNote era and is unreachable in normal flow because `summaryResponse` is rarely populated in the new flow. If it ever IS populated (e.g. legacy data without markdown field), the user sees a duplicated summary render on top of the BlockNote view.
**Why it matters**: Latent UI bug for any user with old data + new build — duplicate / broken summary display. Also pollutes the layout with a `fixed bottom-0 left-0 right-0` panel that overlays the BlockNote view.
**Fix**: Delete lines 730-769 (and remove the `summaryResponse` prop from the interface if unused elsewhere).

## Minor

### m1. processor.rs — `clean_llm_markdown_output` PREFIXES list misses many real LLM fence variants
**File**: `frontend/src-tauri/src/summary/processor.rs`
**Issue**: Only `["```markdown\n", "```\n"]` are matched. Real LLMs also emit `` ```Markdown ``, `` ```MD ``, `` ```text ``, `` ```md\n ``, or sometimes wrap with a trailing newline before the closing fence. None of these are stripped — the fence markers leak into the final output and break BlockNote rendering.
**Why it matters**: Intermittent "raw markdown leaks into UI" bug depending on model behavior.
**Fix**: Use a regex like `^\s*```(?:markdown|md|Markdown|MD)?\s*\n(.*?)\n```\s*$` and trim; fall through to `trimmed.to_string()` only if no match.

### m2. processor.rs — `replace_range` direction is reversed from the bug fix comment
**File**: `frontend/src-tauri/src/summary/hard_post_process.rs`
**Issue**: `replace_range` (lines 190-205) is correct in spirit (defensive is_char_boundary floor) but the docstring at lines 191-192 says "防止上游计算 start/end 时 byte 漂移" — the upstream call in `normalize_standard_verbs` line 132 passes byte indices computed from `extract_chinese_ngrams` which uses char-safe byte positions, so the floor is unreachable defensive code. Not a bug, just over-engineering.
**Why it matters**: None — leave as-is.

### m3. service.rs — `regen_marker` meeting_id suffix logic is convoluted
**File**: `frontend/src-tauri/src/summary/service.rs`
**Issue**: Lines 586-598 compute `meeting_short` by reversing, taking 8 chars, reversing again. This produces the last 8 chars of meeting_id, but the implementation is harder to read than `meeting_id.chars().rev().take(8).collect::<Vec<_>>().into_iter().rev().collect::<String>()` or `meeting_id[meeting_id.len().saturating_sub(8)..].to_string()`. Functionally correct.
**Why it matters**: Code clarity only.

### m4. SummaryPanel.tsx — `summaryStatus === null` is dead branch
**File**: `frontend/src/components/MeetingDetails/SummaryPanel.tsx`
**Issue**: Line 214 checks `summaryStatus === 'error' || summaryStatus === null` — but `summaryStatus` is typed as a string union (never null).
**Why it matters**: None.

### m5. useSummaryGeneration.ts — heartbeat toast spam every 8s
**File**: `frontend/src/hooks/meeting-details/useSummaryGeneration.ts`
**Issue**: Comment on line 174 says "5s 心跳 toast" but the code on line 181 is `8000` (8s). Misleading comment; not a bug, just inconsistency.
**Why it matters**: Doc/code drift — easy to misread.

### m6. fact_guard.rs — DEFENDANT_RE misses CJK Extension A
**File**: `frontend/src-tauri/src/summary/fact_guard.rs`
**Issue**: The regex `[\u4e00-\u9fa5]{2,3}` only covers BMP CJK Unified Ideographs. CJK Extension A (0x3400-0x4DBF, 罕用汉字) characters in defendant names are silently dropped. Unlikely to bite in practice (rare surnames).
**Why it matters**: Edge case.

### m7. hard_post_process.rs — `jaccard_char_similarity` computes `union` then ignores it
**File**: `frontend/src-tauri/src/summary/hard_post_process.rs`
**Issue**: Line 181 computes `let union = a_chars.union(&b_chars).count() as f32;` but never uses `union` — returns `2.0 * inter / (a_chars.len() + b_chars.len())` (Dice / Sørensen coefficient, not Jaccard). The function is misnamed but functionally OK.
**Why it matters**: Dead computation; misleading name.

