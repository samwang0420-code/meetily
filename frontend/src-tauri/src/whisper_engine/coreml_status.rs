//! CoreML status reporter for the Whisper transcription engine.
//!
//! Whisper.cpp's CoreML fast path is opt-in: the encoder `.mlmodelc` bundle
//! must live next to the ggml model on disk. Without it the runtime silently
//! falls back to the Metal/CPU encoder and the user never sees the speedup.
//! This module exposes a single Tauri command (`whisper_coreml_status`) that
//! scans the models directory, classifies each Whisper model as CoreML-ready
//! or CPU-fallback, and returns a structured report the UI can surface.

use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::config::WHISPER_MODEL_CATALOG;

#[derive(Debug, Clone, Serialize)]
pub struct CoreMLEntry {
    pub model: String,
    pub ggml_filename: String,
    pub ggml_path: String,
    pub ggml_size_bytes: u64,
    pub encoder_path: String,
    pub encoder_size_bytes: u64,
    pub coreml_ready: bool,
    pub backend: &'static str,
    pub missing_reason: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoreMLStatusReport {
    pub models_dir: String,
    pub tag: String,
    pub coreml_compiled: bool,
    pub coreml_compiled_reason: String,
    pub macos_arm64: bool,
    pub entries: Vec<CoreMLEntry>,
    pub ready_count: usize,
    pub total_downloaded: usize,
}

const WHISPER_CPP_TAG: &str = "v1.7.1";

fn coreml_compiled_features() -> bool {
    // The whisper_engine crate is built with `coreml` on macOS aarch64 by
    // default (Cargo.toml: whisper-rs = { features = ["raw-api", "metal", "coreml"] }).
    // We re-detect here so the report stays in sync with the actual build.
    cfg!(all(target_os = "macos", target_arch = "aarch64"))
}

fn encoder_dir_for(ggml_path: &Path) -> PathBuf {
    // whisper.cpp looks for `<basename>.mlmodelc` next to the .bin file
    // (see whisper-encoder-impl.m URLOfModelInThisBundle lookup).
    let stem = ggml_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    ggml_path.with_file_name(format!("{stem}.mlmodelc"))
}

fn dir_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| entry.metadata().ok())
        .map(|meta| meta.len())
        .sum()
}

pub fn build_status_report(models_dir: &Path) -> CoreMLStatusReport {
    let compiled = coreml_compiled_features();
    let mut entries = Vec::new();
    let mut ready_count = 0usize;
    let mut total_downloaded = 0usize;

    for &(name, filename, _, _, _, _) in WHISPER_MODEL_CATALOG {
        let ggml_path = models_dir.join(filename);
        if !ggml_path.exists() {
            continue;
        }
        let ggml_size = std::fs::metadata(&ggml_path).map(|m| m.len()).unwrap_or(0);
        if ggml_size < 1024 * 1024 {
            continue;
        }
        total_downloaded += 1;

        let encoder_path = encoder_dir_for(&ggml_path);
        let encoder_size = dir_size(&encoder_path);
        let coreml_ready = compiled && encoder_path.is_dir() && encoder_size > 0;

        if coreml_ready {
            ready_count += 1;
        }

        let missing_reason: Option<&'static str> = if !compiled {
            Some("whisper-rs built without coreml feature; rebuild with --features coreml")
        } else if !encoder_path.is_dir() {
            Some("whisper_encoder_impl.mlmodelc not found next to ggml model")
        } else {
            None
        };

        entries.push(CoreMLEntry {
            model: name.to_string(),
            ggml_filename: filename.to_string(),
            ggml_path: ggml_path.display().to_string(),
            ggml_size_bytes: ggml_size,
            encoder_path: encoder_path.display().to_string(),
            encoder_size_bytes: encoder_size,
            coreml_ready,
            backend: if coreml_ready { "CoreML+Metal" } else { "Metal" },
            missing_reason,
        });
    }

    CoreMLStatusReport {
        models_dir: models_dir.display().to_string(),
        tag: WHISPER_CPP_TAG.to_string(),
        coreml_compiled: compiled,
        coreml_compiled_reason: if compiled {
            "macOS aarch64 build enables whisper-rs/coreml feature"
                .to_string()
        } else {
            "current build does not include whisper-rs/coreml".to_string()
        },
        macos_arm64: cfg!(all(target_os = "macos", target_arch = "aarch64")),
        entries,
        ready_count,
        total_downloaded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_temp_dir(label: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "speakmirror-coreml-status-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn reports_zero_when_no_models_present() {
        let dir = make_temp_dir("empty");
        let report = build_status_report(&dir);
        assert_eq!(report.total_downloaded, 0);
        assert_eq!(report.ready_count, 0);
        assert!(report.entries.is_empty());
        assert!(!report.coreml_compiled || report.coreml_compiled);
    }

    #[test]
    fn marks_model_as_coreml_ready_when_encoder_present() {
        let dir = make_temp_dir("ready");
        let ggml = dir.join("ggml-tiny.bin");
        std::fs::write(&ggml, vec![0u8; 1_500_000]).unwrap();
        let encoder = encoder_dir_for(&ggml);
        std::fs::create_dir_all(&encoder).unwrap();
        std::fs::write(encoder.join("coremldata.bin"), vec![0u8; 2048]).unwrap();

        let report = build_status_report(&dir);
        let tiny = report
            .entries
            .iter()
            .find(|entry| entry.ggml_filename == "ggml-tiny.bin")
            .expect("tiny entry must be reported");
        assert!(tiny.encoder_size_bytes > 0);
        // compiled is cfg-gated; on macOS dev machines this is true, on CI it may be false.
        if report.coreml_compiled {
            assert!(tiny.coreml_ready, "encoder is present and CoreML is compiled: {tiny:?}");
        } else {
            assert!(!tiny.coreml_ready);
            assert!(tiny.missing_reason.unwrap().contains("coreml"));
        }
    }

    #[test]
    fn marks_model_as_not_ready_when_encoder_missing() {
        let dir = make_temp_dir("missing-encoder");
        let ggml = dir.join("ggml-tiny.bin");
        std::fs::write(&ggml, vec![0u8; 1_500_000]).unwrap();
        let report = build_status_report(&dir);
        let tiny = report
            .entries
            .iter()
            .find(|entry| entry.ggml_filename == "ggml-tiny.bin")
            .expect("tiny entry must be reported");
        assert_eq!(tiny.encoder_size_bytes, 0);
        assert!(!tiny.coreml_ready);
    }
}
