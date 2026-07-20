/// Application configuration constants
///
/// Centralized definitions for default models and settings.
/// Used across database initialization, import, and retranscription.

use std::path::PathBuf;

/// Default Whisper model for transcription when no preference is configured.
/// This is the recommended balance of accuracy and speed.
pub const DEFAULT_WHISPER_MODEL: &str = "large-v3-turbo";

/// Default Parakeet model for transcription when no preference is configured.
/// This is the quantized version optimized for speed.
pub const DEFAULT_PARAKEET_MODEL: &str = "parakeet-tdt-0.6b-v3-int8";

/// Whisper model catalog with metadata for all supported models.
/// Used by both WhisperEngine::discover_models() and discover_models_standalone().
///
/// Format: (name, filename, size_mb, accuracy, speed, description)
pub const WHISPER_MODEL_CATALOG: &[(&str, &str, u32, &str, &str, &str)] = &[
    // Standard f16 models (full precision)
    ("tiny", "ggml-tiny.bin", 74, "Decent", "Very Fast", "Fastest processing, good for real-time use"),
    ("base", "ggml-base.bin", 142, "Good", "Fast", "Good balance of speed and accuracy"),
    ("small", "ggml-small.bin", 466, "Good", "Medium", "Better accuracy, moderate speed"),
    ("medium", "ggml-medium.bin", 1463, "High", "Slow", "High accuracy for professional use"),
    ("large-v3-turbo", "ggml-large-v3-turbo.bin", 1549, "High", "Medium", "Best accuracy with improved speed"),
    ("large-v3", "ggml-large-v3.bin", 2951, "High", "Slow", "Most Accurate, latest large model"),

    // Q5_1 quantized models (balanced speed/accuracy, slightly better quality than Q5_0)
    ("tiny-q5_1", "ggml-tiny-q5_1.bin", 31, "Decent", "Very Fast", "Quantized tiny model, ~50% faster processing"),
    ("base-q5_1", "ggml-base-q5_1.bin", 57, "Good", "Fast", "Quantized base model, good speed/accuracy balance"),
    ("small-q5_1", "ggml-small-q5_1.bin", 181, "Good", "Fast", "Quantized small model, faster than f16 version"),

    // Q5_0 quantized models (balanced speed/accuracy)
    ("medium-q5_0", "ggml-medium-q5_0.bin", 514, "High", "Medium", "Quantized medium model, professional quality"),
    ("large-v3-turbo-q5_0", "ggml-large-v3-turbo-q5_0.bin", 547, "High", "Medium", "Quantized large model, best balance"),
    ("large-v3-q5_0", "ggml-large-v3-q5_0.bin", 1031, "High", "Slow", "Quantized large model, high accuracy"),
];

/// 当前固定法律录音 A/B: SenseVoice CER 2.17%, FunASR-Nano CER 2.90%.
/// Nano 尚未通过 10 段完整基准和性能准入，因此默认保持 SenseVoice。
pub const DEFAULT_SHERPA_MODEL: &str = "sense-voice-zh-int8";

/// v0.7.0+: Sherpa-onnx 模型优先级 (用于 fallback).
pub const SHERPA_MODEL_FALLBACK_ORDER: &[&str] = &[
    "sense-voice-zh-int8",
    "paraformer-zh",
    "funasr-nano-zh",
];

/// v0.7.0+: 运行时挑选当前最佳的 Sherpa-onnx 默认模型.
/// 扫描 ~/Library/Application Support/cn.lixianhuiji.app/models/sherpa/,
/// 按 SHERPA_MODEL_FALLBACK_ORDER 优先级取第一个已下载的.
/// 没有任何下载时回退到 SenseVoice；Nano 仅在用户显式选择时使用。
pub fn pick_default_sherpa_model() -> String {
    let base = match dirs_sherpa_models_dir() {
        Some(p) => p,
        None => return DEFAULT_SHERPA_MODEL.to_string(),
    };
    for tag in SHERPA_MODEL_FALLBACK_ORDER {
        let sub = match tag_as_dir_name(tag) {
            Some(n) => base.join(n),
            None => continue,
        };
        if is_model_downloaded(&sub) {
            return tag.to_string();
        }
    }
    DEFAULT_SHERPA_MODEL.to_string()
}

fn dirs_sherpa_models_dir() -> Option<PathBuf> {
    // macOS: ~/Library/Application Support/cn.lixianhuiji.app/models/sherpa/
    // Linux: $XDG_DATA_HOME/cn.lixianhuiji.app/models/sherpa/
    // Windows: %APPDATA%/cn.lixianhuiji.app/models/sherpa/
    if let Some(mut p) = dirs_root_app_data() {
        p.push("models");
        p.push("sherpa");
        return Some(p);
    }
    None
}

fn dirs_root_app_data() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| {
            let mut p = PathBuf::from(h);
            p.push("Library/Application Support/cn.lixianhuiji.app");
            p
        })
    } else if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(|v| PathBuf::from(v).join("cn.lixianhuiji.app"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(|v| PathBuf::from(v).join("cn.lixianhuiji.app"))
            .or_else(|| {
                std::env::var_os("HOME").map(|h| {
                    PathBuf::from(h).join(".local/share/cn.lixianhuiji.app")
                })
            })
    }
}

fn tag_as_dir_name(tag: &str) -> Option<&'static str> {
    match tag {
        "funasr-nano-zh" => Some("funasr-nano-int8"),
        "paraformer-zh" => Some("paraformer-zh-int8"),
        "sense-voice-zh-int8" => Some("sense-voice-zh-int8"),
        _ => None,
    }
}

fn is_model_downloaded(dir: &std::path::Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    // Heuristic: 至少有一个 .onnx 文件 + 至少 30MB 大小
    let mut has_onnx = false;
    let mut total = 0u64;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for ent in rd.flatten() {
            let p = ent.path();
            if p.extension().and_then(|s| s.to_str()) == Some("onnx") {
                has_onnx = true;
            }
            if let Ok(meta) = std::fs::metadata(&p) {
                total += meta.len();
            }
        }
    }
    has_onnx && total > 30 * 1024 * 1024
}
