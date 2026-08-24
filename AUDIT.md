# codex/accuracy-experiment — Audit Report

**Branch**: `codex/accuracy-experiment`
**Baseline**: `main` (v0.9.3)
**Diff size**: 25 files, +1796 / -59
**Audit method**: 4 parallel focused reviews + independent code inspection + empirical regex verification

| Severity | Count |
|----------|-------|
| Critical | 2 |
| Major | 8 |
| Minor | 12 |
| False positives filtered | 2 |

---

## Critical

### C1. `commands.rs:333-367` + `useSummaryGeneration.ts:192-205` — §169.1 triple-name fallback is dead code on a wrong premise
**Files**: `frontend/src-tauri/src/summary/commands.rs`, `frontend/src/hooks/meeting-details/useSummaryGeneration.ts`, `frontend/src-tauri/migrations/20260823000000_summary_invoke_log.sql`

**Issue**: The §169.1 commit claims Tauri v2's `invoke()` does NOT auto-convert `camelCase` ↔ `snake_case`, and adds defensive triple-name parameters (`force_fresh`, `force_fresh_camel`, `force_fresh_alias`) plus two migration columns. **This premise is factually wrong.** Tauri 2.6+ auto-converts camelCase → snake_case by default ([Tauri PR #1753](https://github.com/tauri-apps/tauri/pull/1753)). The project's pinned `@tauri-apps/api` is `^2.6.0`. Every existing parameter on the same command — `meetingId`, `chunkSize`, `customPrompt`, `templateId`, `summaryLanguage`, `evidence`, `forceFresh`, `regenerationFlag` — already works as camelCase → snake_case. The two new parameters `force_fresh_camel` / `force_fresh_alias` are unreachable; the corresponding migration columns `force_fresh_camel_recv` / `force_fresh_alias_recv` will always log `NULL` in production.

**Why it matters**:
1. Every regenerate silently writes 4 always-NULL fields to `summary_invoke_log`. The diagnostic output (which the §169.4 commit promises is the only ground-truth channel on macOS where stderr is discarded) will be useless for debugging the very issue §169.1 was trying to solve — the user will see `force_fresh_recv = true, force_fresh_camel_recv = NULL, force_fresh_alias_recv = NULL, regeneration_flag_recv = true` and won't be able to tell if the auto-conversion worked.
2. If a future Tauri release changes the auto-conversion default (or if a config option disables it), the regenerate path silently degrades to first-time behavior, reintroducing the §169 bug — but this time without any test that would catch it.

**Fix**: Either
- (a) Drop the triple-name fallback and the two unreachable migration columns. Rely on Tauri's documented auto-conversion. Keep only `force_fresh` (snake_case, for completeness with the JS `force_fresh` key — which Tauri auto-converts anyway) and `regeneration_flag`.
- (b) Make the contract explicit: `#[tauri::command(rename_all = "snake_case")]` on the command function (Tauri 2.6+ supports this), or move to a single typed request struct with `#[serde(rename_all = "camelCase")]`. This is the most defensive option and decouples the contract from Tauri behavior.
Either way, fix the §169.1 comment to reflect Tauri 2.6 reality.

### C2. `useSummaryGeneration.ts:337` — Polling strips `fact_guard` so §148 legal-critical banner and §170 multi-case banner go silent on regenerate
**Files**: `frontend/src/hooks/meeting-details/useSummaryGeneration.ts`, `frontend/src/components/AISummary/BlockNoteSummaryView.tsx`, `frontend/src/components/MeetingDetails/SummaryPanel.tsx`

**Issue**: When polling returns a successful result, the new code does
```ts
setAiSummary({ markdown: pollingResult.data.markdown } as any);
```
instead of `setAiSummary({ ...pollingResult.data } as any)`. This throws away `fact_guard`, `fact_guard_legal_critical`, `_multiCase`, `english_cache`, `summary_json` — every field the Rust `build_summary_result_json_with_facts` emits except `markdown`. After this code lands, every regenerate silently degrades:
- `BlockNoteSummaryView` lines 343/364/392 read `data?.fact_guard_legal_critical` to render the §148 critical banner → always falsy.
- `SummaryPanel.tsx:628` reads `(aiSummary as any)?.fact_guard` for the FactGuardBanner → undefined → no banner.
- `detectMultiCaseSummary(aiSummary)` still works because it reads `aiSummary.markdown` directly.

**Why it matters**: §148 critical detection is most valuable on regenerate (when the LLM is re-evaluating with new warnings), and the §170 multi-case banner is most valuable on the same path. Both are silently disabled by this one-line change. The user will see "regenerate succeeded" but no warning that the new summary contains a fabricated verdict / cross-case pollution / missing evidence.

**Fix**: Change line 337 from
```ts
setAiSummary({ markdown: pollingResult.data.markdown } as any);
```
to
```ts
setAiSummary({ ...pollingResult.data } as any);
```

---

## Major

### M1. `database/repositories/summary.rs:96-165` — `create_or_reset_process` silently destroys §135 history archive
**File**: `frontend/src-tauri/src/database/repositories/summary.rs`

**Issue**: The UPSERT (lines 96-119) runs FIRST and sets `result = NULL`, moving the old value to `result_backup`. Then the SELECT (lines 122-135) reads `sp.result` (now NULL). The guard `if let Some((Some(result_str), _, …)) = old` requires `result_str` to be `Some(...)`, but it's now `None` (NULL). The history archive branch (lines 136-165) never fires. After this diff, every regenerate silently loses the previous summary — `result_backup` survives temporarily but is cleared by `update_process_completed` on success.

**Why it matters**: Silent data loss. §135 (`api_summary_history`) is the entire mechanism by which users can revisit past generations. The §169.5 commit removed the `WHERE status != 'completed'` guard without fixing the follow-up SELECT. Every regenerate on a completed meeting loses the previous version forever.

**Fix**: Either
- (a) Snapshot the SELECT BEFORE the UPSERT and use that snapshot for the archive INSERT.
- (b) Read `result_backup` (not `result`) in the SELECT after the UPSERT — the backup column holds the old value at that point.
Option (b) is one-line:
```rust
SELECT sp.result_backup, sp.result, ...
```
…then destructure as `(result_backup, _, …)` instead of `(result, _, …)`.

### M2. `SummaryPanel.tsx:188-199` — Heartbeat monotonic guard doesn't protect the `summary-phase` listener (0→100% saw-tooth regression)
**File**: `frontend/src/components/MeetingDetails/SummaryPanel.tsx`

**Issue**: The §170.5 commit's heartbeat effect bumps `summaryProgress` with a `prev + 1` monotonic guard. But the `summary-phase` event listener on line 188 directly calls `setSummaryProgress(pct)` on line 194 with no monotonic guard. When the backend transitions `map` (progress=0.5) → `reduce` (progress=0) on a multi-chunk Map-Reduce, the listener resets progress from 50 → 0. The user's progress bar visibly saw-tooths backward at every chunk boundary — exactly the regression the §170.5 commit claims to fix.

**Why it matters**: User-visible progress bar flicker during any long summary (multi-chunk Map-Reduce). The commit message promises "防止 0-100% 循环"; the actual code only prevents the heartbeat itself from causing the loop, not the listener.

**Fix**: Wrap the listener's `setSummaryProgress` in a monotonic clamp:
```ts
setSummaryProgress((prev) => Math.max(prev, pct));
```

### M3. `fact_guard.rs:1821-1842` — `wrap_summary_as_multi_case_array` JSON-unescaped `first`/`second` defendant interpolation
**File**: `frontend/src-tauri/src/summary/fact_guard.rs`

**Issue**: `wrap_summary_as_multi_case_array` builds JSON via `format!(concat!(... "{first}" ... "{second}" ...))` where `first` / `second` are raw Chinese defendant name strings. `content_json` and `warning_json` use `serde_json::to_string` (safe), but `first` / `second` are interpolated raw. A defendant name containing `"`, `\`, control chars, or `\u2028`/`\u2029` produces malformed JSON that breaks the frontend `JSON.parse`. Low practical likelihood for Chinese names, but the same code path is hit by any future transcript in another language.

**Why it matters**: Edge case but real. ASR errors producing punctuation in names would silently corrupt the user-visible multi-case banner. Worst case: backend returns malformed JSON, frontend's `detectMultiCaseSummary` throws, banner falls back to "not multi-case" — silently losing the §165 fix.

**Fix**: Use `serde_json::to_string(&first_defendant).unwrap_or_default()` and `serde_json::to_string(&second_defendant).unwrap_or_default()` for both fields, OR use `serde_json::json!({ ... })` macro for the whole payload.

### M4. `fact_guard.rs:1799-1818` + `1829-1842` — `wrap_summary_as_multi_case_array` duplicates defendant extraction logic that can disagree with `detect_cross_case_pollution`
**File**: `frontend/src-tauri/src/summary/fact_guard.rs`

**Issue**: `wrap_summary_as_multi_case_array` calls `detect_cross_case_pollution(transcript, summary)`. If issues are found, it then runs ITS OWN `DEFENDANT_RE` (line 1799) and ITS OWN `stop_words` (line 1804) to extract `first_defendant` / `second_defendant`. The internal detector used a different `stop_words` set and a different second-defendant extraction (`defendants[defendants.len() - 1]`). The two views can disagree: `detect_cross_case_pollution` reports issues → `wrap_summary_as_multi_case_array` proceeds → local extraction finds 0 or 1 defendants → returns `None` → the user sees no §165 banner despite cross-case pollution being detected. The test `section_165_no_multi_case_returns_none` only covers the early-return (`issues.is_empty()`) and does NOT cover this `issues_non_empty_but_defendants_insufficient` branch.

**Why it matters**: Silent degradation of the §165 multi-case wrap path on transcripts where cross-case pollution is detected but defendant extraction fails (e.g., ASR produced punctuation in defendant names, or defendant name is 1 char long).

**Fix**: Extract the shared defendant extraction into a `pub(crate) fn extract_defendants(transcript: &str) -> Vec<String>` and reuse from both functions. Add a test for the `issues_non_empty_but_defendants_insufficient` branch.

### M5. `hard_post_process.rs:71-100` — `replace_with_chinese_boundary` only blocks when next char equals `wrong[last]`; lookbehind is missing
**File**: `frontend/src-tauri/src/summary/hard_post_process.rs`

**Issue**: The function documents intent as `(?<![一-龥])X(?![一-龥])` (both lookbehind AND lookahead), but only implements a narrow `next == wrong[last]` lookahead. The lookbehind is entirely missing — "被告人李富强" still gets replaced (the documented regex would forbid). Also, the narrow lookahead means "李富强" + "国" (next CJK ≠ wrong[last]) still gets replaced → "李福强国", wrongly rewriting a 4-char name.

**Why it matters**: For the default mapping, low practical impact (no common 4-char names in the dict). For user-configurable mappings in Settings, this means any 2-3 char wrong word will have false-positive replacements when preceded by ANY CJK char or followed by a CJK char ≠ `wrong[last]`.

**Fix**: Implement both `prev_is_cjk` and `next_is_cjk` checks. Replace the single-char comparison with the broader CJK boundary test.

### M6. `processor.rs:746-754` — `normalize_section_key` keeps digits, contradicting docstring; §138 dedup regression on timestamps
**File**: `frontend/src-tauri/src/summary/processor.rs`

**Issue**: The docstring says the function removes digits ("去数字"), but the implementation is `.filter(|c| c.is_alphanumeric() || (*c as u32) > 0x4E00)` — `is_alphanumeric()` includes digits 0-9. So timestamps like `[08:17]` and `[8:17]` produce keys `0817` vs `817` and won't be dedup'd. The §138 P0.1 dedup fails for any section whose dedup identity differs only by leading zeros or digit punctuation.

**Why it matters**: Real dedup regression on sections that cite mm:ss timestamps — every chunk of a 90-min transcript that quotes `[00:12]`-style markers produces a distinct "duplicate" section. The Map-Reduce final output gets bloated with N copies of the same timestamp-cited paragraph.

**Fix**: Add `&& !c.is_ascii_digit()` to the filter, or use `c.is_alphabetic() || (*c as u32) > 0x4E00`.

### M7. `useSummaryGeneration.ts:555/574` — `fetchAllTranscripts` called twice per `handleGenerateSummary`
**File**: `frontend/src/hooks/meeting-details/useSummaryGeneration.ts`

**Issue**: `handleGenerateSummary` calls `await fetchAllTranscripts(meeting.id)` once inside the early-return validation block (line 555), validates emptiness, then calls it AGAIN on line 574 to obtain the payload. Each call internally does **two** Tauri IPC round-trips (`limit=1` then `limit=totalCount`). For a 1000-segment meeting, every "Generate Summary" click triggers 4 extra DB queries before the LLM is invoked.

**Why it matters**: Doubles pre-LLM latency on the click path. Also opens a race window: between the two fetches, transcripts may be edited/deleted and the second result can disagree with the validation result the user just saw.

**Fix**: Capture the result of the first fetch into a variable in the outer scope and reuse it.

### M8. `BlockNoteSummaryView.tsx:292-336` — Dead `caseBlocks`/`caseEditorsRef`/parent `useEffect` double-parses every multi-case
**File**: `frontend/src/components/AISummary/BlockNoteSummaryView.tsx`

**Issue**: The parent `BlockNoteSummaryView` defines `caseBlocks`, `caseEditorsRef`, and a `useEffect` (lines 320-336) that sequentially awaits `parseCaseMarkdown` for every entry in `multiCases` and calls `setCaseBlocks(next)`. None of these are read by the render path. The multi-case branch renders `<MultiCaseCard caseData={c} … />`, and `MultiCaseCard` re-parses its own content inside its own effect. The parent does the work once, throws it away, then `MultiCaseCard` does the same work again.

**Why it matters**: Every multi-case summary pays the markdown-parsing cost twice. For meetings with many cases, wasted CPU on the main render path.

**Fix**: Delete lines 316-317, 320-336, and the `parseCaseMarkdown` callback (lines 299-313). `MultiCaseCard` already handles parsing per-case.

---

## Minor

### m1. `service.rs:742` — `extract_meeting_name_from_markdown` runs AFTER `highlight_unexpected_facts`, polluting meeting titles with `==⚠️…⚠️==`
**File**: `frontend/src-tauri/src/summary/service.rs`

The meeting title extraction happens at line 742, AFTER `final_markdown = highlight_unexpected_facts(...)` on line 734. When fact_guard flags a fabricated date in the title, it gets wrapped in `==⚠️X⚠️==` and saved to the `meetings` table as the canonical title. Extract the meeting name BEFORE `highlight_unexpected_facts` runs.

### m2. `fact_guard.rs:1033-1075` — `extract_canonical_names` only handles 3-char names; 4-char compound surnames (欧阳明, 司马迁) are silently dropped
**File**: `frontend/src-tauri/src/summary/fact_guard.rs`

Loop is `while i + 3 <= chars.len()` and always inserts a 3-char slice. 4-char names like "欧阳明" (compound surname) are missed. Either extend to 4-char inserts at compound-surname positions, or extend the SURNAMES table with compound surnames (欧阳, 司马, 诸葛, etc.) and handle them as 2-char surname + 2-char given.

### m3. `fact_guard.rs:669-683` — `detect_attribution_confusion` mode 3 is dead code
**File**: `frontend/src-tauri/src/summary/fact_guard.rs`

Iterates `court_decision_fake` patterns but has an empty body. Comment correctly notes this is handled by `detect_fabricated_verdict`. Either remove the empty block or implement the intent.

### m4. `SummaryPanel.tsx:698/703` — `detectMultiCaseSummary(aiSummary)` called twice per render, re-parses JSON twice
**File**: `frontend/src/components/MeetingDetails/SummaryPanel.tsx`

Plus the hardcoded "个案件" suffix is not localized for English users. Memoise with `useMemo` and add an i18n key with a count placeholder.

### m5. `SummaryPanel.tsx:730-769` — Dead `summaryResponse && (...)` legacy render block
**File**: `frontend/src/components/MeetingDetails/SummaryPanel.tsx`

Pre-BlockNote-era render of `summaryResponse.summary.key_points.blocks` etc. Latent UI bug for any user with old data + new build: duplicate summary render on top of the BlockNote view. Delete lines 730-769 (and remove the `summaryResponse` prop from the interface if unused).

### m6. `service.rs:586-602` — `regen_marker` inlines raw `meeting_id` into the LLM prompt; no escaping, no length cap
**File**: `frontend/src-tauri/src/summary/service.rs`

`meeting_id` originates from the frontend and is written verbatim into the LLM prompt inside `<regeneration_marker>…</regeneration_marker>` (which is then prepended inside `<transcript_chunks>…</transcript_chunks>` on processor.rs line 1101). A pathological `meeting_id` containing `<transcript_chunks>`, `<system>`, or `\n<regeneration_marker>` would let the caller inject arbitrary instructions into the prompt envelope. Combined with §169.3's hard-coded `temperature=0.7` on the regenerate path, the injected instruction has more effect than at the default 0.1. Strip / cap `meeting_id`, or hash it (truncated SHA256 first 16 hex chars) so the suffix is bounded.

### m7. `useSummaryGeneration.ts:176-181` — Heartbeat `safeToast.info(...)` has no `id`, so sonner queues a new toast every 8 s
**File**: `frontend/src/hooks/meeting-details/useSummaryGeneration.ts`

While the LLM is invoking, the user sees a flashing "请求中..." toast every 8s instead of one steady indicator. Pass a stable `id` so sonner replaces the same toast instead of stacking new ones; dismiss on completion.

### m8. `processor.rs:771` — `clean_llm_markdown_output` PREFIXES list misses many real LLM fence variants
**File**: `frontend/src-tauri/src/summary/processor.rs`

Only `["```markdown\n", "```\n"]` matched. LLMs also emit `` ```Markdown ``, `` ```MD ``, `` ```text ``, etc. — these leak into the final output. Use a regex like `^\s*```(?:markdown|md|Markdown|MD)?\s*\n(.*?)\n```\s*$`.

### m9. `BlockNoteSummaryView.tsx:56-68` — Multi-case JSON detection fails on `[\n{...}]` form (leading newline)
**File**: `frontend/src/components/AISummary/BlockNoteSummaryView.tsx`

`trimStart()` removes leading whitespace but does not strip an opening newline. If the LLM emits `"[\n{ ... }]"`, `trimmed.startsWith('[{')` is false. Some Ollama models place the first object on the next line. Use `^\s*\[\s*\{` regex.

### m10. `scripts/check_historical_fixes.py:447` — Anchor `169_1_force_fresh_value_or` uses PCRE-only regex on the script's POSIX ERE fallback path
**File**: `scripts/check_historical_fixes.py`

Pattern `r"force_fresh[\s\S]*?\.or\(force_fresh_camel\)[\s\S]*?\.or\(force_fresh_alias\)"` uses `[\s\S]` and `*?`, both PCRE-only. On machines without `rg` (the script's documented fallback to `grep -E`), the anchor reports false negative. Verified empirically: `grep -rqE` returns exit 1 on the file even though the code on lines 362-365 is correct. Split into two simpler POSIX-portable anchors, OR fix `grep()` to require `rg` and remove the broken fallback.

### m11. `BlockNoteSummaryView.tsx:432-450` — `MultiCaseCard`'s effect ignores `factGuard` deps and leaks the 100ms `setTimeout`
**File**: `frontend/src/components/AISummary/BlockNoteSummaryView.tsx`

Effect deps are `[content, caseIdx]` with `eslint-disable-next-line react-hooks/exhaustive-deps`. If `factGuard` updates, the highlighted body is not re-rendered. Plus the `setTimeout(() => { isContentLoaded.current = true; }, 100)` is never tracked or cleared. Add `factGuard` to deps and clear the timer via a ref in the cleanup.

### m12. `service.rs:692-695` — `cleanup_cancellation_token` runs before processing the result; race with `api_cancel_summary`
**File**: `frontend/src-tauri/src/summary/service.rs`

`Self::cleanup_cancellation_token(&meeting_id)` runs on line 693 before the `match result` block. If a user clicks cancel between 693 and 760 (`update_process_completed`), `cancel_summary` returns false ("no active token"), and the DB write goes through with `status='completed'`. UI may flicker between cancelled/completed. Move cleanup to after result processing, or check `cancellation_token.is_cancelled()` one last time before writing.

---

## False positives (filtered out from subagent reports)

### FP-1. `fact_guard.rs:1799-1818` — "wrap_summary_as_multi_case_array extracts polluted defendant names like '三小因'"
**Verdict**: NOT A BUG. Verified empirically by running the exact regex `r"(?:被告人|罪犯|被告)([\u4e00-\u9fa5]{2,3}?)"` against the test transcript (using the `regex` crate 1.13.1, same as project). Non-greedy `{2,3}?` captures **2 chars** (the minimum), not 3:
```
captured: "三小" (range 9..15)
captured: "赵某" (range 59..65)
```
The trailing 因 / 因 are NOT included. The first subagent's regex-portability analysis was incorrect.

### FP-2. `summary.rs` create_or_reset_process data loss was reported as **Critical** by the first subagent
**Verdict**: REAL BUG, but downgraded to Major in this report. The bug exists, but the impact is bounded — the user's previously-generated summary is preserved in `result_backup` until `update_process_completed` clears it, and `update_process_failed` restores from backup on failure. So in the failure path the user is fine. Only on successful regenerate does the data loss actually happen (because `update_process_completed` clears `result_backup = NULL` on success). This is "Major" because the impact is real (user-visible data loss on the §135 history feature) but not "Critical" because the user's primary summary view is unaffected — only the ability to roll back / compare via `api_summary_history` is broken.

---

## What was cleared (no defects)

- **Tauri invoke snake_case contract (non-§169.1 params)**: existing params (`meetingId`, `chunkSize`, `customPrompt`, etc.) work correctly. No new contract violations.
- **i18n parity**: All 8 new keys present in both `zh.ts` and `en.ts`.
- **XSS**: No new `dangerouslySetInnerHTML` introduced.
- **char-boundary safety**: §169.6 fix in `chunk_text()` is correct. The new code paths use `serde_json::to_string` or `chars()`, no byte indexing.
- **SQL migration correctness**: Schema is well-formed, columns match INSERT binds, idempotent `CREATE IF NOT EXISTS`, indexes the right column for per-meeting drill-down (though the documented query doesn't use it — see m10).
- **`llama-helper/src/main.rs`**: §163 defaults are read at startup, env-var overrides bounded by `unwrap_or`, tests verify all three defaults.
- **`templates/defaults.rs`**: `medical_internal_round` addition is clean (lookup map + switch both updated atomically).
- **`types/index.ts`**: Adding `'multi-case'` to `SummaryFormat` is a clean enum extension.
- **Python script (everything except m10)**: All 41 other newly added anchors match their target code. No shell injection (`subprocess.run` always uses argv lists).

---

## Recommended fix order

1. **C1 + C2** (Critical) — These break the regenerate path's core promise. Fix before any release.
2. **M1** (Major data loss) — Silent data loss on every regenerate. Fix before next release that touches `summary.rs`.
3. **M2, M3, M5, M6, M7, M8** (Major) — User-visible regression bugs. Fix in the same release as C1/C2.
4. **m1, m2, m3, m4, m5** (Minor) — Polish for the next release.
5. **m6** (Minor, security) — `regen_marker` prompt injection. Low practical risk but worth hardening.
6. **m7, m8, m9, m10, m11, m12** (Minor) — Cleanup pass.

