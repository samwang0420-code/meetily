#!/usr/bin/env python3
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]

CHECKS = [
    ("VAD absolute timestamp", "frontend/src-tauri/src/audio/vad.rs", "self.speech_start_sample = timestamp_ms * 16000 / 1000;"),
    ("VAD timestamp regression test", "frontend/src-tauri/src/audio/vad.rs", "test_speech_start_timestamp_is_not_double_counted"),
    ("VAD source duration test", "frontend/src-tauri/src/audio/vad.rs", "test_vad_timestamps_never_exceed_processed_audio_duration"),
    ("Live 8-second split", "frontend/src-tauri/src/audio/vad.rs", "LIVE_TRANSCRIPTION_MAX_SEGMENT_SAMPLES"),
    ("No SpeechEnd duplicate", "frontend/src-tauri/src/audio/vad.rs", "test_speech_end_does_not_repeat_forced_split_audio"),
    ("Boundary text dedup", "frontend/src-tauri/src/audio/recording_saver.rs", "text_boundary_overlap_chars"),
    ("Sherpa shutdown", "frontend/src-tauri/src/audio/sherpa_daemon.rs", "shutdown_global_daemon"),
    ("Visible transcription errors", "frontend/src-tauri/src/audio/transcription/worker.rs", 'emit("transcription-error"'),
    ("Diar pickup loop", "frontend/src-tauri/src/api/diar_pickup_loop.rs", "spawn_diar_pickup_loop"),
    ("Map-Reduce summary", "frontend/src-tauri/src/summary/processor.rs", "chunk_transcript_by_token"),
    ("Summary token clamp", "frontend/src-tauri/src/summary/processor.rs", "DEFAULT_SUMMARY_MAX_TOKENS: u32 = 1200"),
    ("Summary auth fallback", "frontend/src-tauri/src/summary/commands.rs", "latest_session_in_db"),
    ("Summary failure persistence", "frontend/src-tauri/src/summary/commands.rs", "update_process_failed"),
    ("Free monthly quota", "frontend/src-tauri/src/user/quota.rs", "FREE_MONTHLY_MEETING_LIMIT: i64 = 5"),
    ("Free segment quota", "frontend/src-tauri/src/user/quota.rs", "FREE_SEGMENTS_PER_TRANSCRIPT_LIMIT: i64 = 100"),
    ("Free watermark", "frontend/src/hooks/meeting-details/useCopyOperations.ts", "watermark_footer"),
    ("Machine binding", "frontend/src-tauri/src/user/commands.rs", "bound_machine_id"),
    ("Activation rate limit", "frontend/src-tauri/src/user/commands.rs", "ratelimit::check_and_record"),
    ("User isolation migration", "frontend/src-tauri/migrations/20260722000000_user_meeting_isolation.sql", "user_id"),
    ("5GB import limit", "frontend/src-tauri/src/audio/import.rs", "5 * 1024 * 1024 * 1024"),
    ("React hook fix", "frontend/src/hooks/useRecordingStop.ts", "const { isAutoRetranscribe } = useConfig();"),
    ("Sidebar session", "frontend/src/components/Sidebar/SidebarProvider.tsx", "lixianhuiji.session"),
]

failures = []
for name, relative, needle in CHECKS:
    path = ROOT / relative
    if not path.exists():
        failures.append(f"{name}: missing file {relative}")
        continue
    text = path.read_text(encoding="utf-8")
    if needle not in text:
        failures.append(f"{name}: missing anchor {needle!r} in {relative}")

if failures:
    print("Historical fix guard FAILED:")
    for failure in failures:
        print(f"- {failure}")
    sys.exit(1)

print(f"Historical fix guard passed: {len(CHECKS)}/{len(CHECKS)}")
