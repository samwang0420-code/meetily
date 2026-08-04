#!/usr/bin/env python3
"""
meetily historical-fix guard (AGENTS.md §35)
Independent of cargo/next build. Pure-text grep against the repo
to ensure that previously-fixed regressions are still present.

§15 §37 compliance: this script is a hard gate. Any non-zero
exit code blocks release binary.
"""
from __future__ import annotations
import argparse
import os
import subprocess
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def grep(pattern: str, path: str) -> bool:
    """rg if available, else grep -r. Exit 0 = match found."""
    try:
        r = subprocess.run(
            ["rg", "--quiet", pattern, path],
            capture_output=True, timeout=20,
        )
        return r.returncode == 0
    except FileNotFoundError:
        r = subprocess.run(
            ["grep", "-rqE", pattern, path],
            capture_output=True, timeout=20,
        )
        return r.returncode == 0


# Each anchor = (id, file, regex). The regex MUST match for the fix to be live.
# If a fix commit was rebased away, the regex will fail, and the guard fails.
ANCHORS = [
    # §32: 8s force-split threshold for continuous speech
    ("32_forced_split_threshold",
     "frontend/src-tauri/src/audio/transcription/worker.rs",
     r"(8\s*\*\s*1000|8000|forced_split|force_split)"),
    ("32_continuous_speech_test",
     "frontend/src-tauri/src/audio/transcription/worker.rs",
     r"test_continuous_speech_is_force_split_for_live_output"),

    # §33: VAD timestamp = absolute ms * 16000 / 1000, NO processed_samples add-back
    ("33_timestamp_not_double_counted",
     "frontend/src-tauri/src/audio/vad.rs",
     r"timestamp_ms\s*\*\s*16000\s*/\s*1000"),
    ("33_timestamp_test",
     "frontend/src-tauri/src/audio/vad.rs",
     r"test_speech_start_timestamp_is_not_double_counted"),

    # §34: force-split suppresses SpeechEnd.samples re-emission
    ("34_force_split_suppress_repeat",
     "frontend/src-tauri/src/audio/transcription/worker.rs",
     r"(suppress.*speech_end|speech_end_after_split|test_speech_end_does_not_repeat)"),

    # §23: sherpa daemon 2min idle kill (memory leak fix)
    ("23_daemon_idle_kill",
     "frontend/src-tauri/src/audio/sherpa_daemon.rs",
     r"(idle_kill|shutdown_global_daemon|touch_daemon_activity)"),

    # §36: Map-Reduce 1800-token hard cap (anti-hallucination)
    ("36_local_summary_chunk_threshold",
     "frontend/src-tauri/src/summary/service.rs",
     r"(LOCAL_SUMMARY_CHUNK_THRESHOLD\s*=\s*1800|min\(\s*1800)"),
    ("36_narrative_empty_states",
     "frontend/src-tauri/src/summary/templates/standard_meeting.json",
     r"(本次无新决议|本次无行动事项|单向叙事)"),

    # §22 §24: summary auth_token fallback to DB lookup
    ("24_summary_auth_db_fallback",
     "frontend/src-tauri/src/user/commands.rs",
     r"latest_session_in_db"),

    # §29: FunASR-Nano pro-only pricing tier gate
    ("29_funasr_nano_tier_gate",
     "frontend/src-tauri/src",
     r"(pro_only_funasr_nano|FunASR.*pro|Pro.*FunASR)"),

    # §27: 5GB import hard limit (user 7/22 explicit instruction)
    ("27_5gb_import_limit",
     "frontend/src-tauri/src/audio/import.rs",
     r"5\s*\*\s*1024\s*\*\s*1024\s*\*\s*1024"),

    # §38: default model = FunASR-Nano high precision
    ("38_default_funasr_nano",
     "frontend/src/contexts/ConfigContext.tsx",
     r"funasr-nano-zh"),

    # §15: Rust integration tests under #[cfg(test)]
    ("15_rust_tests_compile",
     "frontend/src-tauri/src/audio/transcription/worker.rs",
     r"#\[cfg\(test\)\]"),
]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--strict", action="store_true",
                    help="exit 1 on any failure (CI gate)")
    ap.add_argument("--verbose", action="store_true")
    args = ap.parse_args()

    print(f"=== Historical fix guard (AGENTS.md §35) — {len(ANCHORS)} anchors ===\n")
    passed, failed = 0, 0
    for anchor_id, rel_path, regex in ANCHORS:
        full_path = os.path.join(REPO, rel_path)
        if not os.path.exists(full_path):
            ok = False
            detail = f"FILE MISSING: {rel_path}"
        else:
            ok = grep(regex, full_path)
            detail = "OK" if ok else f"regex {regex!r} not found in {rel_path}"
        status = "PASS" if ok else "FAIL"
        print(f"  [{status}] {anchor_id:<40} {detail[:90]}")
        if ok:
            passed += 1
        else:
            failed += 1

    print(f"\nResult: {passed}/{len(ANCHORS)} anchors passed, {failed} failed.")
    if args.strict and failed > 0:
        print("STRICT MODE: refusing release binary.", file=sys.stderr)
        return 1
    return 0 if failed == 0 else 2


if __name__ == "__main__":
    raise SystemExit(main())
