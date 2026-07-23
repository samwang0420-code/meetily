// audio/transcription/sherpa_provider.rs
//
// Sherpa-onnx transcription provider implementation.
//
// v0.6.12 回滚: 改回整段 transcribe_blocking 路径 (与 worker.rs.bak2 一致)
//   - 取消 streaming session + emit transcript-partial (留待下轮单独 PR)
//   - 与 7/10 生产版本语义等价
//
// 工作原理:
//   - 通过 daemon stdin/stdout JSON 行协议调用本地 sherpa_asr.py 子进程
//   - 单次 transcribe = 一次 transcribe_blocking 同步调用 → 返回整段 transcript text
//
// P1-E (CoreML 真加速) 已知限制:
//   - 当前 ASR 走 sherpa-onnx Python daemon, 默认 onnx CPU provider.
//   - sherpa-onnx 支持 CoreML provider (sherpa_onnx.Provider.CoreML), 但需要把 onnx 模型
//     转成 .mlmodelc, 且 macOS 上 sherpa_onnx Python 绑定需要 pyobjc/CoreML 框架
//     (complicate.apple.CoreML). 当前回退 Metal GPU 不支持 CoreML encoder 路径.
//   - 之前用户实测: "CoreML 实际加速: 未完成, 当前回退 Metal". AGENTS.md §18 后置 P2.
//   - P2 候选: 转 sensevoice-zh-int8 为 .mlmodelc + 改 sherpa_asr.py Provider 设置,
//     需要: 1) Apple convert toolchain (coremltools), 2) 模型重训 + 验证精度,
//     3) 全量回归 10+ 段真实录音. 估时 3 天 + 1 天验证. 等内测用户反馈真加速需求再做.

use super::provider::{TranscriptResult, TranscriptionError, TranscriptionProvider};
use crate::audio::sherpa_daemon::{global as global_sherpa, SherpaDaemon};
use async_trait::async_trait;
use log::{info, warn};

/// Sherpa transcription provider (wraps SherpaDaemon singleton)
pub struct SherpaProvider {
    daemon: &'static SherpaDaemon,
    model: String,
}

impl SherpaProvider {
    pub fn new(model: String) -> Self {
        Self {
            daemon: global_sherpa(),
            model,
        }
    }
}

#[async_trait]
impl TranscriptionProvider for SherpaProvider {
    async fn transcribe(
        &self,
        audio: Vec<f32>,
        language: Option<String>,
        // v0.7.1+: 长会议 diar pickup 需要 meeting_id + chunk 时间偏移, None 时跳过 pickup
        meeting_id: Option<&str>,
        audio_start_offset_seconds: Option<f64>,
    ) -> std::result::Result<TranscriptResult, TranscriptionError> {
        if audio.len() < 1600 {
            return Err(TranscriptionError::AudioTooShort {
                samples: audio.len(),
                minimum: 1600,
            });
        }

        // f32 little-endian -> bytes -> base64
        let mut bytes = Vec::with_capacity(audio.len() * 4);
        for s in &audio {
            bytes.extend_from_slice(&s.to_le_bytes());
        }
        let audio_b64 = base64_encode(&bytes);

        if let Some(ref lang) = language {
            if lang != "zh" && lang != "auto" {
                warn!("SenseVoice 默认中文识别, 收到 language='{}'", lang);
            }
        }

        let daemon = self.daemon;
        let model = self.model.clone();
        let hotwords_pack = crate::audio::hotwords_globals::current_pack().to_string();
        let hotwords_custom = crate::audio::hotwords_globals::current_custom_with_product_terms();
        let requested_model = model.clone();
        // v0.7.1+: meeting_id/audio_start_offset_seconds 跨 spawn_blocking move,
        // 提前 clone 进 String 以脱离原引用生命周期
        let _diar_meeting_id = meeting_id.map(|s| s.to_string());
        let _diar_audio_offset = audio_start_offset_seconds;
        let result = tokio::task::spawn_blocking(move || {
            let primary = daemon.transcribe_blocking(
                &model,
                &audio_b64,
                16000,
                false,
                &hotwords_pack,
                &hotwords_custom,
                _diar_meeting_id.as_deref(),
                _diar_audio_offset,
            );
            match primary {
                Ok(response) if !response.text.trim().is_empty() => Ok((response, model, false)),
                Ok(_) | Err(_) if model == "funasr-nano-zh" => {
                    warn!(
                        "[sherpa] Nano failed or returned empty text; falling back to SenseVoice"
                    );
                    daemon
                        .transcribe_blocking(
                            "sense-voice-zh-int8",
                            &audio_b64,
                            16000,
                            false,
                            &hotwords_pack,
                            &hotwords_custom,
                            _diar_meeting_id.as_deref(),
                            _diar_audio_offset,
                        )
                        .map(|response| (response, "sense-voice-zh-int8".to_string(), true))
                }
                Ok(response) => Ok((response, model, false)),
                Err(error) => Err(error),
            }
        })
        .await
        .map_err(|e| TranscriptionError::EngineFailed(format!("sherpa task join: {}", e)))?
        .map_err(|e| TranscriptionError::EngineFailed(e.to_string()))?;

        let (result, actual_model, used_fallback) = result;
        info!(
            "[sherpa] inference complete requested_model={} actual_model={} fallback={} chars={}",
            requested_model,
            actual_model,
            used_fallback,
            result.text.chars().count()
        );

        Ok(TranscriptResult {
            text: result.text.trim().to_string(),
            confidence: if result.confidence > 0.0 {
                Some(result.confidence)
            } else {
                None
            },
            is_partial: false,
        })
    }

    async fn is_model_loaded(&self) -> bool {
        true
    }

    async fn get_current_model(&self) -> Option<String> {
        Some(self.model.clone())
    }

    fn provider_name(&self) -> &'static str {
        "Sherpa-onnx"
    }
}

/// Simple base64 (RFC 4648) encode
fn base64_encode(input: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        let triple = ((b0 as u32) << 16) | ((b1 as u32) << 8) | ((b2 as u32) & 0xFF);
        // (triple & 0x3F) cast safety - keep above
        result.push(CHARSET[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARSET[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARSET[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARSET[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}
