import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.dirname(fileURLToPath(import.meta.url));
const decisionPath = path.join(root, 'reports', 'long-audio-decision.json');
const rawPath = path.join(root, 'reports', 'four-speaker-hour.json');
const runtimePath = path.join(root, '..', '..', 'src-tauri', 'scripts', 'diar.py');

const decision = JSON.parse(fs.readFileSync(decisionPath, 'utf8'));
const raw = JSON.parse(fs.readFileSync(rawPath, 'utf8'));
const runtime = fs.readFileSync(runtimePath, 'utf8');

const checks = {
  full_hour_audio: decision.audio_seconds >= 3600,
  expected_speakers_recorded: decision.expected_speakers === 4,
  actual_speaker_mismatch_recorded: decision.actual_speakers === raw.num_speakers && raw.num_speakers !== decision.expected_speakers,
  raw_segments_complete: Array.isArray(raw.segments) && raw.segments.length === decision.segments && raw.segments.length >= 800,
  measured_runtime_recorded: decision.wall_seconds > 0,
  measured_memory_recorded: decision.maximum_resident_bytes > 0,
  release_decision_disables_long_audio: decision.decision === 'disable_over_300_seconds',
  runtime_limit_matches_decision: /MAX_DIAR_AUDIO_SECONDS\s*=\s*300/.test(runtime),
  runtime_returns_long_audio_warning: /warning[^\n]+audio_too_long/.test(runtime),
};

const passed = Object.values(checks).every(Boolean);
const report = {
  passed,
  checks,
  evidence: {
    audio_seconds: decision.audio_seconds,
    expected_speakers: decision.expected_speakers,
    actual_speakers: decision.actual_speakers,
    segments: decision.segments,
    windows: decision.windows,
    wall_seconds: decision.wall_seconds,
    maximum_resident_mb: Math.round(decision.maximum_resident_bytes / 1024 / 1024),
    realtime_factor: Number((decision.wall_seconds / decision.audio_seconds).toFixed(3)),
    decision: decision.decision,
  },
};

fs.writeFileSync(path.join(root, 'reports', 'long-audio-verification.json'), `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify(report, null, 2));
if (!passed) process.exitCode = 1;
