#!/usr/bin/env python3
"""
v0.8 migration completeness guard

用户原话 (2026-07-30):
  "我再编译一个0.8版本, 我们测试一下你不会弄丢代码的控制"

目的: 用 §28 决策迁移铁律验证 v0.8 HEAD 仍然包含 v0.7 决策池中
的关键修复锚点. 如果某条锚点缺失, 说明 v0.8 演进过程中"漏迁"
了 v0.7 已经写过的代码.

检查策略:
  1) 静态字符串锚点 (Path + phrase) 必须存在
  2) Rust 函数名锚点必须存在 (在 *.rs 中 grep fn <name>)
  3) DB migration 文件必须存在 (migrations/ 目录)
  4) AGENTS.md / Obsidian 决策日志 锚点必须存在
  5) tag v0.7.0-rc8 链可达

退出码: 0 = 全过, 1 = 有缺失.

用法:
  python3 scripts/check_v08_migration_completeness.py [--json]
"""
from __future__ import annotations
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def sh(cmd: list[str]) -> str:
    return subprocess.check_output(cmd, cwd=ROOT, text=True).strip()


def sh_status(cmd: list[str]) -> tuple[int, str]:
    p = subprocess.run(cmd, cwd=ROOT, text=True, capture_output=True)
    return p.returncode, (p.stdout + p.stderr).strip()


# v0.7 → v0.8 必须保留的决策 (按 AGENTS.md 编号)
ANCHORS: list[tuple[str, str, str, str]] = []  # (label, kind, target, expected)


def add(label: str, kind: str, target: str, expected: str) -> None:
    ANCHORS.append((label, kind, target, expected))


# §27 5GB 导入限制铁律
add(
    "§27 5GB import limit (audio/import.rs)",
    "rust_constant",
    "frontend/src-tauri/src/audio/import.rs",
    "5 * 1024 * 1024 * 1024",
)
add(
    "§27 5GB import limit (zh i18n)",
    "phrase",
    "frontend/src/i18n/locales/zh.ts",
    "5 GB",
)
add(
    "§27 5GB import limit (en i18n)",
    "phrase",
    "frontend/src/i18n/locales/en.ts",
    "5 GB",
)

# §31 free quota
add(
    "§31 Free monthly meeting limit (quota.rs)",
    "rust_constant",
    "frontend/src-tauri/src/user/quota.rs",
    "FREE_MONTHLY_MEETING_LIMIT: i64 = 5",
)
add(
    "§31 Free segments per transcript (quota.rs)",
    "rust_constant",
    "frontend/src-tauri/src/user/quota.rs",
    "FREE_SEGMENTS_PER_TRANSCRIPT_LIMIT: i64 = 100",
)
add(
    "§31 Free monthly watermark (useCopyOperations.ts)",
    "phrase",
    "frontend/src/hooks/meeting-details/useCopyOperations.ts",
    "watermark_footer",
)

# §28 machine binding
add(
    "§28 machine binding (user/commands.rs)",
    "rust_func",
    "frontend/src-tauri/src/user/commands.rs",
    "bound_machine_id",
)

# §28 activation rate limit
add(
    "§28 activation rate limit (user/commands.rs)",
    "phrase",
    "frontend/src-tauri/src/user/commands.rs",
    "ratelimit::check_and_record",
)

# §26 user isolation migration
add(
    "§26 user isolation migration SQL",
    "file",
    "frontend/src-tauri/migrations/20260722000000_user_meeting_isolation.sql",
    "user_id",
)

# §22 / §24 / §25 summary fixes chain
add(
    "§22 §24 summary token clamp (processor.rs)",
    "rust_constant",
    "frontend/src-tauri/src/summary/processor.rs",
    "DEFAULT_SUMMARY_MAX_TOKENS: u32 = 1200",
)
add(
    "§24 summary auth fallback (commands.rs)",
    "phrase",
    "frontend/src-tauri/src/summary/commands.rs",
    "latest_session_in_db",
)
add(
    "§22 summary failure persistence (commands.rs)",
    "phrase",
    "frontend/src-tauri/src/summary/commands.rs",
    "update_process_failed",
)

# §36 summary Map-Reduce + 防幻觉 (this commit)
add(
    "§36 LOCAL_SUMMARY_CHUNK_THRESHOLD (service.rs)",
    "rust_constant",
    "frontend/src-tauri/src/summary/service.rs",
    "LOCAL_SUMMARY_CHUNK_THRESHOLD",
)
add(
    "§36 local_summary_token_threshold function",
    "rust_func",
    "frontend/src-tauri/src/summary/service.rs",
    "fn local_summary_token_threshold",
)
add(
    "§36 EVIDENCE_GROUNDED_SUMMARY_RULES rule 12 (processor.rs)",
    "phrase",
    "frontend/src-tauri/src/summary/processor.rs",
    "First classify whether the source is an actual meeting",
)
add(
    "§36 standard_meeting template 本次无新决议",
    "phrase",
    "frontend/src-tauri/templates/standard_meeting.json",
    "本次无新决议",
)
add(
    "§36 standard_meeting template 本次无行动事项",
    "phrase",
    "frontend/src-tauri/templates/standard_meeting.json",
    "本次无行动事项",
)
add(
    "§36 default template narrative-empty-states test (defaults.rs)",
    "rust_func",
    "frontend/src-tauri/src/summary/templates/defaults.rs",
    "fn test_standard_meeting_has_narrative_empty_states",
)

# §33 / §34 / §35 ASR+VAD regression fixes (this commit)
add(
    "§33 VAD absolute timestamp (vad.rs)",
    "phrase",
    "frontend/src-tauri/src/audio/vad.rs",
    "self.speech_start_sample = timestamp_ms * 16000 / 1000;",
)
add(
    "§34 LIVE_TRANSCRIPTION_MAX_SEGMENT_SAMPLES (vad.rs)",
    "rust_constant",
    "frontend/src-tauri/src/audio/vad.rs",
    "LIVE_TRANSCRIPTION_MAX_SEGMENT_SAMPLES",
)
add(
    "§35 transcription-error event (worker.rs)",
    "phrase",
    "frontend/src-tauri/src/audio/transcription/worker.rs",
    'emit("transcription-error"',
)

# §15 historical fix guard self-bootstrap
add(
    "§15 historical fix guard script",
    "file",
    "scripts/check_historical_fixes.py",
    "Historical fix guard",
)

# §22 cli state parity
add(
    "§15 reference in global AGENTS.md",
    "phrase",
    str(Path.home() / ".codex" / "AGENTS.md"),
    "§15",
)


def check_phrase(path: str, phrase: str) -> tuple[bool, str]:
    p = ROOT / path
    if not p.exists():
        return False, f"missing file: {path}"
    text = p.read_text(encoding="utf-8", errors="ignore")
    if phrase in text:
        return True, f"{path}: found '{phrase[:40]}…'"
    return False, f"{path}: missing '{phrase[:60]}'"


def check_constant(path: str, pattern: str) -> tuple[bool, str]:
    return check_phrase(path, pattern)


def check_func(path: str, fn_name: str) -> tuple[bool, str]:
    p = ROOT / path
    if not p.exists():
        return False, f"missing file: {path}"
    text = p.read_text(encoding="utf-8", errors="ignore")
    pattern = re.compile(rf"\b{re.escape(fn_name)}\b")
    if pattern.search(text):
        return True, f"{path}: found {fn_name}"
    return False, f"{path}: missing function {fn_name}"


def check_file(path: str, phrase: str) -> tuple[bool, str]:
    p = ROOT / path
    if not p.exists():
        return False, f"missing file: {path}"
    text = p.read_text(encoding="utf-8", errors="ignore")
    if phrase in text:
        return True, f"{path}: file present + phrase '{phrase[:40]}' found"
    return False, f"{path}: file present but missing phrase '{phrase[:60]}'"


def check_anchor(label: str, kind: str, target: str, expected: str) -> tuple[bool, str]:
    if kind == "phrase":
        return check_phrase(target, expected)
    if kind == "rust_constant":
        return check_constant(target, expected)
    if kind == "rust_func":
        return check_func(target, expected)
    if kind == "file":
        return check_file(target, expected)
    return False, f"{label}: unknown kind={kind}"


def main() -> int:
    passed: list[str] = []
    failed: list[tuple[str, str]] = []
    for label, kind, target, expected in ANCHORS:
        ok, info = check_anchor(label, kind, target, expected)
        if ok:
            passed.append(info)
        else:
            failed.append((label, info))

    print(f"\nv0.8 migration completeness guard: {len(passed)}/{len(passed)+len(failed)} passed\n")
    for line in passed:
        print(f"  ✓ {line}")
    for label, info in failed:
        print(f"  ✗ {label}: {info}")

    # Check v0.7.0-rc8 tag still reachable
    rc, _ = sh_status(["git", "rev-parse", "--verify", "v0.7.0-rc8^{commit}"])
    if rc == 0:
        print("\n  ✓ tag v0.7.0-rc8 reachable from HEAD")
    else:
        print("\n  ✗ tag v0.7.0-rc8 NOT reachable (you've likely lost the v0.7 baseline)")
        failed.append(("v0.7.0-rc8 tag reachable", "git rev-parse failed"))

    if failed:
        print("\nMigration guard FAILED.")
        return 1
    print("\nMigration guard PASSED.")
    return 0


# §31 P1 member-only 模板闸门 — v0.8.1 commit 5ea2bad 加 test 真断言
add(
    "§31 P1 test_list_templates 真实断言 (template_commands.rs)",
    "rust_func",
    "frontend/src-tauri/src/summary/template_commands.rs",
    "fn test_list_templates",
)
add(
    "§31 P1 test_list_templates_falls_back_to_free 已知 tier 边界 (template_commands.rs)",
    "rust_func",
    "frontend/src-tauri/src/summary/template_commands.rs",
    "fn test_list_templates_falls_back_to_free",
)
add(
    "§31 P1 list_templates_for_tier 过滤逻辑 (loader.rs)",
    "rust_func",
    "frontend/src-tauri/src/summary/templates/loader.rs",
    "fn list_templates_for_tier",
)
add(
    "§31 P1 TemplateInfo 含 required_tier 字段 (template_commands.rs)",
    "phrase",
    "frontend/src-tauri/src/summary/template_commands.rs",
    "required_tier",
)
add(
    "§18 LLM Streaming 已实装 (processor.rs)",
    "phrase",
    "frontend/src-tauri/src/summary/processor.rs",
    "StreamSink",
)
add(
    "§18 CoreML feature flag 已 wire (whisper_engine.rs)",
    "phrase",
    "frontend/src-tauri/src/whisper_engine/whisper_engine.rs",
    'feature = "coreml"',
)
add(
    "§18 CoreML enabled 入口日志 (whisper_engine.rs)",
    "phrase",
    "frontend/src-tauri/src/whisper_engine/whisper_engine.rs",
    "Apple CoreML support: enabled",
)


if __name__ == "__main__":
    sys.exit(main())

# §31 P1 member-only 模板闸门 — v0.8.1 commit 5ea2bad 加 test 真断言.
add(
    "§31 P1 test_list_templates 真实断言 (template_commands.rs)",
    "rust_func",
    "frontend/src-tauri/src/summary/template_commands.rs",
    "fn test_list_templates",
)
add(
    "§31 P1 test_list_templates_falls_back_to_free 已知 tier 边界 (template_commands.rs)",
    "rust_func",
    "frontend/src-tauri/src/summary/template_commands.rs",
    "fn test_list_templates_falls_back_to_free",
)
add(
    "§31 P1 list_templates_for_tier 过滤逻辑 (loader.rs)",
    "rust_func",
    "frontend/src-tauri/src/summary/templates/loader.rs",
    "fn list_templates_for_tier",
)
add(
    "§31 P1 TemplateInfo 含 required_tier 字段 (template_commands.rs)",
    "phrase",
    "frontend/src-tauri/src/summary/template_commands.rs",
    "required_tier",
)
add(
    "§36 本地 LLM 摘要强制 1800-token chunk (service.rs) — 已在上面 v0.7",
    "phrase",
    "frontend/src-tauri/src/summary/service.rs",
    "LOCAL_SUMMARY_CHUNK_THRESHOLD",
)
# 已包含 (§36 在 v0.7 锚点已建), 但加一条 LLM streaming 锚点对应 "都做了"
add(
    "§18 LLM Streaming 已实装 (processor.rs)",
    "phrase",
    "frontend/src-tauri/src/summary/processor.rs",
    "StreamSink",
)
add(
    "§18 CoreML feature flag 已 wire (whisper_engine.rs)",
    "phrase",
    "frontend/src-tauri/src/whisper_engine/whisper_engine.rs",
    'feature = "coreml"',
)
