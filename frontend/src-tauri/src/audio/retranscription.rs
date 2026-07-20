// Retranscription module - allows re-processing stored audio with different settings

use crate::audio::decoder::decode_audio_file;
use crate::audio::audio_processing::prepare_for_asr_16k;
use crate::audio::industry_terms::{correct_industry_terms, correct_industry_terms_with_known, runtime_hotword_terms, L3Config};
use crate::audio::vad::get_speech_chunks_with_progress;
use super::common::{create_transcript_segments, split_segment_at_silence, write_transcripts_json};
use super::constants::AUDIO_EXTENSIONS;
use crate::config::{DEFAULT_WHISPER_MODEL, DEFAULT_PARAKEET_MODEL};
use crate::parakeet_engine::ParakeetEngine;
use crate::state::AppState;
use crate::whisper_engine::WhisperEngine;
use anyhow::{anyhow, Result};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use base64::Engine as _;

/// Global flag to track if retranscription is in progress
static RETRANSCRIPTION_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Global flag to signal cancellation
static RETRANSCRIPTION_CANCELLED: AtomicBool = AtomicBool::new(false);

/// RAII guard for RETRANSCRIPTION_IN_PROGRESS flag
/// Ensures flag is cleared even if retranscription panics or returns early
struct RetranscriptionGuard;

impl RetranscriptionGuard {
    /// Create guard and set flag atomically
    fn acquire() -> Result<Self, String> {
        if RETRANSCRIPTION_IN_PROGRESS
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err("Retranscription already in progress".to_string());
        }
        Ok(RetranscriptionGuard)
    }
}

impl Drop for RetranscriptionGuard {
    fn drop(&mut self) {
        RETRANSCRIPTION_IN_PROGRESS.store(false, Ordering::SeqCst);
    }
}

/// VAD redemption time for batch enhancement.
///
/// A long redemption window merges several sentences into one segment. That
/// makes the transcript readable, but destroys the timestamp granularity used
/// by the evidence-based summary. Keep short recordings close to the live
/// pipeline; only long recordings get a wider pause bridge.
fn vad_redemption_time_ms(duration_seconds: f64) -> u32 {
    if duration_seconds <= 90.0 {
        700
    } else if duration_seconds <= 900.0 {
        1200
    } else {
        1600
    }
}

const MAX_EVIDENCE_SEGMENT_SECONDS: f64 = 12.0;

/// Progress update emitted during retranscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetranscriptionProgress {
    pub meeting_id: String,
    pub stage: String, // "decoding", "transcribing", "saving"
    pub progress_percentage: u32,
    pub message: String,
}

/// Result of retranscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetranscriptionResult {
    pub meeting_id: String,
    pub segments_count: usize,
    pub duration_seconds: f64,
    pub language: Option<String>,
    pub speech_coverage: f64,
    pub transcript_chars: usize,
    pub average_segment_seconds: f64,
    pub low_quality_warning: Option<String>,
    pub applied: bool,
}

#[derive(Debug, Default)]
struct ExistingTranscriptStats {
    segments: usize,
    chars: usize,
    average_segment_seconds: f64,
    texts: Vec<String>,
}

const LEGAL_GUARD_TERMS: &[&str] = &[
    "原告", "被告", "合同", "违约金", "管辖权异议", "仲裁条款", "仲裁程序",
    "仲裁裁决", "北京仲裁委员会", "中级人民法院", "申请撤销", "提起上诉",
];
const MEDICAL_GUARD_TERMS: &[&str] = &[
    "患者", "诊断", "高血压", "糖尿病", "二甲双胍", "胰岛素", "糖化血红蛋白",
    "肾小球滤过率", "低盐低脂饮食", "复查", "医嘱", "不良反应",
];

fn normalized_content(texts: &[String]) -> String {
    texts.iter().flat_map(|text| text.chars()).filter(|ch| {
        !ch.is_whitespace() && !ch.is_ascii_punctuation() && !"，。！？；：、（）【】《》“”‘’…—".contains(*ch)
    }).collect()
}

fn meaningful_char_count(text: &str) -> usize {
    normalized_content(&[text.to_string()]).chars().count()
}

fn fragment_ratio(texts: &[String]) -> f64 {
    if texts.is_empty() { return 1.0; }
    let fragments = texts.iter().filter(|text| meaningful_char_count(text) < 4).count();
    fragments as f64 / texts.len() as f64
}

fn punctuation_only_count(texts: &[String]) -> usize {
    texts.iter().filter(|text| meaningful_char_count(text) == 0).count()
}

fn retained_domain_terms(existing: &str, candidate: &str) -> (usize, usize) {
    let terms = LEGAL_GUARD_TERMS.iter().chain(MEDICAL_GUARD_TERMS.iter());
    let expected: Vec<&&str> = terms.filter(|term| existing.contains(**term)).collect();
    let retained = expected.iter().filter(|term| candidate.contains(***term)).count();
    (retained, expected.len())
}

fn should_apply_retranscription(
    existing: &ExistingTranscriptStats,
    candidate_segments: usize,
    candidate_chars: usize,
    candidate_average_segment_seconds: f64,
    candidate_texts: &[String],
    low_quality_warning: Option<&str>,
) -> Result<(), String> {
    if candidate_segments == 0 || candidate_chars == 0 {
        return Err("未识别出有效内容，已保留原始转录".to_string());
    }
    if low_quality_warning.is_some() && existing.chars > 0 {
        return Err("新结果质量检测未通过，已保留原始转录".to_string());
    }
    if existing.chars >= 20 && candidate_chars * 100 < existing.chars * 75 {
        return Err(format!("新结果仅保留原文约 {}%，已保留原始转录", candidate_chars * 100 / existing.chars));
    }
    if existing.segments >= 4 && candidate_segments * 2 < existing.segments {
        return Err("新结果分段明显减少，已保留原始时间戳分段".to_string());
    }
    if existing.segments >= 2 && candidate_segments > existing.segments.saturating_mul(3) {
        return Err("新结果分段暴增，已保留原始时间戳分段".to_string());
    }
    let candidate_fragment_ratio = fragment_ratio(candidate_texts);
    let existing_fragment_ratio = fragment_ratio(&existing.texts);
    if candidate_segments >= 5 && candidate_fragment_ratio > 0.25 && candidate_fragment_ratio > existing_fragment_ratio + 0.10 {
        return Err(format!("新结果碎片段占比过高（{}%），已保留原始转录", (candidate_fragment_ratio * 100.0).round() as usize));
    }
    if punctuation_only_count(candidate_texts) > punctuation_only_count(&existing.texts) {
        return Err("新结果包含独立标点段，已保留原始转录".to_string());
    }
    let existing_content = normalized_content(&existing.texts);
    let candidate_content = normalized_content(candidate_texts);
    let (retained_terms, expected_terms) = retained_domain_terms(&existing_content, &candidate_content);
    if expected_terms >= 3 && retained_terms * 100 < expected_terms * 85 {
        return Err(format!("新结果仅保留 {}/{} 个关键术语，已保留原始转录", retained_terms, expected_terms));
    }
    if existing.average_segment_seconds > 0.0
        && candidate_average_segment_seconds > existing.average_segment_seconds * 2.5
        && candidate_average_segment_seconds > 10.0
    {
        return Err("新结果时间戳粒度明显变差，已保留原始分段".to_string());
    }
    let gains_more_content = candidate_content.chars().count() >= existing_content.chars().count() * 105 / 100;
    let gains_fewer_fragments = candidate_fragment_ratio + 0.05 < existing_fragment_ratio;
    let gains_better_segments = existing.segments >= 6 && candidate_segments < existing.segments && candidate_segments * 2 >= existing.segments;
    if existing.chars > 0 && !gains_more_content && !gains_fewer_fragments && !gains_better_segments {
        return Err("新结果没有检测到明确质量提升，已保留原始转录".to_string());
    }
    Ok(())
}

/// Error during retranscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetranscriptionError {
    pub meeting_id: String,
    pub error: String,
}

/// Check if retranscription is currently in progress
pub fn is_retranscription_in_progress() -> bool {
    RETRANSCRIPTION_IN_PROGRESS.load(Ordering::SeqCst)
}

/// Cancel ongoing retranscription
pub fn cancel_retranscription() {
    RETRANSCRIPTION_CANCELLED.store(true, Ordering::SeqCst);
}

/// Start retranscription of a meeting's audio
pub async fn start_retranscription<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
    meeting_folder_path: String,
    language: Option<String>,
    model: Option<String>,
    provider: Option<String>,
) -> Result<RetranscriptionResult> {
    // Acquire guard - ensures flag is cleared even on panic/early return
    let _guard = RetranscriptionGuard::acquire().map_err(|e| anyhow!(e))?;

    // Reset cancellation flag
    RETRANSCRIPTION_CANCELLED.store(false, Ordering::SeqCst);

    let use_parakeet = provider.as_deref() == Some("parakeet");
    let result = run_retranscription(app.clone(), meeting_id.clone(), meeting_folder_path, language, model, provider).await;

    // Unload the engine after the batch job (success, failure, or cancellation)
    super::common::unload_engine_after_batch(use_parakeet).await;

    // Guard will automatically clear flag on drop
    // No need for manual: RETRANSCRIPTION_IN_PROGRESS.store(false, Ordering::SeqCst);

    match &result {
        Ok(res) => {
            let _ = app.emit(
                "retranscription-complete",
                serde_json::json!({
                    "meeting_id": res.meeting_id,
                    "segments_count": res.segments_count,
                    "duration_seconds": res.duration_seconds,
                    "language": res.language,
                    "speech_coverage": res.speech_coverage,
                    "transcript_chars": res.transcript_chars,
                    "average_segment_seconds": res.average_segment_seconds,
                    "low_quality_warning": res.low_quality_warning,
                    "applied": res.applied
                }),
            );
        }
        Err(e) => {
            let _ = app.emit(
                "retranscription-error",
                RetranscriptionError {
                    meeting_id: meeting_id.clone(),
                    error: e.to_string(),
                },
            );
        }
    }

    result
}

/// Find audio file in meeting folder
/// Tries common names first, then scans for any file with an audio extension
fn find_audio_file(folder: &Path) -> Result<PathBuf> {
    let candidates = [
        "audio.mp4", "audio.m4a", "audio.wav", "audio.mp3",
        "audio.flac", "audio.ogg", "recording.mp4",
        "audio.mkv", "audio.webm", "audio.wma",
    ];

    for name in candidates {
        let path = folder.join(name);
        if path.exists() {
            return Ok(path);
        }
    }

    // Fallback: scan folder for any file with an audio extension
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if AUDIO_EXTENSIONS.contains(&ext.as_str()) {
                    return Ok(path);
                }
            }
        }
    }

    Err(anyhow!("No audio file found in: {}", folder.display()))
}

/// Internal function to run retranscription
async fn run_retranscription<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
    meeting_folder_path: String,
    language: Option<String>,
    model: Option<String>,
    provider: Option<String>,
) -> Result<RetranscriptionResult> {
    let folder_path = PathBuf::from(&meeting_folder_path);
    let audio_path = find_audio_file(&folder_path)?;

    // Batch enhancement follows the same local Sherpa default as recording.
    let effective_provider = provider.as_deref().unwrap_or("sherpa_funasr_nano");
    let use_parakeet = effective_provider == "parakeet";
    let use_sherpa = effective_provider == "sherpa_paraformer"
        || effective_provider == "sherpa_funasr_nano";

    info!(
        "Starting retranscription for meeting {} with language {:?}, model {:?}, provider {:?}",
        meeting_id, language, model, effective_provider
    );

    // Emit progress: decoding
    emit_progress(&app, &meeting_id, "decoding", 5, "Decoding audio file...");

    // Check for cancellation
    if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) {
        return Err(anyhow!("Retranscription cancelled"));
    }

    // Decode the audio file (CPU-intensive, run in blocking task)
    let path_for_decode = audio_path.clone();
    let decoded = tokio::task::spawn_blocking(move || {
        decode_audio_file(&path_for_decode)
    })
    .await
    .map_err(|e| anyhow!("Decode task panicked: {}", e))??;
    let duration_seconds = decoded.duration_seconds;

    info!(
        "Decoded audio: {:.2}s, {}Hz, {} channels",
        duration_seconds, decoded.sample_rate, decoded.channels
    );

    emit_progress(&app, &meeting_id, "decoding", 15, "Converting audio format...");

    // Check for cancellation
    if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) {
        return Err(anyhow!("Retranscription cancelled"));
    }

    // Convert to 16kHz mono format (CPU-intensive, run in blocking task)
    let audio_samples = tokio::task::spawn_blocking(move || {
        let samples = decoded.to_whisper_format();
        prepare_for_asr_16k(&samples)
    })
    .await
    .map_err(|e| anyhow!("Resample task panicked: {}", e))?;
    info!("Converted to 16kHz mono format: {} samples", audio_samples.len());

    emit_progress(&app, &meeting_id, "vad", 20, "Detecting speech segments...");

    // Check for cancellation
    if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) {
        return Err(anyhow!("Retranscription cancelled"));
    }

    // Use VAD to find natural speech boundaries (same approach as live transcription)
    // IMPORTANT: Run VAD in a blocking task to avoid blocking the async runtime
    // For large files (35+ minutes), VAD processing can take several minutes
    let app_for_vad = app.clone();
    let meeting_id_for_vad = meeting_id.clone();
    // 离线会记 W2: sherpa 路径需要整体音频,VAD 闭包会 move audio_samples,先 clone 一份备用
    // W2.2: sherpa 路径已改成 VAD 段级循环,每段用 segment.samples 直接送 daemon,无需整段克隆
    let _sherpa_audio_samples = if use_sherpa { Some(audio_samples.clone()) } else { None };

    let vad_redemption_ms = vad_redemption_time_ms(duration_seconds);
    let speech_segments = tokio::task::spawn_blocking(move || {
        get_speech_chunks_with_progress(
            &audio_samples,
            vad_redemption_ms,
            |vad_progress, segments_found| {
                // Map VAD progress (0-100) to overall progress (20-25)
                let overall_progress = 20 + (vad_progress as f32 * 0.05) as u32;
                emit_progress(
                    &app_for_vad,
                    &meeting_id_for_vad,
                    "vad",
                    overall_progress,
                    &format!("Detecting speech segments... {}% ({} found)", vad_progress, segments_found),
                );

                // Return false to cancel if cancellation requested
                !RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst)
            },
        )
    })
    .await
    .map_err(|e| anyhow!("VAD task panicked: {}", e))?
    .map_err(|e| anyhow!("VAD processing failed: {}", e))?;

    let total_segments = speech_segments.len();
    info!("VAD detected {} speech segments (redemption_time={}ms)", total_segments, vad_redemption_ms);

    // Diagnostic: log segment duration distribution
    if !speech_segments.is_empty() {
        let durations_ms: Vec<f64> = speech_segments.iter()
            .map(|s| s.end_timestamp_ms - s.start_timestamp_ms)
            .collect();
        let total_speech_ms: f64 = durations_ms.iter().sum();
        let avg_duration = total_speech_ms / durations_ms.len() as f64;
        let min_duration = durations_ms.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_duration = durations_ms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        info!(
            "VAD segment stats: avg={:.0}ms, min={:.0}ms, max={:.0}ms, total_speech={:.1}s/{:.1}s ({:.0}%)",
            avg_duration, min_duration, max_duration,
            total_speech_ms / 1000.0, duration_seconds,
            (total_speech_ms / 1000.0 / duration_seconds) * 100.0
        );
        // Log first 10 segments for detailed inspection
        for (i, seg) in speech_segments.iter().take(10).enumerate() {
            let dur = seg.end_timestamp_ms - seg.start_timestamp_ms;
            debug!("  Segment {}: {:.0}ms-{:.0}ms ({:.0}ms, {} samples)",
                i, seg.start_timestamp_ms, seg.end_timestamp_ms, dur, seg.samples.len());
        }
        if total_segments > 10 {
            debug!("  ... and {} more segments", total_segments - 10);
        }
    }

    if total_segments == 0 {
        warn!("No speech detected in audio");
        return Err(anyhow!("No speech detected in audio file"));
    }

    emit_progress(&app, &meeting_id, "transcribing", 25, "Loading transcription engine...");

    // Initialize the appropriate engine once (not per-segment)
    // 离线会记 W2: sherpa 路径不加载 whisper/parakeet engine,直接走 daemon subprocess
    let whisper_engine = if !use_parakeet && !use_sherpa {
        Some(get_or_init_whisper(&app, model.as_deref()).await?)
    } else {
        None
    };
    let parakeet_engine = if use_parakeet && !use_sherpa {
        Some(get_or_init_parakeet(&app, model.as_deref()).await?)
    } else {
        None
    };

    // Split very long segments at silence boundaries for better transcription quality.
    // Hard cuts at arbitrary sample positions lose words at boundaries. Instead, scan
    // for the lowest-energy window near the target split point and cut there.
    const MAX_SEGMENT_SAMPLES: usize = (MAX_EVIDENCE_SEGMENT_SECONDS as usize) * 16000;

    let mut processable_segments: Vec<crate::audio::vad::SpeechSegment> = Vec::new();
    for segment in &speech_segments {
        if segment.samples.len() > MAX_SEGMENT_SAMPLES {
            debug!(
                "Splitting large segment ({:.0}ms, {} samples) at silence boundaries",
                segment.end_timestamp_ms - segment.start_timestamp_ms,
                segment.samples.len()
            );

            let sub_segments = split_segment_at_silence(segment, MAX_SEGMENT_SAMPLES);
            debug!("Split into {} sub-segments", sub_segments.len());
            processable_segments.extend(sub_segments);
        } else {
            processable_segments.push(segment.clone());
        }
    }

    let processable_count = processable_segments.len();
    info!("Processing {} segments (after splitting)", processable_count);

    // Process each speech segment with progress updates
    let mut all_transcripts: Vec<(String, f64, f64)> = Vec::new(); // (text, start_ms, end_ms)
    let mut total_confidence = 0.0f32;

    // sherpa backend name (resolved once, reused per segment)
    let sherpa_backend = if use_sherpa {
        if effective_provider == "sherpa_funasr_nano" {
            Some("sensevoice-zh".to_string())
        } else {
            Some("paraformer-zh".to_string())
        }
    } else {
        None
    };
    let hotwords_pack_str = crate::audio::hotwords_globals::current_pack();
    let hotwords_custom_owned = crate::audio::hotwords_globals::current_custom_with_product_terms();
    let hotwords_custom_str = hotwords_custom_owned.as_str();
    // Level 3 不再需要全局 Mutex 暂存 token — 每段 daemon 返回后立即在闭包内切 sub-segments

    // 离线会记 W2.2: 所有 backend (whisper/parakeet/sherpa) 都走 VAD 段级 loop
    // 之前 sherpa 路径绕过 VAD 整段喂 Paraformer,导致 127s 音频只识别 1 段 (丢 95% 内容)
    for (i, segment) in processable_segments.iter().enumerate() {
        // Check for cancellation before each segment
        if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) {
            return Err(anyhow!("Retranscription cancelled"));
        }

        // Calculate progress (25% to 80% range for transcription)
        let progress = 25 + ((i as f32 / processable_count as f32) * 55.0) as u32;
        let segment_duration_sec = (segment.end_timestamp_ms - segment.start_timestamp_ms) / 1000.0;
        emit_progress(
            &app,
            &meeting_id,
            "transcribing",
            progress,
            &format!(
                "Transcribing segment {} of {} ({:.1}s)...",
                i + 1,
                processable_count,
                segment_duration_sec
            ),
        );

        // Skip very short segments (< 100ms of audio = 1600 samples at 16kHz)
        if segment.samples.len() < 1600 {
            debug!("Skipping short segment {} with {} samples", i, segment.samples.len());
            continue;
        }

        // Transcribe this segment — dispatch by backend
        // Level 3 设计: sherpa 分支返回 ((text, conf), Option<(tokens, timestamps)>) —
        // 把 token/timestamp 直接返回给外层, 避免任何 Mutex 共享状态.
        let (text, conf, sherpa_tokens_ts) = if let Some(backend) = sherpa_backend.as_deref() {
            // sherpa-onnx: per-segment call to daemon (each VAD segment <=25s, aligned with model window)
            // daemon.transcribe_blocking is sync I/O — use block_in_place to avoid blocking the tokio worker pool
            // (SherpaHandle is !Send so we can't use spawn_blocking; block_in_place keeps it on this thread).
            let pcm_bytes: Vec<u8> = segment.samples.iter().flat_map(|s| s.to_le_bytes()).collect();
            let pcm_b64 = base64::engine::general_purpose::STANDARD.encode(&pcm_bytes);
            debug!(
                "Sherpa segment {}/{}: backend={} samples={} ({:.2}s)",
                i + 1, processable_count, backend, segment.samples.len(), segment_duration_sec
            );
            let seg_idx = i + 1;
            let result: anyhow::Result<crate::audio::sherpa_daemon::SherpaResponse> = tokio::task::block_in_place(|| {
                let daemon = crate::audio::sherpa_daemon::global();
                // Level 3: 总是请求 timestamps (RAM<8GB 时 daemon 端自动降级为不返回)
                daemon.transcribe_blocking(backend, &pcm_b64, 16000, true, hotwords_pack_str, hotwords_custom_str, None, None)
            });
            match result {
                Ok(resp) if !resp.text.trim().is_empty() => {
                    debug!(
                        "Sherpa seg {}/{} OK: chars={} conf={:.2} tokens={} timestamps={}",
                        seg_idx, processable_count,
                        resp.text.chars().count(), resp.confidence,
                        resp.tokens.len(), resp.timestamps.len()
                    );
                    let ts = if !resp.tokens.is_empty() && !resp.timestamps.is_empty() {
                        Some((resp.tokens, resp.timestamps))
                    } else {
                        None
                    };
                    (resp.text, resp.confidence.max(0.0), ts)
                }
                Ok(_) => (String::new(), 0.0, None),
                Err(e) => {
                    warn!(
                        "Sherpa daemon seg {}/{} failed: {} (skip segment)",
                        seg_idx, processable_count, e
                    );
                    (String::new(), 0.0, None)
                }
            }
        } else if use_parakeet {
            let engine = parakeet_engine.as_ref().unwrap();
            let text = engine
                .transcribe_audio(segment.samples.clone())
                .await
                .map_err(|e| anyhow!("Parakeet transcription failed on segment {}: {}", i, e))?;
            (text, 0.9f32, None)
        } else {
            let engine = whisper_engine.as_ref().unwrap();
            let (text, conf, _) = engine
                .transcribe_audio_with_confidence(segment.samples.clone(), language.clone())
                .await
                .map_err(|e| anyhow!("Whisper transcription failed on segment {}: {}", i, e))?;
            (text, conf, None)
        };

        // Skip empty transcripts
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            debug!(
                "Segment {}/{}: {:.1}s, conf={:.2}, text='{}'",
                i + 1, processable_count, segment_duration_sec, conf,
                if trimmed.len() > 80 { let mut end = 80; while !trimmed.is_char_boundary(end) { end -= 1; } &trimmed[..end] } else { trimmed }
            );

            // Level 3 (sherpa only): 用字级 timestamps 切 sub-segments.
            // whisper/parakeet 没有 token timestamps, 直接 push 整段.
            if let Some((toks, tss)) = sherpa_tokens_ts {
                let sub_segs = split_text_by_timestamps(
                    &text,
                    &toks,
                    &tss,
                    segment.start_timestamp_ms,
                );
                debug!(
                    "  Level 3 split: 1 VAD seg -> {} sub-segments (tokens={})",
                    sub_segs.len(), toks.len()
                );
                for (sub_text, sub_start, sub_end) in sub_segs {
                    all_transcripts.push((correct_industry_terms_with_known(&sub_text, &runtime_hotword_terms(), L3Config::default()), sub_start, sub_end));
                }
                total_confidence += conf;
                // toks/tss 在这里 drop (作用域结束)
            } else {
                all_transcripts.push((correct_industry_terms_with_known(&text, &runtime_hotword_terms(), L3Config::default()), segment.start_timestamp_ms, segment.end_timestamp_ms));
                total_confidence += conf;
            }
        } else {
            debug!("Segment {}/{}: {:.1}s — empty transcription", i + 1, processable_count, segment_duration_sec);
        }
    }

    let transcribed_count = all_transcripts.len();
    let avg_confidence = if transcribed_count > 0 {
        total_confidence / transcribed_count as f32
    } else {
        0.0
    };

    info!(
        "Transcription complete: {} segments transcribed out of {}, avg confidence: {:.2}",
        transcribed_count, processable_count, avg_confidence
    );

    let speech_ms: f64 = speech_segments
        .iter()
        .map(|segment| segment.end_timestamp_ms - segment.start_timestamp_ms)
        .sum();
    let speech_coverage = if duration_seconds > 0.0 {
        (speech_ms / 1000.0 / duration_seconds).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let transcript_chars = all_transcripts
        .iter()
        .map(|(text, _, _)| text.chars().count())
        .sum();
    let average_segment_seconds = if transcribed_count > 0 {
        all_transcripts
            .iter()
            .map(|(_, start, end)| (end - start) / 1000.0)
            .sum::<f64>()
            / transcribed_count as f64
    } else {
        0.0
    };
    let low_quality_warning = if transcribed_count == 0 {
        Some("没有识别出有效语音，请检查麦克风、录音音量或模型状态".to_string())
    } else if speech_coverage > 0.05 && transcript_chars < (speech_ms / 1000.0 * 1.5) as usize {
        Some("识别文本密度偏低，建议检查录音音量或使用更高质量模型".to_string())
    } else {
        None
    };
    info!(
        "Retranscription quality: coverage={:.1}%, chars={}, avg_segment={:.2}s, warning={:?}",
        speech_coverage * 100.0,
        transcript_chars,
        average_segment_seconds,
        low_quality_warning
    );

    // Check for cancellation
    if RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst) {
        return Err(anyhow!("Retranscription cancelled"));
    }

    // 离线会记 W2.2: sherpa 段已在上面 for-loop 内逐段处理完成,这里直接进入保存阶段
    emit_progress(&app, &meeting_id, "saving", 80, "Saving transcripts...");

    // Create transcript segments with proper timestamps from VAD
    let segments = create_transcript_segments(&all_transcripts);

    // Save to database
    let app_state = app
        .try_state::<AppState>()
        .ok_or_else(|| anyhow!("App state not available"))?;

    // Wrap delete+insert+update in a transaction to prevent data loss
    let pool = app_state.db_manager.pool();
    let existing_rows: Vec<(String, Option<f64>, Option<f64>)> = sqlx::query_as(
        "SELECT transcript, audio_start_time, audio_end_time FROM transcripts WHERE meeting_id = ? ORDER BY audio_start_time ASC"
    )
    .bind(&meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|e| anyhow!("Failed to inspect existing transcripts: {}", e))?;
    let existing_duration_sum: f64 = existing_rows.iter().map(|(_, start, end)| match (start, end) {
        (Some(start), Some(end)) if end > start => end - start,
        _ => 0.0,
    }).sum();
    let existing = ExistingTranscriptStats {
        segments: existing_rows.len(),
        chars: existing_rows.iter().map(|(text, _, _)| text.chars().count()).sum(),
        average_segment_seconds: if existing_rows.is_empty() { 0.0 } else { existing_duration_sum / existing_rows.len() as f64 },
        texts: existing_rows.iter().map(|(text, _, _)| text.clone()).collect(),
    };
    let candidate_texts: Vec<String> = segments.iter().map(|segment| segment.text.clone()).collect();
    if let Err(reason) = should_apply_retranscription(
        &existing,
        segments.len(),
        transcript_chars,
        average_segment_seconds,
        &candidate_texts,
        low_quality_warning.as_deref(),
    ) {
        warn!("Retranscription rejected for meeting {}: {}", meeting_id, reason);
        emit_progress(&app, &meeting_id, "complete", 100, "Original transcript preserved");
        return Ok(RetranscriptionResult {
            meeting_id,
            segments_count: existing.segments,
            duration_seconds,
            language,
            speech_coverage,
            transcript_chars: existing.chars,
            average_segment_seconds: existing.average_segment_seconds,
            low_quality_warning: Some(reason),
            applied: false,
        });
    }
    let mut conn = pool.acquire().await.map_err(|e| anyhow!("DB error: {}", e))?;
    let mut tx = sqlx::Connection::begin(&mut *conn)
        .await
        .map_err(|e| anyhow!("Failed to start transaction: {}", e))?;

    sqlx::query("DELETE FROM transcripts WHERE meeting_id = ?")
        .bind(&meeting_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow!("Failed to delete existing transcripts: {}", e))?;

    for segment in &segments {
        sqlx::query(
            "INSERT OR IGNORE INTO transcripts (id, meeting_id, transcript, timestamp, audio_start_time, audio_end_time, duration)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&segment.id)
        .bind(&meeting_id)
        .bind(&segment.text)
        .bind(&segment.timestamp)
        .bind(segment.audio_start_time)
        .bind(segment.audio_end_time)
        .bind(segment.duration)
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow!("Failed to insert transcript: {}", e))?;
    }

    tx.commit().await
        .map_err(|e| anyhow!("Failed to commit transaction: {}", e))?;

    info!(
        "Updated {} transcripts for meeting {} in transaction",
        segments.len(),
        meeting_id
    );

    // Write updated transcripts.json and metadata.json to the meeting folder
    emit_progress(&app, &meeting_id, "saving", 90, "Writing transcript files...");

    if let Err(e) = write_transcripts_json(&folder_path, &segments) {
        warn!("Failed to write transcripts.json: {}", e);
    }

    // Find audio filename for metadata
    let audio_filename = audio_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.mp4")
        .to_string();

    if let Err(e) = write_retranscription_metadata(
        &folder_path,
        &meeting_id,
        duration_seconds,
        &audio_filename,
    ) {
        warn!("Failed to update metadata.json: {}", e);
    }

    emit_progress(&app, &meeting_id, "complete", 100, "Retranscription complete");

    Ok(RetranscriptionResult {
        meeting_id,
        segments_count: segments.len(),
        duration_seconds,
        language,
        speech_coverage,
        transcript_chars,
        average_segment_seconds,
        low_quality_warning,
        applied: true,
    })
}

/// Emit progress event
fn emit_progress<R: Runtime>(
    app: &AppHandle<R>,
    meeting_id: &str,
    stage: &str,
    progress: u32,
    message: &str,
) {
    let _ = app.emit(
        "retranscription-progress",
        RetranscriptionProgress {
            meeting_id: meeting_id.to_string(),
            stage: stage.to_string(),
            progress_percentage: progress,
            message: message.to_string(),
        },
    );
}

/// Get or initialize the Whisper engine, auto-loading the model if needed
/// If `requested_model` is provided, ensures that specific model is loaded
async fn get_or_init_whisper<R: Runtime>(
    app: &AppHandle<R>,
    requested_model: Option<&str>,
) -> Result<Arc<WhisperEngine>> {
    use crate::whisper_engine::commands::WHISPER_ENGINE;

    let engine = {
        let guard = WHISPER_ENGINE.lock().unwrap_or_else(|e| e.into_inner());
        guard.as_ref().cloned()
    };

    match engine {
        Some(e) => {
            // Determine which model to use
            let target_model = match requested_model {
                Some(model) => model.to_string(),
                None => get_configured_whisper_model(app).await?,
            };

            // Check if the correct model is already loaded
            let current_model = e.get_current_model().await;
            let needs_load = match &current_model {
                Some(loaded) => loaded != &target_model,
                None => true,
            };

            if needs_load {
                info!(
                    "Loading Whisper model '{}' (current: {:?})",
                    target_model, current_model
                );

                // Discover available models first (populates the internal cache)
                info!("Discovering available Whisper models...");
                if let Err(discover_err) = e.discover_models().await {
                    warn!("Error during model discovery (continuing anyway): {}", discover_err);
                }

                match e.load_model(&target_model).await {
                    Ok(_) => {
                        info!("Whisper model '{}' loaded successfully", target_model);
                        Ok(e)
                    }
                    Err(load_err) => {
                        error!("Failed to load Whisper model '{}': {}", target_model, load_err);
                        Err(anyhow!("Failed to load Whisper model '{}': {}", target_model, load_err))
                    }
                }
            } else {
                info!("Whisper model '{}' already loaded", target_model);
                Ok(e)
            }
        }
        None => Err(anyhow!("Whisper engine not initialized")),
    }
}

/// Get the configured Whisper model name from the database
async fn get_configured_whisper_model<R: Runtime>(app: &AppHandle<R>) -> Result<String> {
    debug!("Getting configured Whisper model from database...");

    let app_state = app
        .try_state::<AppState>()
        .ok_or_else(|| {
            error!("App state not available");
            anyhow!("App state not available")
        })?;

    debug!("Querying transcript_settings table...");

    // Query the transcript settings from the database - get both provider and model
    let result: Option<(String, String)> = sqlx::query_as(
        "SELECT provider, model FROM transcript_settings WHERE id = '1'"
    )
    .fetch_optional(app_state.db_manager.pool())
    .await
    .map_err(|e| {
        error!("Failed to query transcript config: {}", e);
        anyhow!("Failed to query transcript config: {}", e)
    })?;

    match result {
        Some((provider, model)) => {
            info!("Found transcript config: provider={}, model={}", provider, model);

            // Check if provider is Whisper-based
            if provider == "localWhisper" || provider == "whisper" {
                Ok(model)
            } else {
                error!("Retranscription requires Whisper provider, but configured provider is: {}", provider);
                Err(anyhow!("Retranscription requires Whisper. Current provider '{}' does not support retranscription with language selection.", provider))
            }
        },
        None => {
            // Default to configured Whisper model if no config exists
            warn!("No transcript config found, using default model '{}'", DEFAULT_WHISPER_MODEL);
            Ok(DEFAULT_WHISPER_MODEL.to_string())
        }
    }
}

/// Get or initialize the Parakeet engine, auto-loading the model if needed
async fn get_or_init_parakeet<R: Runtime>(
    app: &AppHandle<R>,
    requested_model: Option<&str>,
) -> Result<Arc<ParakeetEngine>> {
    use crate::parakeet_engine::commands::PARAKEET_ENGINE;

    let engine = {
        let guard = PARAKEET_ENGINE.lock().unwrap_or_else(|e| e.into_inner());
        guard.as_ref().cloned()
    };

    match engine {
        Some(e) => {
            // Determine which model to use
            let target_model = match requested_model {
                Some(model) => model.to_string(),
                None => get_configured_parakeet_model(app).await?,
            };

            // Check if the correct model is already loaded
            let current_model = e.get_current_model().await;
            let needs_load = match &current_model {
                Some(loaded) => loaded != &target_model,
                None => true,
            };

            if needs_load {
                info!(
                    "Loading Parakeet model '{}' (current: {:?})",
                    target_model, current_model
                );

                // Discover available models first
                info!("Discovering available Parakeet models...");
                if let Err(discover_err) = e.discover_models().await {
                    warn!("Error during Parakeet model discovery (continuing anyway): {}", discover_err);
                }

                match e.load_model(&target_model).await {
                    Ok(_) => {
                        info!("Parakeet model '{}' loaded successfully", target_model);
                        Ok(e)
                    }
                    Err(load_err) => {
                        error!("Failed to load Parakeet model '{}': {}", target_model, load_err);
                        Err(anyhow!("Failed to load Parakeet model '{}': {}", target_model, load_err))
                    }
                }
            } else {
                info!("Parakeet model '{}' already loaded", target_model);
                Ok(e)
            }
        }
        None => Err(anyhow!("Parakeet engine not initialized")),
    }
}

/// Get the configured Parakeet model name from the database
async fn get_configured_parakeet_model<R: Runtime>(app: &AppHandle<R>) -> Result<String> {
    debug!("Getting configured Parakeet model from database...");

    let app_state = app
        .try_state::<AppState>()
        .ok_or_else(|| {
            error!("App state not available");
            anyhow!("App state not available")
        })?;

    // Query the transcript settings from the database
    let result: Option<(String, String)> = sqlx::query_as(
        "SELECT provider, model FROM transcript_settings WHERE id = '1'"
    )
    .fetch_optional(app_state.db_manager.pool())
    .await
    .map_err(|e| {
        error!("Failed to query transcript config: {}", e);
        anyhow!("Failed to query transcript config: {}", e)
    })?;

    match result {
        Some((provider, model)) => {
            info!("Found transcript config: provider={}, model={}", provider, model);

            if provider == "parakeet" {
                Ok(model)
            } else {
                // Default to configured Parakeet model
                warn!("Configured provider is not Parakeet, using default model");
                Ok(DEFAULT_PARAKEET_MODEL.to_string())
            }
        },
        None => {
            // Default to configured Parakeet model if no config exists
            warn!("No transcript config found, using default Parakeet model");
            Ok(DEFAULT_PARAKEET_MODEL.to_string())
        }
    }
}

/// Write or update metadata.json for retranscription (preserves existing fields, adds retranscribed_at)
fn write_retranscription_metadata(
    folder: &Path,
    meeting_id: &str,
    duration_seconds: f64,
    audio_filename: &str,
) -> Result<()> {
    let metadata_path = folder.join("metadata.json");
    let temp_path = folder.join(".metadata.json.tmp");
    let now = chrono::Utc::now().to_rfc3339();

    // Try to read existing metadata and update it
    let json = if metadata_path.exists() {
        let existing = std::fs::read_to_string(&metadata_path)?;
        let mut value: serde_json::Value = serde_json::from_str(&existing)?;
        if let Some(obj) = value.as_object_mut() {
            obj.insert("retranscribed_at".to_string(), serde_json::json!(now));
            obj.insert("status".to_string(), serde_json::json!("completed"));
            obj.insert("transcript_file".to_string(), serde_json::json!("transcripts.json"));
            obj.remove("detected_summary_language");
        }
        value
    } else {
        serde_json::json!({
            "version": "1.0",
            "meeting_id": meeting_id,
            "created_at": now,
            "completed_at": now,
            "retranscribed_at": now,
            "duration_seconds": duration_seconds,
            "audio_file": audio_filename,
            "transcript_file": "transcripts.json",
            "status": "completed",
            "source": "retranscription"
        })
    };

    let json_string = serde_json::to_string_pretty(&json)?;
    std::fs::write(&temp_path, &json_string)?;
    std::fs::rename(&temp_path, &metadata_path)?;

    info!("Wrote metadata.json to {}", metadata_path.display());
    Ok(())
}

// Tauri commands

/// Response when retranscription is started
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetranscriptionStarted {
    pub meeting_id: String,
    pub message: String,
}

// Start retranscription (Beta gated using configContext.betaFeatures)
#[tauri::command]
pub async fn start_retranscription_command<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
    meeting_folder_path: String,
    language: Option<String>,
    model: Option<String>,
    provider: Option<String>,
) -> Result<RetranscriptionStarted, String> {

    // Check if retranscription is already in progress (guard will be acquired in start_retranscription)
    if RETRANSCRIPTION_IN_PROGRESS.load(Ordering::SeqCst) {
        return Err("Retranscription already in progress".to_string());
    }

    // Clone values for the spawned task
    let meeting_id_clone = meeting_id.clone();

    // Spawn the retranscription in a background task
    tauri::async_runtime::spawn(async move {
        let result = start_retranscription(
            app,
            meeting_id_clone,
            meeting_folder_path,
            language,
            model,
            provider,
        )
        .await;

        // Errors are already emitted as events in start_retranscription
        // so we just log here for debugging
        if let Err(e) = result {
            error!("Retranscription failed: {}", e);
        }
    });

    Ok(RetranscriptionStarted {
        meeting_id,
        message: "Retranscription started".to_string(),
    })
}

#[tauri::command]
pub async fn cancel_retranscription_command() -> Result<(), String> {
    if !is_retranscription_in_progress() {
        return Err("No retranscription in progress".to_string());
    }
    cancel_retranscription();
    Ok(())
}

#[tauri::command]
pub async fn is_retranscription_in_progress_command() -> bool {
    is_retranscription_in_progress()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_transcript_segments_empty() {
        let transcripts: Vec<(String, f64, f64)> = vec![];
        let segments = create_transcript_segments(&transcripts);
        assert!(segments.is_empty());
    }

    #[test]
    fn test_create_transcript_segments_single() {
        let transcripts = vec![
            ("Hello world".to_string(), 0.0, 1500.0), // 0-1.5 seconds
        ];
        let segments = create_transcript_segments(&transcripts);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "Hello world");
        assert_eq!(segments[0].audio_start_time, Some(0.0));
        assert_eq!(segments[0].audio_end_time, Some(1.5));
        assert_eq!(segments[0].duration, Some(1.5));
    }

    #[test]
    fn test_create_transcript_segments_multiple() {
        let transcripts = vec![
            ("First segment".to_string(), 0.0, 2000.0),      // 0-2 seconds
            ("Second segment".to_string(), 3000.0, 5000.0),  // 3-5 seconds
            ("Third segment".to_string(), 6500.0, 8000.0),   // 6.5-8 seconds
        ];
        let segments = create_transcript_segments(&transcripts);

        assert_eq!(segments.len(), 3);

        // First segment
        assert_eq!(segments[0].text, "First segment");
        assert_eq!(segments[0].audio_start_time, Some(0.0));
        assert_eq!(segments[0].audio_end_time, Some(2.0));
        assert_eq!(segments[0].duration, Some(2.0));

        // Second segment
        assert_eq!(segments[1].text, "Second segment");
        assert_eq!(segments[1].audio_start_time, Some(3.0));
        assert_eq!(segments[1].audio_end_time, Some(5.0));
        assert_eq!(segments[1].duration, Some(2.0));

        // Third segment
        assert_eq!(segments[2].text, "Third segment");
        assert_eq!(segments[2].audio_start_time, Some(6.5));
        assert_eq!(segments[2].audio_end_time, Some(8.0));
        assert_eq!(segments[2].duration, Some(1.5));
    }

    #[test]
    fn test_create_transcript_segments_trims_whitespace() {
        let transcripts = vec![
            ("  Hello with spaces  ".to_string(), 0.0, 1000.0),
        ];
        let segments = create_transcript_segments(&transcripts);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "Hello with spaces");
    }

    #[test]
    fn test_create_transcript_segments_generates_unique_ids() {
        let transcripts = vec![
            ("Segment one".to_string(), 0.0, 1000.0),
            ("Segment two".to_string(), 1000.0, 2000.0),
        ];
        let segments = create_transcript_segments(&transcripts);

        assert_eq!(segments.len(), 2);
        assert_ne!(segments[0].id, segments[1].id);
        assert!(segments[0].id.starts_with("transcript-"));
        assert!(segments[1].id.starts_with("transcript-"));
    }

    #[test]
    fn test_cancellation_flag() {
        // Reset flag to known state
        RETRANSCRIPTION_CANCELLED.store(false, Ordering::SeqCst);
        RETRANSCRIPTION_IN_PROGRESS.store(false, Ordering::SeqCst);

        assert!(!is_retranscription_in_progress());

        // Test cancellation
        cancel_retranscription();
        assert!(RETRANSCRIPTION_CANCELLED.load(Ordering::SeqCst));

        // Reset for other tests
        RETRANSCRIPTION_CANCELLED.store(false, Ordering::SeqCst);
    }

    #[test]
    fn test_vad_redemption_time_adapts_to_recording_length() {
        assert_eq!(vad_redemption_time_ms(30.0), 700);
        assert_eq!(vad_redemption_time_ms(90.0), 700);
        assert_eq!(vad_redemption_time_ms(91.0), 1200);
        assert_eq!(vad_redemption_time_ms(900.0), 1200);
        assert_eq!(vad_redemption_time_ms(901.0), 1600);
    }

    #[test]
    fn test_find_audio_file_common_candidates() {
        let dir = tempfile::tempdir().unwrap();

        // No audio file → error
        assert!(find_audio_file(dir.path()).is_err());

        // Create audio.mp4 — should be found first
        std::fs::write(dir.path().join("audio.mp4"), b"fake").unwrap();
        let found = find_audio_file(dir.path()).unwrap();
        assert_eq!(found.file_name().unwrap(), "audio.mp4");
    }

    #[test]
    fn test_find_audio_file_non_mp4_extensions() {
        let dir = tempfile::tempdir().unwrap();

        // Create audio.wav (imported as .wav, not .mp4)
        std::fs::write(dir.path().join("audio.wav"), b"fake").unwrap();
        let found = find_audio_file(dir.path()).unwrap();
        assert_eq!(found.file_name().unwrap(), "audio.wav");
    }

    #[test]
    fn test_find_audio_file_fallback_scan() {
        let dir = tempfile::tempdir().unwrap();

        // Create a file with an audio extension but non-standard name
        std::fs::write(dir.path().join("my_recording.flac"), b"fake").unwrap();
        // Also add a non-audio file that should be ignored
        std::fs::write(dir.path().join("notes.txt"), b"text").unwrap();

        let found = find_audio_file(dir.path()).unwrap();
        assert_eq!(found.file_name().unwrap(), "my_recording.flac");
    }

    #[test]
    fn test_find_audio_file_priority_order() {
        let dir = tempfile::tempdir().unwrap();

        // Create both audio.m4a and audio.mp4 — mp4 should win (listed first in candidates)
        std::fs::write(dir.path().join("audio.m4a"), b"fake").unwrap();
        std::fs::write(dir.path().join("audio.mp4"), b"fake").unwrap();
        let found = find_audio_file(dir.path()).unwrap();
        assert_eq!(found.file_name().unwrap(), "audio.mp4");
    }

    #[test]
    fn test_find_audio_file_empty_folder() {
        let dir = tempfile::tempdir().unwrap();
        let result = find_audio_file(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No audio file found"));
    }

    #[test]
    fn test_find_audio_file_nonexistent_folder() {
        let result = find_audio_file(Path::new("/nonexistent/path/12345"));
        assert!(result.is_err());
    }

    #[test]
    fn test_audio_extensions_constant() {
        // Verify all expected formats are covered
        assert!(AUDIO_EXTENSIONS.contains(&"mp4"));
        assert!(AUDIO_EXTENSIONS.contains(&"m4a"));
        assert!(AUDIO_EXTENSIONS.contains(&"wav"));
        assert!(AUDIO_EXTENSIONS.contains(&"mp3"));
        assert!(AUDIO_EXTENSIONS.contains(&"flac"));
        assert!(AUDIO_EXTENSIONS.contains(&"ogg"));
        assert!(AUDIO_EXTENSIONS.contains(&"aac"));
        // FFmpeg-backed formats
        assert!(AUDIO_EXTENSIONS.contains(&"mkv"));
        assert!(AUDIO_EXTENSIONS.contains(&"webm"));
        assert!(AUDIO_EXTENSIONS.contains(&"wma"));
        // Non-audio formats
        assert!(!AUDIO_EXTENSIONS.contains(&"txt"));
        assert!(!AUDIO_EXTENSIONS.contains(&"pdf"));
    }
}

/// Level 3: 按字级 timestamps 把一段 text 切成多 sub-segments.
/// 切点规则:
///   1. token 字符是 `。！？，；` → 强制切
///   2. 当前 token 与下一个 token 时间戳差 > 0.5s → 切 (长停顿)
///   3. token 是空格 + 下一个时间戳差 > 0.3s → 切 (英文短语边界)
/// 太短的 sub-segment (< 200ms) 合并到下一个.
/// 返回 Vec<(text, start_ms, end_ms)>.
/// `segment_offset_ms`: 本段在整音频中的起点 (用于把 token 时间戳转绝对毫秒).
fn split_text_by_timestamps(
    text: &str,
    tokens: &[String],
    timestamps: &[f32],
    segment_offset_ms: f64,
) -> Vec<(String, f64, f64)> {
    // 兜底: 无 token/timestamps 或长度不匹配, 直接返回整段
    if tokens.is_empty() || timestamps.is_empty() || tokens.len() != timestamps.len() {
        return vec![(text.to_string(), segment_offset_ms, segment_offset_ms + (text.len() as f64 * 50.0))];
    }
    let mut out: Vec<(String, f64, f64)> = Vec::new();
    let mut cur_text = String::new();
    let mut cur_start_ms: Option<f64> = None;
    let mut cur_end_ms: f64 = 0.0;

    let punct_end = "。！？";  // 强制句末切
    let _punct_comma = "，；、";  // 强制短语切
    let pause_long_ms = 500.0_f32;   // >0.5s 停顿切
    let pause_short_ms = 300.0_f32;  // 空格 + >0.3s 停顿切

    for i in 0..tokens.len() {
        let tok = &tokens[i];
        let ts_sec = timestamps[i] as f64;
        let ts_ms = ts_sec * 1000.0;

        // 起始: 第一个 token
        if cur_start_ms.is_none() {
            cur_start_ms = Some(segment_offset_ms + ts_ms);
        }
        cur_text.push_str(tok);
        cur_end_ms = segment_offset_ms + ts_ms;

        // 判断是否要在此 token 后切
        let mut should_split = false;
        let last_char = tok.chars().last().unwrap_or(' ');

        // 规则 1: 强标点
        if punct_end.contains(last_char) {
            should_split = true;
        }
        // 规则 2: 长停顿 (下一 token 时间差)
        else if i + 1 < tokens.len() {
            let next_ts = timestamps[i + 1] as f64;
            let gap_ms = (next_ts * 1000.0) - ts_ms;
            if gap_ms > pause_long_ms as f64 {
                should_split = true;
            }
            // 规则 3: 空格 + 中等停顿
            else if last_char == ' ' && gap_ms > pause_short_ms as f64 {
                should_split = true;
            }
            // 规则 1b: 逗号类标点 (短语切, 但不强制)
            // 暂不强制, 避免过度切; 如需可加
        }

        if should_split {
            let start_ms = cur_start_ms.unwrap_or(segment_offset_ms);
            // 兜底: end 不能 < start
            let end_ms = if cur_end_ms > start_ms { cur_end_ms } else { start_ms + 100.0 };
            let trimmed = cur_text.trim().to_string();
            if !trimmed.is_empty() {
                out.push((trimmed, start_ms, end_ms));
            }
            cur_text.clear();
            cur_start_ms = None;
            cur_end_ms = 0.0;
        }
    }

    // flush 残余
    if !cur_text.trim().is_empty() {
        let start_ms = cur_start_ms.unwrap_or(segment_offset_ms);
        let end_ms = if cur_end_ms > start_ms { cur_end_ms } else { start_ms + 100.0 };
        out.push((cur_text.trim().to_string(), start_ms, end_ms));
    }

    // 合并过短 sub-segments (<200ms 或 <2 字符)
    let min_dur_ms = 200.0_f64;
    let mut merged: Vec<(String, f64, f64)> = Vec::new();
    for (txt, s, e) in out {
        if let Some(last) = merged.last_mut() {
            let last_dur = last.2 - last.1;
            let cur_dur = e - s;
            // 合并条件: 上一个 < min_dur_ms 或 (上一个 < 500ms 且当前 < min_dur_ms)
            if last_dur < min_dur_ms || (last_dur < 500.0 && cur_dur < min_dur_ms) {
                last.0.push(' ');
                last.0.push_str(&txt);
                last.2 = e;
                continue;
            }
        }
        merged.push((txt, s, e));
    }
    merged
}

#[cfg(test)]
mod quality_gate_tests {
    use super::*;

    #[test]
    fn rejects_candidate_that_loses_most_text() {
        let existing = ExistingTranscriptStats { segments: 8, chars: 200, average_segment_seconds: 4.0, texts: vec!["原告主张合同违约金".into(); 8] };
        assert!(should_apply_retranscription(&existing, 8, 100, 4.0, &vec!["原告主张".to_string(); 8], None).is_err());
    }

    #[test]
    fn rejects_candidate_that_destroys_timestamp_granularity() {
        let existing = ExistingTranscriptStats { segments: 10, chars: 200, average_segment_seconds: 3.0, texts: vec!["仲裁条款合法有效".into(); 10] };
        assert!(should_apply_retranscription(&existing, 3, 190, 12.0, &vec!["仲裁条款合法有效".to_string(); 3], None).is_err());
    }

    #[test]
    fn accepts_candidate_with_comparable_content_and_segments() {
        let existing = ExistingTranscriptStats { segments: 8, chars: 200, average_segment_seconds: 4.0, texts: vec!["原告主张合同违约金".into(); 8] };
        assert!(should_apply_retranscription(&existing, 7, 210, 5.0, &vec!["原告主张合同违约金并申请撤销仲裁裁决".to_string(); 7], None).is_ok());
    }

    #[test]
    fn rejects_screenshot_regression_with_fragment_explosion() {
        let existing = ExistingTranscriptStats {
            segments: 3,
            chars: 118,
            average_segment_seconds: 12.0,
            texts: vec![
                "本案原告主张被告违反合同约定，要求解除合同并支付违约金。被告提出管辖权异议，认为案件应当提交北京仲裁委员会处理".into(),
                "法院审查后认为，仲裁条款合法有效，双方应先履行仲裁程序。如果任何一方对仲裁裁决不服，可以依法申请撤销，但不能直接向中级人民法院提起上诉".into(),
                "请记录原告、被告、违约金、管辖权异议、仲裁条款和仲裁裁决".into(),
            ],
        };
        let degraded = vec![
            "，本案原告主张被告违反合同约定，要求解除合同并支付违约金".into(),
            "啊。".into(),
            "被告提出管辖权异议，认为案件应当提交北京仲裁委员会处理。".into(),
            "法院审查后认为，仲裁条款合法有效，双方应先履行仲裁程序。".into(),
            "如果任何一方对仲裁".into(),
            "裁决不服。".into(),
            "可以依法申请撤销".into(),
            "。".into(),
            "但不能直接向中级人民法院提起".into(),
            "上诉，请进入原告被告违约金管辖权异议、仲裁".into(),
            "条款和".into(),
            "仲裁裁决".into(),
            "。".into(),
        ];
        assert!(should_apply_retranscription(&existing, degraded.len(), 126, 3.0, &degraded, None).is_err());
    }
}
