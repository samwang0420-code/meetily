// audio/transcription/sherpa_provider.rs
//
// 离线会记 W2: sherpa-onnx 双 ASR provider 实现.
//   * sherpa_paraformer  - Paraformer-Large-ZH INT8 量化(默认/免费),中文 SOTA
//   * sherpa_funasr_nano - SenseVoice INT8 量化(Pro/¥299/年),同 k2-fsa 阿里达摩院
//                          系生态,多语种 + 情绪/语种检测
//
// model 路径: ~/Library/Application Support/cn.lixianhuiji.app/models/sherpa/

use super::provider::{TranscriptionError, TranscriptionProvider, TranscriptResult};
use async_trait::async_trait;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

// sherpa-rs 是 thewh1teagle/sherpa-rs 的 k2-fsa/sherpa-onnx Rust binding.
// offline 模式直接消费 16kHz mono f32,和 meetily audio 完全匹配
use sherpa_rs::paraformer::{ParaformerConfig, ParaformerRecognizer};
use sherpa_rs::sense_voice::{SenseVoiceConfig, SenseVoiceRecognizer};

/// 离线会记 默认 ASR 模型存放路径(与 parakeet/whisper 同 base)
fn default_models_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/wangwei".into());
    PathBuf::from(home)
        .join("Library/Application Support/cn.lixianhuiji.app/models/sherpa")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SherpaBackend {
    /// Paraformer-zh INT8 量化模型,默认免费
    Paraformer,
    /// SenseVoice INT8 量化模型,Pro 独占
    SenseVoice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SherpaProviderConfig {
    pub backend: SherpaBackend,
    pub models_dir: PathBuf,
    /// CPU 线程数(M1 8 核,免费版 4 避免抢 Ollama,Pro 可以用 6)
    pub num_threads: i32,
}

impl SherpaProviderConfig {
    pub fn default_for_free_user() -> Self {
        Self {
            backend: SherpaBackend::Paraformer,
            models_dir: default_models_dir(),
            num_threads: 4,
        }
    }

    pub fn default_for_pro_user() -> Self {
        Self {
            backend: SherpaBackend::SenseVoice,
            models_dir: default_models_dir(),
            num_threads: 6,
        }
    }

    fn model_paths(&self) -> Result<(PathBuf, PathBuf), String> {
        let (model_dir, model_name, tokens_name) = match self.backend {
            SherpaBackend::Paraformer => ("paraformer-zh-int8", "model.int8.onnx", "tokens.txt"),
            SherpaBackend::SenseVoice => ("sense-voice-zh-int8", "model.int8.onnx", "tokens.txt"),
        };

        let model_dir = self.models_dir.join(model_dir);
        let model = model_dir.join(model_name);
        let tokens = model_dir.join(tokens_name);

        if !model.exists() {
            return Err(format!(
                "模型文件不存在: {}\n请运行: bash outputs/scripts/install-sherpa-asr.sh",
                model.display()
            ));
        }
        if !tokens.exists() {
            return Err(format!("tokens.txt 不存在: {}", tokens.display()));
        }
        Ok((model, tokens))
    }
}

impl Default for SherpaProviderConfig {
    fn default() -> Self {
        Self::default_for_free_user()
    }
}

/// 双模式 Provider:同一个 wrapper 根据 config.backend 切换 backend.
/// 内部用 mutex 保护 recognizer,因为 ParaformerRecognizer/SenseVoiceRecognizer
/// 都不是 Sync 的(transcribe() 需要 &mut self)。
pub struct SherpaProvider {
    config: SherpaProviderConfig,
    state: Mutex<SherpaState>,
}

enum SherpaState {
    Uninitialized,
    Paraformer(ParaformerRecognizer),
    SenseVoice(SenseVoiceRecognizer),
}

impl SherpaProvider {
    pub fn new(config: SherpaProviderConfig) -> std::result::Result<Self, TranscriptionError> {
        Ok(Self {
            config,
            state: Mutex::new(SherpaState::Uninitialized),
        })
    }

    fn ensure_loaded(&self) -> std::result::Result<(), TranscriptionError> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| TranscriptionError::EngineFailed(format!("mutex poisoned: {e}")))?;

        match &*state {
            SherpaState::Uninitialized => {}
            SherpaState::Paraformer(_) if self.config.backend == SherpaBackend::Paraformer => {
                return Ok(());
            }
            SherpaState::SenseVoice(_) if self.config.backend == SherpaBackend::SenseVoice => {
                return Ok(());
            }
            _ => {
                warn!("切换 backend,丢弃旧 recognizer");
                *state = SherpaState::Uninitialized;
            }
        }

        let (model_path, tokens_path) = self
            .config
            .model_paths()
            .map_err(|e| TranscriptionError::EngineFailed(format!("sherpa 模型缺失: {e}")))?;

        let new_state = match self.config.backend {
            SherpaBackend::Paraformer => {
                info!("🐉 加载 Paraformer-zh INT8: {}", model_path.display());
                let cfg = ParaformerConfig {
                    model: model_path.to_string_lossy().into_owned(),
                    tokens: tokens_path.to_string_lossy().into_owned(),
                    provider: Some("cpu".into()),
                    num_threads: Some(self.config.num_threads),
                    debug: false,
                };
                let recognizer = ParaformerRecognizer::new(cfg)
                    .map_err(|e| TranscriptionError::EngineFailed(format!("Paraformer 初始化失败: {e}")))?;
                SherpaState::Paraformer(recognizer)
            }
            SherpaBackend::SenseVoice => {
                info!("🔮 加载 SenseVoice INT8: {}", model_path.display());
                let cfg = SenseVoiceConfig {
                    model: model_path.to_string_lossy().into_owned(),
                    tokens: tokens_path.to_string_lossy().into_owned(),
                    language: "zh".into(),
                    use_itn: true,
                    provider: Some("cpu".into()),
                    num_threads: Some(self.config.num_threads),
                    debug: false,
                };
                let recognizer = SenseVoiceRecognizer::new(cfg).map_err(|e| {
                    TranscriptionError::EngineFailed(format!("SenseVoice 初始化失败: {e}"))
                })?;
                SherpaState::SenseVoice(recognizer)
            }
        };
        *state = new_state;
        Ok(())
    }
}

#[async_trait]
impl TranscriptionProvider for SherpaProvider {
    async fn transcribe(
        &self,
        audio: Vec<f32>,
        _language: Option<String>,
    ) -> std::result::Result<TranscriptResult, TranscriptionError> {
        if audio.len() < 1600 {
            return Err(TranscriptionError::AudioTooShort {
                samples: audio.len(),
                minimum: 1600,
            });
        }

        self.ensure_loaded()?;

        let mut state = self
            .state
            .lock()
            .map_err(|e| TranscriptionError::EngineFailed(format!("mutex poisoned: {e}")))?;

        let text = match &mut *state {
            SherpaState::Uninitialized => {
                return Err(TranscriptionError::EngineFailed(
                    "sherpa recognizer 未初始化".into(),
                ));
            }
            SherpaState::Paraformer(recognizer) => {
                let result = recognizer.transcribe(16000, &audio);
                result.text.trim().to_string()
            }
            SherpaState::SenseVoice(recognizer) => {
                let result = recognizer.transcribe(16000, &audio);
                result.text.trim().to_string()
            }
        };

        Ok(TranscriptResult {
            text,
            confidence: None,
            is_partial: false,
        })
    }

    async fn is_model_loaded(&self) -> bool {
        let state = match self.state.lock() {
            Ok(s) => s,
            Err(_) => return false,
        };
        !matches!(*state, SherpaState::Uninitialized)
    }

    async fn get_current_model(&self) -> Option<String> {
        let dir = match self.config.backend {
            SherpaBackend::Paraformer => "paraformer-zh-int8",
            SherpaBackend::SenseVoice => "sense-voice-zh-int8",
        };
        Some(format!("{}/model.int8.onnx", dir))
    }

    fn provider_name(&self) -> &'static str {
        match self.config.backend {
            SherpaBackend::Paraformer => "sherpa/Paraformer-zh-INT8",
            SherpaBackend::SenseVoice => "sherpa/SenseVoice-INT8",
        }
    }
}

/// 检查 sherpa-onnx 静态库是否安装到 ~/Documents/离线会记/sherpa-libs/
pub fn validate_sherpa_lib_installed() -> Result<(), String> {
    let lib_dir = std::env::var("SHERPA_LIB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/wangwei".into());
            PathBuf::from(home).join("Documents/离线会记/sherpa-libs")
        });

    let lib = lib_dir.join("lib/libsherpa-onnx.a");
    if !lib.exists() {
        return Err(format!(
            "sherpa-onnx 静态库未找到: {}\n请先跑: bash outputs/scripts/install-sherpa-onnx.sh",
            lib.display()
        ));
    }
    Ok(())
}
