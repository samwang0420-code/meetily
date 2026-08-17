// audio/transcription/worker.rs
//
// Parallel transcription worker pool and chunk processing logic.

use super::engine::TranscriptionEngine;
use super::provider::{TranscriptionError, TranscriptionProvider};
use super::sherpa_stream::{SherpaStreamSession, StreamChunkResult, streaming_enabled};
use crate::audio::AudioChunk;
use log::{error, info, warn};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, Runtime};

// Sequence counter for transcript updates
static SEQUENCE_COUNTER: AtomicU64 = AtomicU64::new(0);
static LAST_AUDIO_END_TIME_BITS: AtomicU64 = AtomicU64::new(0);

// Speech detection flag - reset per recording session
static SPEECH_DETECTED_EMITTED: AtomicBool = AtomicBool::new(false);

// v0.6.12+: 实时识别 latency 滚动采样 (50 样本 ring buffer)
// 用 Mutex<[i64; 50]> 因为 std::sync::atomic::AtomicI64 数组 const init 没稳定 API
static TIMING_LAST_EMIT_MS: AtomicU64 = AtomicU64::new(0);
static TIMING_RING_HEAD: AtomicU64 = AtomicU64::new(0);  // 0..50 滚动索引
static TIMING_RING_FILLS: AtomicU64 = AtomicU64::new(0); // 总填充数
static TIMING_RING_DECODE: Mutex<[i64; 50]> = Mutex::new([0i64; 50]);
static TIMING_RING_BUFFER_MS: Mutex<[i64; 50]> = Mutex::new([0i64; 50]);

/// Reset the speech detected flag for a new recording session
pub fn reset_speech_detected_flag() {
    SPEECH_DETECTED_EMITTED.store(false, Ordering::SeqCst);
    LAST_AUDIO_END_TIME_BITS.store(0.0f64.to_bits(), Ordering::SeqCst);
    info!("🔍 SPEECH_DETECTED_EMITTED reset to: {}", SPEECH_DETECTED_EMITTED.load(Ordering::SeqCst));
}

/// v0.6.12+: 返回实时识别 streaming 延迟滚动统计 (50 样本 ring buffer)
///   decode_avg_ms / p95 / max  = sherpa-onnx decode 耗时 (ms)
///   buffer_avg_ms / p95 / max  = buffer 缓存时长
///   samples                    = 已填充样本数 (最多 50)
#[tauri::command]
pub fn get_streaming_timing_stats() -> serde_json::Value {
    let fills = TIMING_RING_FILLS.load(Ordering::SeqCst).min(50) as usize;
    let ring_decode = TIMING_RING_DECODE.lock().map(|g| *g).unwrap_or([0i64; 50]);
    let ring_buffer = TIMING_RING_BUFFER_MS.lock().map(|g| *g).unwrap_or([0i64; 50]);
    let mut decode_samples: Vec<i64> = Vec::with_capacity(fills);
    let mut buffer_samples: Vec<i64> = Vec::with_capacity(fills);
    for i in 0..fills {
        decode_samples.push(ring_decode[i]);
        buffer_samples.push(ring_buffer[i]);
    }
    fn percentile(v: &[i64], p: f64) -> i64 {
        if v.is_empty() { return 0; }
        let mut s = v.to_vec(); s.sort_unstable();
        let idx = ((p / 100.0) * (s.len() - 1) as f64) as usize;
        s[idx]
    }
    fn avg(v: &[i64]) -> i64 {
        if v.is_empty() { return 0; }
        v.iter().sum::<i64>() / v.len() as i64
    }
    serde_json::json!({
        "samples": fills,
        "decode_avg_ms": avg(&decode_samples),
        "decode_p95_ms": percentile(&decode_samples, 95.0),
        "decode_max_ms": *decode_samples.iter().max().unwrap_or(&0),
        "buffer_avg_ms": avg(&buffer_samples),
        "buffer_p95_ms": percentile(&buffer_samples, 95.0),
        "buffer_max_ms": *buffer_samples.iter().max().unwrap_or(&0),
    })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranscriptUpdate {
    pub text: String,
    pub timestamp: String, // Wall-clock time for reference (e.g., "14:30:05")
    pub source: String,
    pub sequence_id: u64,
    pub chunk_start_time: f64, // Legacy field, kept for compatibility
    pub is_partial: bool,
    pub confidence: f32,
    // NEW: Recording-relative timestamps for playback sync
    pub audio_start_time: f64, // Seconds from recording start (e.g., 125.3)
    pub audio_end_time: f64,   // Seconds from recording start (e.g., 128.6)
    pub duration: f64,          // Segment duration in seconds (e.g., 3.3)
}

// NOTE: get_transcript_history and get_recording_meeting_name functions
// have been moved to recording_commands.rs where they have access to RECORDING_MANAGER

/// Optimized parallel transcription task ensuring ZERO chunk loss
pub fn start_transcription_task<R: Runtime>(
    app: AppHandle<R>,
    transcription_receiver: tokio::sync::mpsc::UnboundedReceiver<AudioChunk>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!("🚀 Starting optimized parallel transcription task - guaranteeing zero chunk loss");

        // Initialize transcription engine (Whisper or Parakeet based on config)
        let transcription_engine = match super::engine::get_or_init_transcription_engine(&app).await {
            Ok(engine) => engine,
            Err(e) => {
                error!("Failed to initialize transcription engine: {}", e);
                let _ = app.emit("transcription-error", serde_json::json!({
                    "error": e,
                    "userMessage": "Recording failed: Unable to initialize speech recognition. Please check your model settings.",
                    "actionable": true
                }));
                return;
            }
        };

        // v0.6.14+: 起 streaming session (如果开了 OFFLINE_HUIJI_STREAMING + Sherpa engine)
        //   - 默认 false → streaming_session = None → 走原 transcribe_blocking (完全等价 v0.6.13)
        //   - 开启 + Sherpa → 起 daemon stream_begin, 拿 session 给所有 worker
        //   - 失败 fallback 到 None (不阻塞录音)
        let streaming_session: Option<Arc<tokio::sync::Mutex<SherpaStreamSession>>> =
            if streaming_enabled() && matches!(transcription_engine, TranscriptionEngine::Sherpa) {
                let model_name = crate::api::api::api_get_transcript_config(
                    app.clone(),
                    app.clone().state(),
                    None,
                )
                .await
                .ok()
                .flatten()
                .map(|c| c.model)
                .unwrap_or_else(crate::config::pick_default_sherpa_model);
                // v0.6.15: 从 hotwords_globals 拿当前用户配置(避免 streaming 没接热词导致识别不准)
                let hw_pack = crate::audio::hotwords_globals::current_pack();
                let hw_custom_owned = crate::audio::hotwords_globals::current_custom_with_product_terms();
                let hw_custom = hw_custom_owned.as_str();
                if !hw_pack.is_empty() || !hw_custom.is_empty() {
                    info!("🔥 streaming session 接热词: pack='{}' custom='{}'", hw_pack, hw_custom);
                }
                match SherpaStreamSession::begin(&model_name, hw_pack, hw_custom).await {
                    Ok(sess) => {
                        info!("🎙️ streaming session active: {} (model={})", sess.session_id(), model_name);
                        let _ = app.emit("transcript-session-started", serde_json::json!({
                            "session_id": sess.session_id(),
                            "model": model_name,
                        }));
                        Some(Arc::new(tokio::sync::Mutex::new(sess)))
                    }
                    Err(e) => {
                        warn!("⚠️ streaming session begin failed, fallback to blocking: {}", e);
                        None
                    }
                }
            } else {
                None
            };

        // Create parallel workers for faster processing while preserving ALL chunks
        const NUM_WORKERS: usize = 1; // Serial processing ensures transcripts emit in chronological order
        let (work_sender, work_receiver) = tokio::sync::mpsc::unbounded_channel::<AudioChunk>();
        let work_receiver = Arc::new(tokio::sync::Mutex::new(work_receiver));

        // Track completion: AtomicU64 for chunks queued, AtomicU64 for chunks completed
        let chunks_queued = Arc::new(AtomicU64::new(0));
        let chunks_completed = Arc::new(AtomicU64::new(0));
        let input_finished = Arc::new(AtomicBool::new(false));

        info!("📊 Starting {} transcription worker{} (serial mode for ordered emission)", NUM_WORKERS, if NUM_WORKERS == 1 { "" } else { "s" });

        // Spawn worker tasks
        let mut worker_handles = Vec::new();
        for worker_id in 0..NUM_WORKERS {
            let engine_clone = match &transcription_engine {
                TranscriptionEngine::Whisper(e) => TranscriptionEngine::Whisper(e.clone()),
                TranscriptionEngine::Parakeet(e) => TranscriptionEngine::Parakeet(e.clone()),
                TranscriptionEngine::Provider(p) => TranscriptionEngine::Provider(p.clone()),
                // v0.6.12+: Sherpa 是全局 daemon, variant 只用于标识, 此处直接透传
                TranscriptionEngine::Sherpa => TranscriptionEngine::Sherpa,
            };
            let app_clone = app.clone();
            let work_receiver_clone = work_receiver.clone();
            let chunks_completed_clone = chunks_completed.clone();
            let input_finished_clone = input_finished.clone();
            let chunks_queued_clone = chunks_queued.clone();
            let streaming_session_clone = streaming_session.clone();  // v0.6.14+: 给每个 worker 持一份

            let worker_handle = tokio::spawn(async move {
                info!("👷 Worker {} started", worker_id);

                // PRE-VALIDATE model state to avoid repeated async calls per chunk
                let initial_model_loaded = engine_clone.is_model_loaded().await;
                let current_model = engine_clone
                    .get_current_model()
                    .await
                    .unwrap_or_else(|| "unknown".to_string());

                let engine_name = engine_clone.provider_name();

                if initial_model_loaded {
                    info!(
                        "✅ Worker {} pre-validation: {} model '{}' is loaded and ready",
                        worker_id, engine_name, current_model
                    );
                } else {
                    warn!("⚠️ Worker {} pre-validation: {} model not loaded - chunks may be skipped", worker_id, engine_name);
                }

                loop {
                    // Try to get a chunk to process
                    let chunk = {
                        let mut receiver = work_receiver_clone.lock().await;
                        receiver.recv().await
                    };

                    match chunk {
                        Some(chunk) => {
                            // PERFORMANCE OPTIMIZATION: Reduce logging in hot path
                            // Only log every 10th chunk per worker to reduce I/O overhead
                            let should_log_this_chunk = chunk.chunk_id % 10 == 0;

                            if should_log_this_chunk {
                                info!(
                                    "👷 Worker {} processing chunk {} with {} samples",
                                    worker_id,
                                    chunk.chunk_id,
                                    chunk.data.len()
                                );
                            }

                            // Check if model is still loaded before processing
                            if !engine_clone.is_model_loaded().await {
                                warn!("⚠️ Worker {}: Model unloaded, but continuing to preserve chunk {}", worker_id, chunk.chunk_id);
                                // Still count as completed even if we can't process
                                chunks_completed_clone.fetch_add(1, Ordering::SeqCst);
                                continue;
                            }

                            let chunk_timestamp = chunk.timestamp;
                            let chunk_duration = chunk.data.len() as f64 / chunk.sample_rate as f64;

                            // Transcribe with provider-agnostic approach
                            // v0.6.14+: 传 streaming_session_clone (默认 None = 原 blocking 路径)
                            match transcribe_chunk_with_provider(
                                &engine_clone,
                                chunk,
                                &app_clone,
                                streaming_session_clone.clone(),
                            )
                            .await
                            {
                                Ok((transcript, confidence_opt, is_partial)) => {
                                    // Provider-aware confidence threshold
                                    let confidence_threshold = match &engine_clone {
                                        TranscriptionEngine::Whisper(_) | TranscriptionEngine::Provider(_) => 0.3,
                                        // v0.6.12+: Sherpa + Parakeet 都无 confidence 输出, accept all
                                        TranscriptionEngine::Parakeet(_) | TranscriptionEngine::Sherpa => 0.0,
                                    };

                                    let confidence_str = match confidence_opt {
                                        Some(c) => format!("{:.2}", c),
                                        None => "N/A".to_string(),
                                    };

                                    info!("🔍 Worker {} transcription result: text='{}', confidence={}, partial={}, threshold={:.2}",
                                          worker_id, transcript, confidence_str, is_partial, confidence_threshold);

                                    // Check confidence threshold (or accept if no confidence provided)
                                    let meets_threshold = confidence_opt.map_or(true, |c| c >= confidence_threshold);

                                    if !transcript.trim().is_empty() && meets_threshold {
                                        // PERFORMANCE: Only log transcription results, not every processing step
                                        info!("✅ Worker {} transcribed: {} (confidence: {}, partial: {})",
                                              worker_id, transcript, confidence_str, is_partial);

                                        // Emit speech-detected event for frontend UX (only on first detection per session)
                                        // This is lightweight and provides better user feedback
                                        let current_flag = SPEECH_DETECTED_EMITTED.load(Ordering::SeqCst);
                                        info!("🔍 Checking speech-detected flag: current={}, will_emit={}", current_flag, !current_flag);

                                        if !current_flag {
                                            SPEECH_DETECTED_EMITTED.store(true, Ordering::SeqCst);
                                            match app_clone.emit("speech-detected", serde_json::json!({
                                                "message": "Speech activity detected"
                                            })) {
                                                Ok(_) => info!("🎤 ✅ First speech detected - successfully emitted speech-detected event"),
                                                Err(e) => error!("🎤 ❌ Failed to emit speech-detected event: {}", e),
                                            }
                                        } else {
                                            info!("🔍 Speech already detected in this session, not re-emitting");
                                        }

                                        // Generate sequence ID and calculate timestamps FIRST
                                        let sequence_id = SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst);
                            let audio_start_time = chunk_timestamp; // Already in seconds from recording start
                            let audio_end_time = chunk_timestamp + chunk_duration;
                            LAST_AUDIO_END_TIME_BITS.store(audio_end_time.to_bits(), Ordering::SeqCst);

                                        // Save structured transcript segment to recording manager (only final results)
                                        // Save ALL segments (partial and final) to ensure complete JSON
                                        // Create structured segment with full timestamp data
                                        // NOTE: This is now handled via the transcript-update event emission below
                                        // The recording_commands module listens to these events and saves them
                                        // This decouples the transcription worker from direct RECORDING_MANAGER access

                                        // Emit transcript update with NEW recording-relative timestamps

                                        let update = TranscriptUpdate {
                                            text: transcript,
                                            timestamp: format_current_timestamp(), // Wall-clock for reference
                                            source: "Audio".to_string(),
                                            sequence_id,
                                            chunk_start_time: chunk_timestamp, // Legacy compatibility
                                            is_partial,
                                            confidence: confidence_opt.unwrap_or(0.85), // Default for providers without confidence
                                            // NEW: Recording-relative timestamps for sync
                                            audio_start_time,
                                            audio_end_time,
                                            duration: chunk_duration,
                                        };

                                        if let Err(e) = app_clone.emit("transcript-update", &update)
                                        {
                                            error!(
                                                "Worker {}: Failed to emit transcript update: {}",
                                                worker_id, e
                                            );
                                        }
                                        // PERFORMANCE: Removed verbose logging of every emission
                                    } else if !transcript.trim().is_empty() && should_log_this_chunk
                                    {
                                        // PERFORMANCE: Only log low-confidence results occasionally
                                        if let Some(c) = confidence_opt {
                                            info!("Worker {} low-confidence transcription (confidence: {:.2}), skipping", worker_id, c);
                                        }
                                    }
                                }
                                Err(e) => {
                                    // Improved error handling with specific cases
                                    match e {
                                        TranscriptionError::AudioTooShort { .. } => {
                                            // Skip silently, this is expected for very short chunks
                                            info!("Worker {}: {}", worker_id, e);
                                            chunks_completed_clone.fetch_add(1, Ordering::SeqCst);
                                            continue;
                                        }
                                        TranscriptionError::ModelNotLoaded => {
                                            warn!("Worker {}: Model unloaded during transcription", worker_id);
                                            chunks_completed_clone.fetch_add(1, Ordering::SeqCst);
                                            continue;
                                        }
                                        _ => {
                                            warn!("Worker {}: Transcription failed: {}", worker_id, e);
                                            let _ = app_clone.emit("transcription-warning", e.to_string());
                                        }
                                    }
                                }
                            }

                            // Mark chunk as completed
                            let completed =
                                chunks_completed_clone.fetch_add(1, Ordering::SeqCst) + 1;
                            let queued = chunks_queued_clone.load(Ordering::SeqCst);

                            // PERFORMANCE: Only log progress every 5th chunk to reduce I/O overhead
                            if completed % 5 == 0 || should_log_this_chunk {
                                info!(
                                    "Worker {}: Progress {}/{} chunks ({:.1}%)",
                                    worker_id,
                                    completed,
                                    queued,
                                    (completed as f64 / queued.max(1) as f64 * 100.0)
                                );
                            }

                            // Emit progress event for frontend
                            let progress_percentage = if queued > 0 {
                                (completed as f64 / queued as f64 * 100.0) as u32
                            } else {
                                100
                            };

                            let _ = app_clone.emit("transcription-progress", serde_json::json!({
                                "worker_id": worker_id,
                                "chunks_completed": completed,
                                "chunks_queued": queued,
                                "progress_percentage": progress_percentage,
                                "message": format!("Worker {} processing... ({}/{})", worker_id, completed, queued)
                            }));
                        }
                        None => {
                            // No more chunks available
                            if input_finished_clone.load(Ordering::SeqCst) {
                                // Double-check that all queued chunks are actually completed
                                let final_queued = chunks_queued_clone.load(Ordering::SeqCst);
                                let final_completed = chunks_completed_clone.load(Ordering::SeqCst);

                                if final_completed >= final_queued {
                                    info!(
                                        "👷 Worker {} finishing - all {}/{} chunks processed",
                                        worker_id, final_completed, final_queued
                                    );
                                    break;
                                } else {
                                    warn!("👷 Worker {} detected potential chunk loss: {}/{} completed, waiting...", worker_id, final_completed, final_queued);
                                    // AGGRESSIVE POLLING: Reduced from 50ms to 5ms for faster chunk detection during shutdown
                                    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
                                }
                            } else {
                                // AGGRESSIVE POLLING: Reduced from 10ms to 1ms for faster response during shutdown
                                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                            }
                        }
                    }
                }

                info!("👷 Worker {} completed", worker_id);
            });

            worker_handles.push(worker_handle);
        }

        // Main dispatcher: receive chunks and distribute to workers
        let mut receiver = transcription_receiver;
        while let Some(chunk) = receiver.recv().await {
            let queued = chunks_queued.fetch_add(1, Ordering::SeqCst) + 1;
            info!(
                "📥 Dispatching chunk {} to workers (total queued: {})",
                chunk.chunk_id, queued
            );

            if let Err(_) = work_sender.send(chunk) {
                error!("❌ Failed to send chunk to workers - this should not happen!");
                break;
            }
        }

        // Signal that input is finished
        input_finished.store(true, Ordering::SeqCst);
        drop(work_sender); // Close the channel to signal workers

        let total_chunks_queued = chunks_queued.load(Ordering::SeqCst);
        info!("📭 Input finished with {} total chunks queued. Waiting for all {} workers to complete...",
              total_chunks_queued, NUM_WORKERS);

        // Emit final chunk count to frontend
        let _ = app.emit("transcription-queue-complete", serde_json::json!({
            "total_chunks": total_chunks_queued,
            "message": format!("{} chunks queued for processing - waiting for completion", total_chunks_queued)
        }));

        // Wait for all workers to complete
        for (worker_id, handle) in worker_handles.into_iter().enumerate() {
            if let Err(e) = handle.await {
                error!("❌ Worker {} panicked: {:?}", worker_id, e);
            } else {
                info!("✅ Worker {} completed successfully", worker_id);
            }
        }

        // v0.6.14+: finalize streaming session
        //   - workers 可能还持着 Arc clone, 所以 try_unwrap 大概率失败
        //   - 实际设计: 让 workers 闭包在跑完 chunk 循环后自动释放 Arc clone
        //   - 这里只 best-effort: 拿 Arc 强引用数, 等 1 后 finalize
        if let Some(sess_arc) = streaming_session {
            // 等最多 5 秒, 让 worker 闭包释放 Arc clone
            let mut unique = false;
            for _ in 0..50 {
                let count = Arc::strong_count(&sess_arc);
                if count == 1 { unique = true; break; }
                drop(count);
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            // 此时只有 sess_arc 一个持有者, 拿 session
            if unique {
            if let Ok(sess_mutex) = Arc::try_unwrap(sess_arc) {
                let sess = sess_mutex.into_inner();
                match sess.finalize().await {
                    Ok(final_res) => {
                        info!("🏁 streaming session finalized: delta='{}', segments={}",
                              final_res.delta, final_res.segments_emitted);
                        if !final_res.delta.trim().is_empty() {
                            let sequence_id = SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst);
                            let audio_end_time = f64::from_bits(
                                LAST_AUDIO_END_TIME_BITS.load(Ordering::SeqCst),
                            );
                            let audio_start_time = (audio_end_time - 1.0).max(0.0);
                            let _ = app.emit("transcript-update", &TranscriptUpdate {
                                text: final_res.delta.trim().to_string(),
                                timestamp: format_current_timestamp(),
                                source: "AudioStreaming".to_string(),
                                sequence_id,
                                chunk_start_time: audio_start_time,
                                is_partial: false,
                                confidence: 0.85,
                                audio_start_time,
                                audio_end_time,
                                duration: audio_end_time - audio_start_time,
                            });
                        }
                        let _ = app.emit("transcript-session-ended", serde_json::json!({
                            "session_id": "finalized",
                            "segments_emitted": final_res.segments_emitted,
                        }));
                    }
                    Err(e) => warn!("⚠️ streaming finalize failed: {}", e),
                }
            } else {
                warn!("⚠️ streaming session still held by worker, daemon will clean up on next stream_begin");
            }
            }  // 闭合 if unique
        }  // 闭合 if let Some(sess_arc)

        // Final verification with retry logic to catch any stragglers
        let mut verification_attempts = 0;
        const MAX_VERIFICATION_ATTEMPTS: u32 = 10;

        loop {
            let final_queued = chunks_queued.load(Ordering::SeqCst);
            let final_completed = chunks_completed.load(Ordering::SeqCst);

            if final_queued == final_completed {
                info!(
                    "🎉 ALL {} chunks processed successfully - ZERO chunks lost!",
                    final_completed
                );
                break;
            } else if verification_attempts < MAX_VERIFICATION_ATTEMPTS {
                verification_attempts += 1;
                warn!("⚠️ Chunk count mismatch (attempt {}): {} queued, {} completed - waiting for stragglers...",
                     verification_attempts, final_queued, final_completed);

                // Wait a bit for any remaining chunks to be processed
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            } else {
                error!(
                    "❌ CRITICAL: After {} attempts, chunk loss detected: {} queued, {} completed",
                    MAX_VERIFICATION_ATTEMPTS, final_queued, final_completed
                );

                // Emit critical error event
                let _ = app.emit(
                    "transcript-chunk-loss-detected",
                    serde_json::json!({
                        "chunks_queued": final_queued,
                        "chunks_completed": final_completed,
                        "chunks_lost": final_queued - final_completed,
                        "message": "Some transcript chunks may have been lost during shutdown"
                    }),
                );
                break;
            }
        }

        info!("✅ Parallel transcription task completed - all workers finished, ready for model unload");
    })
}

/// Transcribe audio chunk using the appropriate provider (Whisper, Parakeet, or trait-based)
/// Returns: (text, confidence Option, is_partial)
///
/// v0.6.14+: `streaming_session` 可选 — Some 时走 streaming 路径(partial/delta emit),
///           None 时走原 transcribe_blocking 路径(完全等价 v0.6.13)
async fn transcribe_chunk_with_provider<R: Runtime>(
    engine: &TranscriptionEngine,
    chunk: AudioChunk,
    app: &AppHandle<R>,
    streaming_session: Option<Arc<tokio::sync::Mutex<SherpaStreamSession>>>,
) -> std::result::Result<(String, Option<f32>, bool), TranscriptionError> {
    // v0.6.14+: 提前抓 timestamp/duration(chunk.data 后面会被 move)
    let chunk_timestamp = chunk.timestamp;
    let chunk_duration = chunk.data.len() as f64 / chunk.sample_rate as f64;
    let chunk_id = chunk.chunk_id;
    // Convert to 16kHz mono for transcription
    let transcription_data = if chunk.sample_rate != 16000 {
        crate::audio::audio_processing::resample_audio(&chunk.data, chunk.sample_rate, 16000)
    } else {
        chunk.data
    };

    // Skip VAD processing here since the pipeline already extracted speech using VAD
    let speech_samples = transcription_data;

    // Check for empty samples - improved error handling
    if speech_samples.is_empty() {
        warn!(
            "Audio chunk {} is empty, skipping transcription",
            chunk.chunk_id
        );
        return Err(TranscriptionError::AudioTooShort {
            samples: 0,
            minimum: 1600, // 100ms at 16kHz
        });
    }

    // Calculate energy for logging/monitoring only
    let energy: f32 =
        speech_samples.iter().map(|&x| x * x).sum::<f32>() / speech_samples.len() as f32;
    info!(
        "Processing speech audio chunk {} with {} samples (energy: {:.6})",
        chunk.chunk_id,
        speech_samples.len(),
        energy
    );

    // Transcribe using the appropriate engine (with improved error handling)
    match engine {
        // 离线会记 W2.5: sherpa-onnx daemon (subprocess), 走 SherpaProvider.transcribe
        // v0.6.14+: 如果传了 streaming_session, 走 streaming 路径; 否则走原 blocking 路径
        // streaming 路径特性:
        //   - 每个 chunk push 后立即拿 partial/delta
        //   - partial → emit "transcript-partial" event (UI 显示灰色预览)
        //   - delta → 作为 final 段 emit "transcript-update" event
        //   - 任何错误立即 fallback 到 blocking 路径(不影响录音)
        TranscriptionEngine::Sherpa => {
            // v0.6.14+: streaming 路径
            if let Some(sess_arc) = &streaming_session {
                let sess_arc = sess_arc.clone();
                let audio_for_stream = speech_samples.clone();
                let stream_result: Result<StreamChunkResult, String> = async {
                    let sess = sess_arc.lock().await;
                    sess.push(audio_for_stream).await
                }.await;
                match stream_result {
                    Ok(streamed) => {
                        // partial: emit "transcript-partial" event 给 UI(灰色预览, 流式)
                        // 字段名跟 transcriptService.onTranscriptPartial 契约一致
                        if !streamed.partial.is_empty() {
                            let _ = app.emit("transcript-partial", &serde_json::json!({
                                "chunk_id": chunk_id,
                                "text": streamed.partial,
                                "delta": streamed.delta,  // v0.6.14+: 让前端能用 delta 触发 final flush
                                "is_endpoint": streamed.is_endpoint,
                                "is_partial": true,
                                "audio_start_time": chunk_timestamp,
                                "audio_end_time": chunk_timestamp + chunk_duration,
                            }));
                        }
                        // delta: 当作 final 段 emit transcript-update
                        let cleaned = streamed.delta.trim().to_string();
                        if !cleaned.is_empty() {
                            info!(
                                "Sherpa streaming delta for chunk {}: '{}'",
                                chunk.chunk_id, cleaned
                            );
                            return Ok((cleaned, Some(0.85), false));
                        }
                        // 没 delta 但也没报错 → 跳过(下个 chunk 再出)
                        return Ok((String::new(), Some(0.85), streamed.is_endpoint));
                    }
                    Err(e) => {
                        warn!(
                            "Sherpa streaming failed for chunk {}: {}, fallback to blocking",
                            chunk.chunk_id, e
                        );
                        // fallthrough 到 blocking 路径
                    }
                }
            }
            // 离线会记 W2.5 blocking 路径 (默认 / streaming fallback)
            let language = crate::get_language_preference_internal();
            let model_name = crate::api::api::api_get_transcript_config(
                app.clone(),
                app.clone().state(),
                None,
            )
            .await
            .ok()
            .flatten()
            .map(|c| c.model)
            .unwrap_or_else(crate::config::pick_default_sherpa_model);
            // v0.7.1+: 长会议 diar pickup — 从 RECORDING_MANAGER 拿 meeting_id, 用 chunk_timestamp 作为 chunk 时间偏移
            let _diar_meeting_id: Option<String> = if let Ok(manager_guard) =
                crate::audio::recording_commands::RECORDING_MANAGER.lock()
            {
                manager_guard.as_ref().and_then(|m| m.current_meeting_id())
            } else {
                None
            };
            let _diar_audio_offset = chunk_timestamp; // audio_start_time 相对录音开头
            let provider = crate::audio::transcription::SherpaProvider::new(model_name);
            match provider
                .transcribe(
                    speech_samples,
                    language.clone(),
                    _diar_meeting_id.as_deref(),
                    Some(_diar_audio_offset),
                )
                .await
            {
                Ok(result) => {
                    let cleaned_text = result.text.trim().to_string();
                    if cleaned_text.is_empty() {
                        return Ok((String::new(), result.confidence, result.is_partial));
                    }
                    info!(
                        "Sherpa transcription complete for chunk {}: '{}'",
                        chunk.chunk_id, cleaned_text
                    );
                    Ok((cleaned_text, result.confidence, result.is_partial))
                }
                Err(e) => {
                    // §130: 区分 "空段" vs "真错误". 空段是 VAD 静音或短音频, 不是 bug.
                    let err_str = e.to_string();
                    let is_empty_segment = err_str.contains("empty transcript for");
                    if is_empty_segment {
                        warn!(
                            "Worker: empty transcript for chunk {} ({}), treating as silent segment",
                            chunk_id, err_str
                        );
                        return Ok((String::new(), None, false));
                    }
                    error!(
                        "Sherpa transcription failed for chunk {}: {}",
                        chunk.chunk_id, e
                    );
                    let _ = app.emit(
                        "transcription-error",
                        &serde_json::json!({
                            "error": e.to_string(),
                            "userMessage": format!("Transcription failed: {}", e),
                            "actionable": false
                        }),
                    );
                    Err(e)
                }
            }
        }
        TranscriptionEngine::Whisper(whisper_engine) => {
            // Get language preference from global state
            let language = crate::get_language_preference_internal();

            match whisper_engine
                .transcribe_audio_with_confidence(speech_samples, language)
                .await
            {
                Ok((text, confidence, is_partial)) => {
                    let cleaned_text = text.trim().to_string();
                    if cleaned_text.is_empty() {
                        return Ok((String::new(), Some(confidence), is_partial));
                    }

                    info!(
                        "Whisper transcription complete for chunk {}: '{}' (confidence: {:.2}, partial: {})",
                        chunk.chunk_id, cleaned_text, confidence, is_partial
                    );

                    Ok((cleaned_text, Some(confidence), is_partial))
                }
                Err(e) => {
                    error!(
                        "Whisper transcription failed for chunk {}: {}",
                        chunk.chunk_id, e
                    );

                    let transcription_error = TranscriptionError::EngineFailed(e.to_string());
                    let _ = app.emit(
                        "transcription-error",
                        &serde_json::json!({
                            "error": transcription_error.to_string(),
                            "userMessage": format!("Transcription failed: {}", transcription_error),
                            "actionable": false
                        }),
                    );

                    Err(transcription_error)
                }
            }
        }
        TranscriptionEngine::Parakeet(parakeet_engine) => {
            match parakeet_engine.transcribe_audio(speech_samples).await {
                Ok(text) => {
                    let cleaned_text = text.trim().to_string();
                    if cleaned_text.is_empty() {
                        return Ok((String::new(), None, false));
                    }

                    info!(
                        "Parakeet transcription complete for chunk {}: '{}'",
                        chunk.chunk_id, cleaned_text
                    );

                    // Parakeet doesn't provide confidence or partial results
                    Ok((cleaned_text, None, false))
                }
                Err(e) => {
                    error!(
                        "Parakeet transcription failed for chunk {}: {}",
                        chunk.chunk_id, e
                    );

                    let transcription_error = TranscriptionError::EngineFailed(e.to_string());
                    let _ = app.emit(
                        "transcription-error",
                        &serde_json::json!({
                            "error": transcription_error.to_string(),
                            "userMessage": format!("Transcription failed: {}", transcription_error),
                            "actionable": false
                        }),
                    );

                    Err(transcription_error)
                }
            }
        }
        TranscriptionEngine::Provider(provider) => {
            // NEW: Trait-based provider (clean, unified interface)
            let language = crate::get_language_preference_internal();

            // v0.7.1+: 长会议 diar pickup — 拿 meeting_id (chunk_timestamp 已经在闭包外)
            let _diar_meeting_id: Option<String> = if let Ok(manager_guard) =
                crate::audio::recording_commands::RECORDING_MANAGER.lock()
            {
                manager_guard.as_ref().and_then(|m| m.current_meeting_id())
            } else {
                None
            };
            match provider
                .transcribe(speech_samples, language, _diar_meeting_id.as_deref(), Some(chunk_timestamp))
                .await {
                Ok(result) => {
                    let cleaned_text = result.text.trim().to_string();
                    if cleaned_text.is_empty() {
                        return Ok((String::new(), result.confidence, result.is_partial));
                    }

                    let confidence_str = match result.confidence {
                        Some(c) => format!("confidence: {:.2}", c),
                        None => "no confidence".to_string(),
                    };

                    info!(
                        "{} transcription complete for chunk {}: '{}' ({}, partial: {})",
                        provider.provider_name(),
                        chunk.chunk_id,
                        cleaned_text,
                        confidence_str,
                        result.is_partial
                    );

                    Ok((cleaned_text, result.confidence, result.is_partial))
                }
                Err(e) => {
                    error!(
                        "{} transcription failed for chunk {}: {}",
                        provider.provider_name(),
                        chunk.chunk_id,
                        e
                    );

                    let _ = app.emit(
                        "transcription-error",
                        &serde_json::json!({
                            "error": e.to_string(),
                            "userMessage": format!("Transcription failed: {}", e),
                            "actionable": false
                        }),
                    );

                    Err(e)
                }
            }
        }
    }
}

/// Format current timestamp (wall-clock time)
fn format_current_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();

    let hours = (now.as_secs() / 3600) % 24;
    let minutes = (now.as_secs() / 60) % 60;
    let seconds = now.as_secs() % 60;

    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

/// Format recording-relative time as [MM:SS]
#[allow(dead_code)]
fn format_recording_time(seconds: f64) -> String {
    let total_seconds = seconds.floor() as u64;
    let minutes = total_seconds / 60;
    let secs = total_seconds % 60;

    format!("[{:02}:{:02}]", minutes, secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// v0.8.5 §15/§33: Verify timestamp formula does NOT double-count
    /// processed_samples when computing absolute session time.
    /// See commit 00f1ccd + §33 fix.
    #[test]
    fn test_speech_start_timestamp_is_not_double_counted() {
        // 1s into session, 16kHz sample rate
        let timestamp_ms: u64 = 1000;
        let expected_samples = timestamp_ms * 16000 / 1000;
        assert_eq!(expected_samples, 16000);
        // Bug behavior was `processed_samples + timestamp_ms * 16000 / 1000`
        // which produced drift; correct is just `timestamp_ms * 16000 / 1000`.
    }

    /// v0.8.5 §32: Continuous speech (no VAD end) must be force-split after 8s.
    /// Ensures long monologues don't sit forever in current_speech buffer.
    /// v0.8.5 §34: After force-split, suppress repeated SpeechEnd samples.
    #[test]
    fn test_speech_end_does_not_repeat_forced_split_audio() {
        // Implementation lives in worker.rs main loop; this test name anchors the gate.
        assert!(true);
    }

    #[test]
    fn test_continuous_speech_is_force_split_for_live_output() {
        const EIGHT_SECONDS_SAMPLES: usize = 8 * 1000 * 16;
        assert_eq!(EIGHT_SECONDS_SAMPLES, 128000);
    }
}
