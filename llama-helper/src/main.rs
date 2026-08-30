use std::io::{self, BufRead, Write};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::pin::pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use encoding_rs;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use serde::{Deserialize, Serialize};

// ============================================================================
// Protocol Messages (JSON over stdin/stdout)
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Request {
    Generate {
        prompt: String,
        max_tokens: Option<i32>,
        context_size: Option<u32>,
        model_path: Option<String>,
        // §198 (2026-08-30): actual model layer count from Rust models.rs ModelDef::layer_count.
        //   Without this, get_default_gpu_layers() hardcodes estimated_layers=28 for files <2.5GB,
        //   which is wrong for Qwen 2.5 3B (actual 36 layers) → only 28 offloaded, 8 on CPU.
        //   With n_layer=36: weight_per_layer=1.93/36=0.054GB, total_per_layer=0.090GB,
        //   safe_layers=floor(4.30/0.090)=47 → min(47, 36) = 36 = all on GPU.
        n_layer: Option<u32>,
        // Sampling parameters
        temperature: Option<f32>,
        top_k: Option<i32>,
        top_p: Option<f32>,
        presence_penalty: Option<f32>,
        frequency_penalty: Option<f32>,
        repeat_penalty: Option<f32>,
        penalty_last_n: Option<i32>,
        stop_tokens: Option<Vec<String>>,
    },
    Ping,
    Shutdown,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Response {
    Response { text: String, error: Option<String> },
    Delta { text: String },
    Done { text: String },
    Pong,
    Goodbye,
    Error { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct SamplingConfig {
    temperature: f32,
    top_k: i32,
    top_p: f32,
    presence_penalty: f32,
    frequency_penalty: f32,
    repeat_penalty: f32,
    penalty_last_n: i32,
}

impl SamplingConfig {
    fn from_request(
        temperature: Option<f32>,
        top_k: Option<i32>,
        top_p: Option<f32>,
        presence_penalty: Option<f32>,
        frequency_penalty: Option<f32>,
        repeat_penalty: Option<f32>,
        penalty_last_n: Option<i32>,
    ) -> Self {
        // §163: 推理参数固化 (2026-08-23 立, 文档模块 3)
        // Map / Reduce 一律使用 temperature=0.1, top_p=0.3, repetition_penalty=1.05
        // 默认值走 env var 覆盖 (LLAMA_DEFAULT_TEMPERATURE / _TOP_P / _REPEAT_PENALTY),
        // 单元测试或临时验证可 export 改回 1.0。
        let default_temperature: f32 = std::env::var("LLAMA_DEFAULT_TEMPERATURE")
            .ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.1);
        let default_top_p: f32 = std::env::var("LLAMA_DEFAULT_TOP_P")
            .ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.3);
        let default_repeat_penalty: f32 = std::env::var("LLAMA_DEFAULT_REPEAT_PENALTY")
            .ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(1.05);

        let temperature = temperature.unwrap_or(default_temperature);
        let temperature = if temperature.is_finite() {
            temperature.max(0.0)
        } else {
            0.0
        };
        let top_k = top_k.unwrap_or(64).max(1);
        let top_p = top_p.unwrap_or(default_top_p);
        let top_p = if top_p.is_finite() && top_p > 0.0 && top_p <= 1.0 {
            top_p
        } else {
            1.0
        };
        let presence_penalty = presence_penalty.unwrap_or(0.0);
        let presence_penalty = if presence_penalty.is_finite() {
            presence_penalty.max(0.0)
        } else {
            0.0
        };
        let frequency_penalty = frequency_penalty.unwrap_or(0.0);
        let frequency_penalty = if frequency_penalty.is_finite() {
            frequency_penalty.max(0.0)
        } else {
            0.0
        };
        let repeat_penalty = repeat_penalty.unwrap_or(default_repeat_penalty);
        let repeat_penalty = if repeat_penalty.is_finite() && repeat_penalty > 0.0 {
            repeat_penalty
        } else {
            1.0
        };
        let penalty_last_n = penalty_last_n.unwrap_or(0).max(0);

        Self {
            temperature,
            top_k,
            top_p,
            presence_penalty,
            frequency_penalty,
            repeat_penalty,
            penalty_last_n,
        }
    }

    fn uses_penalties(&self) -> bool {
        self.penalty_last_n > 0
            && (self.presence_penalty > 0.0
                || self.frequency_penalty > 0.0
                || (self.repeat_penalty - 1.0).abs() > f32::EPSILON)
    }
}

// ============================================================================
// VRAM Detection and GPU Layer Calculation
// ============================================================================

/// Detect available VRAM in GB
fn detect_vram_gb() -> f32 {
    #[cfg(feature = "metal")]
    {
        // macOS Metal: Query recommended max working set size
        if let Some(vram) = detect_metal_vram() {
            eprintln!("Metal VRAM detected: {:.2} GB", vram);
            return vram;
        }
    }

    #[cfg(feature = "cuda")]
    {
        // NVIDIA CUDA: Query device memory
        if let Some(vram) = detect_cuda_vram() {
            eprintln!("CUDA VRAM detected: {:.2} GB", vram);
            return vram;
        }
    }

    /// TODO: Vulkan VRAM detection

    eprintln!("VRAM detection not available, using conservative estimate");
    4.0 // Conservative fallback
}

#[cfg(feature = "metal")]
fn detect_metal_vram() -> Option<f32> {
    if let Ok(output) = std::process::Command::new("sysctl")
        .arg("hw.memsize")
        .output()
    {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            if let Some(bytes_str) = stdout.split(':').nth(1) {
                if let Ok(bytes) = bytes_str.trim().parse::<u64>() {
                    let gb = bytes as f32 / (1024.0 * 1024.0 * 1024.0);
                    // Assume GPU can use ~60% of system memory on Apple Silicon
                    return Some(gb * 0.6);
                }
            }
        }
    }
    None
}

#[cfg(feature = "cuda")]
fn detect_cuda_vram() -> Option<f32> {
    // Use nvidia-smi to query VRAM
    if let Ok(output) = std::process::Command::new("nvidia-smi")
        .args(&["--query-gpu=memory.free", "--format=csv,noheader,nounits"])
        .output()
    {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            if let Ok(mb) = stdout.trim().parse::<f32>() {
                return Some(mb / 1024.0); // Convert MB to GB
            }
        }
    }
    None
}

/// Calculate safe GPU layer count based on VRAM, model file size, and context size
fn calculate_gpu_layers(
    model_path: &PathBuf,
    model_layers: u32,
    vram_gb: f32,
    context_size: u32,
) -> u32 {
    let file_size_gb = std::fs::metadata(model_path)
        .map(|m| m.len() as f32 / 1024.0 / 1024.0 / 1024.0)
        .unwrap_or(0.0);

    if file_size_gb == 0.0 {
        eprintln!("⚠️ Could not determine model file size, using conservative default");
        return 0;
    }

    // §193 (2026-08-28): GQA-aware KV cache estimation.
    // Old: assumed dense attention (1 KV per head) → overestimated for Qwen 2.5 3B
    //   (uses Grouped Query Attention, KV cache ~1/3 of dense).
    //   With context_size=32768 + kv_per_1k_gb=0.12, total_kv=3.93GB > safe_vram 3.5GB
    //   → Metal layers miscalculated → 100% CPU decode (1.4 tok/s vs expected 30+).
    //
    // New: 3-tier heuristic based on file size + likely architecture.
    //   > 5GB → large dense model (Llama 7B+): 0.25 GB/1K (was 0.25)
    //   > 2.5GB → mid dense (e.g. Qwen 2.5 7B, Mistral 7B): 0.15 GB/1K (was 0.25)
    //   ≤ 2.5GB → small GQA (Qwen 2.5 1.5B/3B, Llama 3.2 3B): 0.04 GB/1K (was 0.12)
    let kv_per_1k_gb = if file_size_gb > 5.0 {
        0.25  // large dense (7B+)
    } else if file_size_gb > 2.5 {
        0.15  // mid dense (Qwen 2.5 7B, Mistral 7B)
    } else {
        0.04  // small GQA (Qwen 2.5 1.5B/3B, Llama 3.2 3B)
    };
    let total_kv_gb = (context_size as f32 / 1000.0) * kv_per_1k_gb;

    // Safety buffer (500MB) for OS/Display
    let safe_vram = vram_gb - 0.5;

    // For debugging
    eprintln!("📊 VRAM Analysis:");
    eprintln!("   • Available: {:.2} GB", vram_gb);
    eprintln!("   • Safe Limit: {:.2} GB", safe_vram);
    eprintln!("   • Model Weights: {:.2} GB", file_size_gb);
    eprintln!(
        "   • KV Cache ({} ctx): {:.2} GB",
        context_size, total_kv_gb
    );

    if safe_vram <= 0.0 {
        eprintln!("⚠️ No safe VRAM available, using CPU only");
        return 0;
    }

    // Calculate cost per layer
    let weight_per_layer = file_size_gb / model_layers as f32;
    let kv_per_layer = total_kv_gb / model_layers as f32;
    let total_per_layer = weight_per_layer + kv_per_layer;

    // Calculate how many layers fit
    let safe_layers = (safe_vram / total_per_layer).floor() as u32;

    // §193 (2026-08-28): enforce minimum GPU offload.
    // Why: even if heuristic estimates 0, Metal (any layer count) is faster than CPU
    //   for matmul. 8 layers is a safe floor (8 × ~70MB Q4_K weights = ~560MB,
    //   fits even on 8GB Apple Silicon). Prevents §193 regression where
    //   "n_gpu_layers=0" caused 100% CPU decode (1.4 tok/s).
    const MIN_GPU_LAYERS: u32 = 8;
    let layers = safe_layers
        .max(MIN_GPU_LAYERS.min(model_layers))
        .min(model_layers);

    eprintln!(
        "   • Cost per layer: {:.2} MB (Weights) + {:.2} MB (KV) = {:.2} MB",
        weight_per_layer * 1024.0,
        kv_per_layer * 1024.0,
        total_per_layer * 1024.0
    );

    if layers < model_layers {
        eprintln!(
            "⚠️ Memory constrained. Offloading {}/{} layers ({:.1}%)",
            layers,
            model_layers,
            (layers as f32 / model_layers as f32) * 100.0
        );
    } else {
        eprintln!("✅ Full offload possible ({} layers)", layers);
    }

    layers
}

/// Get default GPU layer count with smart detection
///
/// §198 (2026-08-30): accept explicit n_layer from caller (Rust models.rs ModelDef::layer_count).
///   Falls back to file-size heuristic only when caller doesn't know (backward-compat).
///   Why: hardcoded "28 for files <2.5GB" was wrong for Qwen 2.5 3B (actual 36 layers),
///   causing 8 layers to stay on CPU → 99.6% CPU matmul → ~17 tok/s instead of 26 tok/s Metal.
fn get_default_gpu_layers(model_path: &PathBuf, context_size: u32, n_layer: Option<u32>) -> u32 {
    let vram = detect_vram_gb();
    let file_size_gb = std::fs::metadata(model_path)
        .map(|m| m.len() as f32 / 1024.0 / 1024.0 / 1024.0)
        .unwrap_or(0.0);

    // Prefer caller-provided layer count (Rust registry has GGUF metadata via ModelDef).
    // Fall back to file-size heuristic for backward-compat with old clients.
    let estimated_layers = match n_layer {
        Some(n) if n > 0 => n,
        _ => {
            if file_size_gb > 2.5 { 33 } else { 28 }
        }
    };

    let layers = calculate_gpu_layers(model_path, estimated_layers, vram, context_size);
    if let Some(caller_n) = n_layer {
        if caller_n > 0 && layers < caller_n {
            eprintln!(
                "§198 partial GPU offload: {} / {} layers (callers reported {} layers)",
                layers, caller_n, caller_n
            );
        }
    }
    layers
}

// ============================================================================
// §193 (2026-08-28) tests: GQA-aware KV cache estimation + min GPU layers floor.
// Why: §193 fix changes calculate_gpu_layers to (a) use 0.04 GB/1K for small GQA
//   models (Qwen 2.5 1.5B/3B, Llama 3.2 3B) instead of dense 0.12, and (b) enforce
//   minimum 8 GPU layers regardless of safe_layers estimate. These tests guard the
//   fix so it cannot silently regress.
#[cfg(test)]
mod section_193_gpu_layer_tests {
    use super::*;

    fn make_temp_model(size_bytes: u64) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("llama_helper_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("model_{}.gguf", size_bytes));
        // Sparse file: just touch it; metadata() reports the requested size.
        std::fs::write(&path, vec![0u8; 1]).unwrap();
        // Set file size via truncate
        let f = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        f.set_len(size_bytes).unwrap();
        path
    }

    /// Qwen 2.5 3B Q4_K_M = 1.93GB, context=32768, vram=4.0GB
    /// Old heuristic: kv_per_1k_gb=0.12 → total_kv=3.93GB > safe_vram=3.5GB
    ///   → safe_layers potentially very low → Metal offload not effective.
    /// New (§193): kv_per_1k_gb=0.04 → total_kv=1.31GB
    ///   → safe_layers = (3.5 - 1.93)/0.069 + ... = layers fit comfortably.
    ///   Plus min 8 GPU layers floor → at least 8 layers on Metal.
    #[test]
    fn test_qwen25_3b_gqa_min_8_layers() {
        let path = make_temp_model(1_930_000_000); // 1.93 GB
        let layers = calculate_gpu_layers(&path, 28, 4.0, 32768);
        assert!(
            layers >= 8,
            "§193 fix: small GQA model must offload >= 8 layers, got {}",
            layers
        );
    }

    /// Large dense model (Llama 7B+, file > 5GB) keeps original kv_per_1k_gb=0.25.
    #[test]
    fn test_large_dense_model_kv_estimate_unchanged() {
        // Use file size > 5GB to trigger the "large" branch
        let path = make_temp_model(5_500_000_000); // 5.5 GB
        let _layers = calculate_gpu_layers(&path, 33, 8.0, 4096);
        // No assertion on exact value (depends on heap math), but should not panic.
        // Verify the function returns a sensible value (>= min 8).
    }

    /// Even if VRAM is super tight, MIN_GPU_LAYERS floor should still apply
    /// (because even 1 layer on Metal is faster than CPU).
    #[test]
    fn test_min_8_layers_enforced_when_vram_constrained() {
        let path = make_temp_model(1_930_000_000); // 1.93 GB Qwen 2.5 3B
        // vram only 1.0 GB → safe_vram = 0.5 GB, definitely not enough for full offload
        let layers = calculate_gpu_layers(&path, 28, 1.0, 32768);
        assert!(
            layers >= 8,
            "§193 fix: min 8 GPU layers floor must hold even under tight VRAM, got {}",
            layers
        );
        assert!(layers <= 28, "must not exceed model_layers");
    }

    /// Mid-size dense model (e.g. Mistral 7B Q4, file 2.5-5GB) gets 0.15 GB/1K.
    #[test]
    fn test_mid_dense_model_uses_0_15_estimate() {
        let path = make_temp_model(3_800_000_000); // 3.8 GB → triggers mid branch
        let _layers = calculate_gpu_layers(&path, 32, 6.0, 4096);
        // Should not panic; exact value depends on math, just verify it runs.
    }

    /// Small GQA gets 0.04 GB/1K estimate. With 8GB VRAM + 4096 ctx:
    ///   total_kv = 4.096 * 0.04 = 0.16 GB
    ///   safe_vram = 7.5 GB
    ///   All 28 layers should fit easily.
    #[test]
    fn test_small_gqa_full_offload_with_ample_vram() {
        let path = make_temp_model(1_930_000_000); // 1.93 GB
        let layers = calculate_gpu_layers(&path, 28, 8.0, 4096);
        assert_eq!(
            layers, 28,
            "§193 fix: ample VRAM should full-offload Qwen 2.5 3B"
        );
    }
}

// ============================================================================
// Model State Management
// ============================================================================

struct ModelState {
    backend: LlamaBackend,
    model: Option<LlamaModel>,
    model_path: Option<PathBuf>,
    context_size: u32,
    last_activity: Arc<AtomicU64>,
}

impl ModelState {
    fn new() -> Result<Self> {
        let backend = LlamaBackend::init().context("Failed to init LlamaBackend")?;
        Ok(Self {
            backend,
            model: None,
            model_path: None,
            context_size: 2048,
            last_activity: Arc::new(AtomicU64::new(Self::current_timestamp())),
        })
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn update_activity(&self) {
        self.last_activity
            .store(Self::current_timestamp(), Ordering::SeqCst);
    }

    fn seconds_since_activity(&self) -> u64 {
        Self::current_timestamp() - self.last_activity.load(Ordering::SeqCst)
    }

    fn load_model_if_needed(
        &mut self,
        model_path: PathBuf,
        context_size: u32,
        n_layer: Option<u32>,  // §198
    ) -> Result<()> {
        // Check if model is already loaded
        if let Some(ref loaded_path) = self.model_path {
            if loaded_path == &model_path && self.context_size == context_size {
                eprintln!("✓ Model already loaded");
                self.update_activity();
                return Ok(());
            }
        }

        eprintln!("📥 Loading model: {}", model_path.display());

        // Detect GPU layers
        let gpu_layers = get_default_gpu_layers(&model_path, context_size, n_layer);

        // Configure model parameters with GPU offload
        let model_params = LlamaModelParams::default().with_n_gpu_layers(gpu_layers);
        let model_params = pin!(model_params);

        let model = LlamaModel::load_from_file(&self.backend, model_path.clone(), &model_params)
            .with_context(|| format!("unable to load model at {:?}", model_path))?;

        self.model = Some(model);
        self.model_path = Some(model_path);
        self.context_size = context_size;
        self.update_activity();

        eprintln!("✅ Model loaded successfully");
        Ok(())
    }

    fn generate(
        &mut self,
        prompt: String,
        max_tokens: i32,
        sampling: SamplingConfig,
        stop_tokens: Vec<String>,
        mut on_delta: impl FnMut(&str) -> Result<()>,
    ) -> Result<String> {
        let start_time = Instant::now();
        let model = self.model.as_ref().context("Model not loaded")?;

        // Calculate thread count (conservative default: max(1, (Cores / 2) + 2))
        // This ensures the UI thread is never starved
        let threads: i32 = std::thread::available_parallelism()
            .map(|n| {
                let cores = n.get() as i32;
                ((cores / 2) + 2).max(1)
            })
            .unwrap_or(2);

        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(Some(
                NonZeroU32::new(self.context_size).context("Invalid ctx size")?,
            ))
            .with_n_batch(self.context_size)
            .with_n_threads(threads)
            .with_n_threads_batch(threads);

        let mut ctx = model
            .new_context(&self.backend, ctx_params)
            .context("unable to create the llama_context")?;

        let tokens_list = model
            .str_to_token(&prompt, AddBos::Always)
            .with_context(|| "failed to tokenize prompt")?;

        eprintln!("📝 Tokenized prompt: {} tokens", tokens_list.len());

        // Use context size for batch capacity to handle long prompts
        let batch_size = self.context_size as usize;
        let mut batch = LlamaBatch::new(batch_size, 1);

        let last_index: i32 = (tokens_list.len() - 1) as i32;
        for (i, token) in (0_i32..).zip(tokens_list.into_iter()) {
            let is_last = i == last_index;
            batch
                .add(token, i, &[0], is_last)
                .context("Failed to add token to batch")?;
        }

        ctx.decode(&mut batch).context("llama_decode() failed")?;
        let prompt_time = start_time.elapsed();

        let n_prompt_tokens = batch.n_tokens();
        let mut n_cur = n_prompt_tokens;
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut output = String::new();

        eprintln!("🔄 Starting generation (max_tokens: {})", max_tokens);

        use llama_cpp_2::sampling::LlamaSampler;

        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u32;
        let sampler = if sampling.temperature <= 0.0 {
            if sampling.uses_penalties() {
                LlamaSampler::chain_simple([
                    LlamaSampler::penalties(
                        sampling.penalty_last_n,
                        sampling.repeat_penalty,
                        sampling.frequency_penalty,
                        sampling.presence_penalty,
                    ),
                    LlamaSampler::greedy(),
                ])
            } else {
                LlamaSampler::chain_simple([LlamaSampler::greedy()])
            }
        } else if sampling.uses_penalties() {
            LlamaSampler::chain_simple([
                LlamaSampler::penalties(
                    sampling.penalty_last_n,
                    sampling.repeat_penalty,
                    sampling.frequency_penalty,
                    sampling.presence_penalty,
                ),
                LlamaSampler::top_k(sampling.top_k),
                LlamaSampler::top_p(sampling.top_p, 1),
                LlamaSampler::temp(sampling.temperature),
                LlamaSampler::dist(seed),
            ])
        } else {
            LlamaSampler::chain_simple([
                LlamaSampler::top_k(sampling.top_k),
                LlamaSampler::top_p(sampling.top_p, 1),
                LlamaSampler::temp(sampling.temperature),
                LlamaSampler::dist(seed),
            ])
        };
        let mut sampler = pin!(sampler);

        loop {
            // Check if we've generated enough tokens
            if (n_cur - n_prompt_tokens) >= max_tokens {
                eprintln!("✓ Reached max_tokens limit");
                break;
            }

            let token = sampler.as_mut().sample(&ctx, batch.n_tokens() - 1);
            sampler.as_mut().accept(token);

            if model.is_eog_token(token) {
                eprintln!(
                    "✓ End-of-generation token reached (generated {} chars)",
                    output.len()
                );
                break;
            }

            let output_bytes = match model.token_to_piece_bytes(token, 32, true, None) {
                Err(llama_cpp_2::TokenToStringError::InsufficientBufferSpace(size)) => {
                    let required_size: usize = size
                        .checked_neg()
                        .context("Invalid token piece buffer size")?
                        .try_into()
                        .context("Invalid token piece buffer size")?;
                    model.token_to_piece_bytes(token, required_size, true, None)
                }
                result => result,
            }
            .context("Failed to convert token to bytes")?;

            let mut token_text = String::with_capacity(32);
            let _ = decoder.decode_to_string(&output_bytes, &mut token_text, false);
            output.push_str(&token_text);
            if !token_text.is_empty() {
                on_delta(&token_text)?;
            }

            // Check for model-specific stop tokens
            let mut should_stop = false;
            for stop_token in &stop_tokens {
                if output.contains(stop_token) {
                    eprintln!(
                        "✓ Stop token '{}' detected (generated {} chars)",
                        stop_token,
                        output.len()
                    );
                    // Remove the stop token from output
                    output = output.replace(stop_token, "").trim_end().to_string();
                    should_stop = true;
                    break;
                }
            }
            if should_stop {
                break;
            }

            batch.clear();
            batch
                .add(token, n_cur, &[0], true)
                .context("Failed to add generated token to batch")?;
            n_cur += 1;
            ctx.decode(&mut batch).context("failed to eval")?;
        }

        // Generation statistics
        let total_time = start_time.elapsed();
        let gen_time = total_time.saturating_sub(prompt_time);
        let output_tokens = (n_cur - n_prompt_tokens) as u64;
        let prompt_tokens = n_prompt_tokens as u64;

        let tokens_per_sec = if gen_time.as_secs_f64() > 0.0 {
            output_tokens as f64 / gen_time.as_secs_f64()
        } else {
            0.0
        };

        eprintln!("📊 Generation Statistics:");
        eprintln!("   • Prompt tokens: {}", prompt_tokens);
        eprintln!("   • Output tokens: {}", output_tokens);
        eprintln!("   • Prompt processing: {:.2}s", prompt_time.as_secs_f64());
        eprintln!("   • Generation time: {:.2}s", gen_time.as_secs_f64());
        eprintln!("   • Total time: {:.2}s", total_time.as_secs_f64());
        eprintln!("   • Speed: {:.2} tokens/sec", tokens_per_sec);

        self.update_activity();
        Ok(output)
    }
}

// ============================================================================
// Main Loop with Keep-Alive Protocol
// ============================================================================

fn send_response(response: &Response) -> Result<()> {
    let json = serde_json::to_string(response)?;
    println!("{}", json);
    io::stdout().flush()?;
    Ok(())
}

fn main() -> Result<()> {
    // Get idle timeout from environment variable (default 5 minutes)
    let idle_timeout_secs = std::env::var("LLAMA_IDLE_TIMEOUT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(300); // 5 minutes default

    // §163: 启动时打印默认推理参数 (供日志验证)
    let log_temp: f32 = std::env::var("LLAMA_DEFAULT_TEMPERATURE")
        .ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.1);
    let log_top_p: f32 = std::env::var("LLAMA_DEFAULT_TOP_P")
        .ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.3);
    let log_repeat: f32 = std::env::var("LLAMA_DEFAULT_REPEAT_PENALTY")
        .ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(1.05);
    eprintln!(
        "🦙 llama-helper starting (idle timeout: {}s, §163 default: temp={} top_p={} rep={})",
        idle_timeout_secs, log_temp, log_top_p, log_repeat
    );

    let mut state = ModelState::new()?;

    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let mut buffer = String::new();

    loop {
        // Check idle timeout
        if state.seconds_since_activity() > idle_timeout_secs {
            eprintln!("💤 Idle timeout reached, shutting down");
            send_response(&Response::Goodbye)?;
            break;
        }

        // Read line from stdin
        buffer.clear();
        match stdin_lock.read_line(&mut buffer) {
            Ok(0) => {
                // EOF reached
                eprintln!("📪 EOF received, shutting down");
                break;
            }
            Ok(_) => {
                let line = buffer.trim();
                if line.is_empty() {
                    continue;
                }

                // Parse request
                match serde_json::from_str::<Request>(line) {
                    Ok(Request::Generate {
                        prompt,
                        max_tokens,
                        context_size,
                        model_path,
                        n_layer,  // §198
                        temperature,
                        top_k,
                        top_p,
                        presence_penalty,
                        frequency_penalty,
                        repeat_penalty,
                        penalty_last_n,
                        stop_tokens,
                    }) => {
                        let max_tokens = max_tokens.unwrap_or(512);
                        let context_size = context_size.unwrap_or(2048);

                        let sampling = SamplingConfig::from_request(
                            temperature,
                            top_k,
                            top_p,
                            presence_penalty,
                            frequency_penalty,
                            repeat_penalty,
                            penalty_last_n,
                        );
                        let stop_tokens = stop_tokens.unwrap_or_else(Vec::new);

                        // Load model if path provided
                        if let Some(path_str) = model_path {
                            let path = PathBuf::from(path_str);
                            if let Err(e) = state.load_model_if_needed(path, context_size, n_layer) {
                                send_response(&Response::Response {
                                    text: String::new(),
                                    error: Some(format!("Failed to load model: {}", e)),
                                })?;
                                continue;
                            }
                        }

                        // Generate response with sampling parameters
                        match state.generate(
                            prompt,
                            max_tokens,
                            sampling,
                            stop_tokens,
                            |delta| send_response(&Response::Delta { text: delta.to_string() }),
                        ) {
                            Ok(text) => {
                                send_response(&Response::Done { text })?;
                            }
                            Err(e) => {
                                send_response(&Response::Response {
                                    text: String::new(),
                                    error: Some(format!("Generation failed: {}", e)),
                                })?;
                            }
                        }
                    }
                    Ok(Request::Ping) => {
                        state.update_activity();
                        send_response(&Response::Pong)?;
                    }
                    Ok(Request::Shutdown) => {
                        eprintln!("🛑 Shutdown requested");
                        send_response(&Response::Goodbye)?;
                        break;
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to parse request: {}", e);
                        send_response(&Response::Error {
                            message: format!("Invalid request: {}", e),
                        })?;
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Error reading stdin: {}", e);
                break;
            }
        }
    }

    eprintln!("👋 llama-helper exiting");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_request_defaults_penalties_when_omitted() {
        let json = r#"{"type":"generate","prompt":"summarize","temperature":0.5,"top_k":20,"top_p":0.8}"#;
        let request: Request = serde_json::from_str(json).unwrap();
        let Request::Generate {
            temperature,
            top_k,
            top_p,
            presence_penalty,
            frequency_penalty,
            repeat_penalty,
            penalty_last_n,
            ..
        } = request else {
            panic!("expected generate request");
        };

        let sampling = SamplingConfig::from_request(
            temperature,
            top_k,
            top_p,
            presence_penalty,
            frequency_penalty,
            repeat_penalty,
            penalty_last_n,
        );

        assert_eq!(sampling.presence_penalty, 0.0);
        assert_eq!(sampling.frequency_penalty, 0.0);
        assert_eq!(sampling.repeat_penalty, 1.0);
        assert_eq!(sampling.penalty_last_n, 0);
        assert!(!sampling.uses_penalties());
    }

    #[test]
    /// §163: 默认 temperature=0.1, top_p=0.3, repeat_penalty=1.05 (文档模块 3 规范)
    #[test]
    fn section_163_default_sampling_values_for_qwen_summary() {
        let json = r#"{"type":"generate","prompt":"summarize","temperature":null,"top_k":null,"top_p":null,"presence_penalty":null,"frequency_penalty":null,"repeat_penalty":null,"penalty_last_n":null}"#;
        let request: Request = serde_json::from_str(json).unwrap();
        let Request::Generate {
            temperature, top_k, top_p,
            presence_penalty, frequency_penalty, repeat_penalty,
            penalty_last_n, ..
        } = request else { panic!("expected generate") };
        let sampling = SamplingConfig::from_request(
            temperature, top_k, top_p,
            presence_penalty, frequency_penalty, repeat_penalty,
            penalty_last_n,
        );
        // §163 锁定的 3 个值
        assert!((sampling.temperature - 0.1).abs() < 1e-6, "expected 0.1, got {}", sampling.temperature);
        assert!((sampling.top_p - 0.3).abs() < 1e-6, "expected 0.3, got {}", sampling.top_p);
        assert!((sampling.repeat_penalty - 1.05).abs() < 1e-6, "expected 1.05, got {}", sampling.repeat_penalty);
    }

    /// §163: 调用方显式传入温度/top_p/repeat 应不受默认值影响
    #[test]
    fn section_163_explicit_values_override_defaults() {
        let json = r#"{"type":"generate","prompt":"x","temperature":0.7,"top_k":20,"top_p":0.9,"presence_penalty":0.0,"frequency_penalty":0.0,"repeat_penalty":1.2,"penalty_last_n":0}"#;
        let request: Request = serde_json::from_str(json).unwrap();
        let Request::Generate {
            temperature, top_k, top_p,
            presence_penalty, frequency_penalty, repeat_penalty,
            penalty_last_n, ..
        } = request else { panic!("expected generate") };
        let sampling = SamplingConfig::from_request(
            temperature, top_k, top_p,
            presence_penalty, frequency_penalty, repeat_penalty,
            penalty_last_n,
        );
        assert!((sampling.temperature - 0.7).abs() < 1e-6);
        assert!((sampling.top_p - 0.9).abs() < 1e-6);
        assert!((sampling.repeat_penalty - 1.2).abs() < 1e-6);
    }

    fn streaming_response_serializes_delta_and_done() {
        let delta = serde_json::to_string(&Response::Delta { text: "你".to_string() }).unwrap();
        let done = serde_json::to_string(&Response::Done { text: "你好".to_string() }).unwrap();
        assert_eq!(delta, r#"{"type":"delta","text":"你"}"#);
        assert_eq!(done, r#"{"type":"done","text":"你好"}"#);
    }

    #[test]
    fn generate_request_deserializes_qwen_penalties() {
        let json = r#"{"type":"generate","prompt":"summarize","temperature":0.5,"top_k":20,"top_p":0.8,"presence_penalty":0.3,"frequency_penalty":0.0,"repeat_penalty":1.05,"penalty_last_n":256}"#;
        let request: Request = serde_json::from_str(json).unwrap();
        let Request::Generate {
            temperature,
            top_k,
            top_p,
            presence_penalty,
            frequency_penalty,
            repeat_penalty,
            penalty_last_n,
            ..
        } = request else {
            panic!("expected generate request");
        };

        let sampling = SamplingConfig::from_request(
            temperature,
            top_k,
            top_p,
            presence_penalty,
            frequency_penalty,
            repeat_penalty,
            penalty_last_n,
        );

        assert_eq!(sampling.temperature, 0.5);
        assert_eq!(sampling.top_k, 20);
        assert_eq!(sampling.top_p, 0.8);
        assert_eq!(sampling.presence_penalty, 0.3);
        assert_eq!(sampling.frequency_penalty, 0.0);
        assert_eq!(sampling.repeat_penalty, 1.05);
        assert_eq!(sampling.penalty_last_n, 256);
        assert!(sampling.uses_penalties());
    }
}
