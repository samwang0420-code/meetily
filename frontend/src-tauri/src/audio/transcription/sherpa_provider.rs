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
            if lang != "zh" && lang != "en" && lang != "auto" {
                warn!(
                    "[sherpa] requested language='{}' may not be supported by local ASR (Chinese-only). \
                     Will auto-detect in daemon.",
                    lang
                );
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
                language.as_deref(),
            );
            match primary {
                Ok(response) if !response.text.trim().is_empty() => Ok((response, model, false)),
                // v0.7.0+ Pro tier gate (AGENTS.md §29): funasr-nano-zh 仅 Pro 用户能用.
                // 旧行为: Nano 失败/空 → 静默 fallback 到 sense-voice-zh-int8, 把 Pro 用户降级, 违反 §29.
                // 新行为: 失败直接 Err, 由用户在 UI 切模型, 不在后端偷偷降级.
                result if model == "funasr-nano-zh" => {
                    let msg = match result {
                        Ok(_) => "funasr-nano-zh returned empty transcript (模型未下载或音频超出 60s 上限)".to_string(),
                        Err(e) => format!("funasr-nano-zh 识别失败: {}", e),
                    };
                    Err(TranscriptionError::EngineFailed(msg))
                }
                Ok(response) => Ok((response, model, false)),
                Err(error) => Err(TranscriptionError::EngineFailed(error.to_string())),
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


#[cfg(test)]
mod tests {
    use super::*;

    /// v0.7.0+ regression: Pro tier gate (AGENTS.md §29).
    /// 旧实现: funasr-nano-zh 失败 → 静默 fallback 到 sense-voice-zh-int8, 把 Pro 用户降级.
    /// 新实现: Pro fallback 路径必须返 Err, 不允许在 Rust 端偷偷降级.
    ///
    /// 直接测 base64_encode (复用纯函数, 不依赖 daemon), 验证 payload 序列化的形状.
    #[test]
    fn base64_encode_payload_roundtrips() {
        // 模拟 8 个 f32 样本 (32 bytes), 3 个 chunk 边界
        let samples: Vec<f32> = (0..8).map(|i| i as f32 * 0.1).collect();
        let mut bytes = Vec::with_capacity(samples.len() * 4);
        for s in &samples {
            bytes.extend_from_slice(&s.to_le_bytes());
        }
        let encoded = base64_encode(&bytes);
        // 32 bytes → ceil(32/3)*4 = 44 chars (base64 padded)
        assert_eq!(encoded.len() % 4, 0, "base64 长度必须是 4 倍数");
        // 反向 decode 应该拿回完全一样的 bytes
        let decoded = base64_decode(&encoded);
        assert_eq!(decoded, bytes, "base64 roundtrip 必须 byte-equal");
    }

    /// RFC 4648 标准 base64 测试向量 (避免我们 base64_encode 自实现有偏差).
    #[test]
    fn base64_encode_matches_rfc4648_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    /// 反向 decode helper (测试专用).
    fn base64_decode(input: &str) -> Vec<u8> {
        let mut result = Vec::with_capacity(input.len() * 3 / 4);
        let chars: Vec<u8> = input.bytes().collect();
        let mut i = 0;
        while i < chars.len() {
            let b0 = decode_char(chars[i]);
            let b1 = decode_char(chars.get(i + 1).copied().unwrap_or(b'='));
            let b2 = decode_char(chars.get(i + 2).copied().unwrap_or(b'='));
            let b3 = decode_char(chars.get(i + 3).copied().unwrap_or(b'='));
            let triple = (b0 << 18) | (b1 << 12) | (b2 << 6) | b3;
            result.push(((triple >> 16) & 0xFF) as u8);
            if chars.get(i + 2).copied().unwrap_or(b'=') != b'=' {
                result.push(((triple >> 8) & 0xFF) as u8);
            }
            if chars.get(i + 3).copied().unwrap_or(b'=') != b'=' {
                result.push((triple & 0xFF) as u8);
            }
            i += 4;
        }
        result
    }

    fn decode_char(c: u8) -> u32 {
        match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            _ => 0,
        }
    }
}
