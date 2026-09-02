// Tauri commands for built-in AI model management
// Exposes model download, status, and management functionality to frontend

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tokio::sync::Mutex;

use super::model_manager::{DownloadProgress, ModelInfo, ModelManager};

const QWEN35_4B_RECOMMENDED_RAM_GB: u64 = 14;

pub(crate) fn summary_model_priority(model_name: &str) -> u8 {
    match model_name {
        // §205.1 (2026-09-02): Spark X2.5 1.7B 优先于 Qwen 2.5 3B (公开 benchmark 17/19 胜).
        //   但仅当用户已下载 — model_manager 默认按 priority 选 available, 未下载会被跳过.
        "spark-x2.5:1.7b" => 5,  // §205.1: 8-15GB Apple Silicon 高级选项 (待 §205.2 llama.cpp fork)
        "qwen2.5:3b" => 4,  // §190: 主推, ≥16GB 用户首选
        "qwen3.5:4b" => 3,  // legacy High Quality
        "qwen3.5:2b" => 2,  // §190.2: 8GB 主流机型默认 (替代已删除的 qwen2.5:1.5b 占位)
        "gemma3:4b" => 1,
        "gemma3:1b" => 0,
        _ => 0,
    }
}

pub(crate) fn recommend_summary_model(is_macos: bool, system_ram_gb: u64) -> &'static str {
    // §205.1 (2026-09-02): Spark X2.5 1.7B 加入 RAM-adaptive 表.
    //   ≥16GB                 → qwen2.5:3b (2.1GB, 已实测稳定, 质量足够)
    //   8-15GB Apple Silicon  → spark-x2.5:1.7b (1.1GB Q4_K_M, 中文 benchmark 显著优势)
    //   ≥10GB Apple Silicon   → spark-x2.5:1.7b (M2/M3 10GB+ 机型吃 1.7B 模型 + KV cache)
    //   8GB                   → qwen3.5:2b (1.2GB, 8GB 主流机型保稳, spark 1.1GB 略紧)
    //   <8GB                  → qwen3.5:2b (1.2GB, 低端设备保稳)
    //
    // Why 8GB 仍走 qwen3.5:2b not spark: spark 量化后 ~1.1GB + KV cache 0.5GB + system 3GB +
    // app 1GB = 5.6GB, 跟 8GB 设备余量只有 2.4GB, 加载 1.1GB 模型会触发内存压力 (≥1.2GB 阈值).
    // qwen3.5:2b 1.2GB 跟 spark 1.1GB 容量差异微小, 但 qwen 24 层全 Metal offload 验证充分 (§197).
    if system_ram_gb >= 16 {
        "qwen2.5:3b"
    } else if system_ram_gb > 8 && system_ram_gb < 16 && is_macos {
        // §205.1: 9-15GB Apple Silicon (M1/M2/M3) → Spark X2.5 1.7B
        //   8GB 边界仍 qwen3.5:2b: spark 1.1GB + KV 0.5GB + system 3GB + app 1GB = 5.6GB 余 2.4GB 易触发 §86 内存降级
        "spark-x2.5:1.7b"
    } else if system_ram_gb >= 10 && is_macos {
        // §190.2 旧规则, 实际已被上面 8-15GB 分支覆盖 (10 < 16)
        "spark-x2.5:1.7b"
    } else {
        "qwen3.5:2b"
    }
}

pub(crate) fn get_recommended_summary_model_for_current_system() -> Result<&'static str, String> {
    let system_ram_gb = get_system_ram_gb()?;
    let is_macos = cfg!(target_os = "macos");

    log::info!(
        "System RAM detected: {} GB, Platform: {}",
        system_ram_gb,
        if is_macos { "macOS" } else { "other" }
    );

    Ok(recommend_summary_model(is_macos, system_ram_gb))
}

// ============================================================================
// Global State
// ============================================================================

/// Global model manager instance
pub struct ModelManagerState(pub Arc<Mutex<Option<Arc<ModelManager>>>>);

/// Initialize the model manager
pub async fn init_model_manager<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<()> {
    let models_dir = app.path().app_data_dir()?.join("models").join("summary");

    let manager = ModelManager::new_with_models_dir(Some(models_dir))?;
    manager.init().await?;

    let state: State<ModelManagerState> = app.state();
    let mut manager_lock = state.0.lock().await;
    *manager_lock = Some(Arc::new(manager));

    log::info!("Built-in AI model manager initialized");
    Ok(())
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// List all available built-in AI models with their status
#[tauri::command]
pub async fn builtin_ai_list_models<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ModelManagerState>,
) -> Result<Vec<ModelInfo>, String> {
    let manager = {
        // Ensure manager is initialized
        {
            let manager_lock = state.0.lock().await;
            if manager_lock.is_none() {
                drop(manager_lock);
                init_model_manager(&app)
                    .await
                    .map_err(|e| format!("Failed to initialize model manager: {}", e))?;
            }
        }

        let manager_lock = state.0.lock().await;
        manager_lock
            .as_ref()
            .ok_or_else(|| "Model manager not initialized".to_string())?
            .clone()
    };

    let models = manager.list_models().await;
    Ok(models)
}

/// Get information about a specific model
#[tauri::command]
pub async fn builtin_ai_get_model_info<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ModelManagerState>,
    model_name: String,
) -> Result<Option<ModelInfo>, String> {
    let manager = {
        // Ensure manager is initialized
        {
            let manager_lock = state.0.lock().await;
            if manager_lock.is_none() {
                drop(manager_lock);
                init_model_manager(&app)
                    .await
                    .map_err(|e| format!("Failed to initialize model manager: {}", e))?;
            }
        }

        let manager_lock = state.0.lock().await;
        manager_lock
            .as_ref()
            .ok_or_else(|| "Model manager not initialized".to_string())?
            .clone()
    };

    let info = manager.get_model_info(&model_name).await;
    Ok(info)
}

/// Download a built-in AI model with progress updates
#[tauri::command]
pub async fn builtin_ai_download_model<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ModelManagerState>,
    model_name: String,
) -> Result<(), String> {
    let manager = {
        // Ensure manager is initialized
        {
            let manager_lock = state.0.lock().await;
            if manager_lock.is_none() {
                drop(manager_lock);
                init_model_manager(&app)
                    .await
                    .map_err(|e| format!("Failed to initialize model manager: {}", e))?;
            }
        }

        let manager_lock = state.0.lock().await;
        manager_lock
            .as_ref()
            .ok_or_else(|| "Model manager not initialized".to_string())?
            .clone() // Clone the Arc, not the ModelManager
    };
    // IMPORTANT: Only emit "downloading" status here, never "completed"
    // Completion event is emitted AFTER download task fully finishes (validation, etc.)
    let app_clone = app.clone();
    let model_name_clone = model_name.clone();
    let progress_callback = Box::new(move |progress: DownloadProgress| {
        let _ = app_clone.emit(
            "builtin-ai-download-progress",
            serde_json::json!({
                "model": model_name_clone,
                "progress": progress.percent,
                "downloaded_mb": progress.downloaded_mb,
                "total_mb": progress.total_mb,
                "speed_mbps": progress.speed_mbps,
                "status": "downloading"  // Always "downloading", never "completed" from progress callback
            }),
        );
    });

    match manager
        .download_model_detailed(&model_name, Some(progress_callback))
        .await
    {
        Ok(_) => {
            // Download task completed successfully (validation passed, status set to Available)
            let _ = app.emit(
                "builtin-ai-download-progress",
                serde_json::json!({
                    "model": model_name,
                    "progress": 100,
                    "downloaded_mb": 0,  // Not used by completion handler
                    "total_mb": 0,       // Not used by completion handler
                    "speed_mbps": 0,     // Not used by completion handler
                    "status": "completed"
                }),
            );
            Ok(())
        },
        Err(e) => {
            let error_msg = e.to_string();

            // Check if this is a cancellation error (marked with "CANCELLED:" prefix)
            // Don't emit error event for cancellations - cancel command already emits cancelled event
            if !error_msg.starts_with("CANCELLED:") {
                // Emit error via progress event for frontend to display (only for real errors)
                let _ = app.emit(
                    "builtin-ai-download-progress",
                    serde_json::json!({
                        "model": model_name,
                        "progress": 0,
                        "downloaded_mb": 0,
                        "total_mb": 0,
                        "speed_mbps": 0,
                        "status": "error",
                        "error": error_msg
                    }),
                );
            }
            Err(error_msg)
        }
    }
}

/// Cancel an ongoing model download
#[tauri::command]
pub async fn builtin_ai_cancel_download<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ModelManagerState>,
    model_name: String,
) -> Result<(), String> {
    let manager = {
        let manager_lock = state.0.lock().await;
        manager_lock
            .as_ref()
            .ok_or_else(|| "Model manager not initialized".to_string())?
            .clone()
    };

    manager
        .cancel_download(&model_name)
        .await
        .map_err(|e| e.to_string())?;

    let _ = app.emit(
        "builtin-ai-download-progress",
        serde_json::json!({
            "model": model_name,
            "progress": 0,
            "status": "cancelled"
        }),
    );

    Ok(())
}

/// Delete a corrupted or available model file
#[tauri::command]
pub async fn builtin_ai_delete_model(
    state: State<'_, ModelManagerState>,
    model_name: String,
) -> Result<(), String> {
    let manager = {
        let manager_lock = state.0.lock().await;
        manager_lock
            .as_ref()
            .ok_or_else(|| "Model manager not initialized".to_string())?
            .clone()
    };

    manager
        .delete_model(&model_name)
        .await
        .map_err(|e| e.to_string())
}

/// Check if a model is ready to use
#[tauri::command]
pub async fn builtin_ai_is_model_ready<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ModelManagerState>,
    model_name: String,
    refresh: Option<bool>,  // NEW: Optional refresh parameter
) -> Result<bool, String> {
    let manager = {
        // Ensure manager is initialized
        {
            let manager_lock = state.0.lock().await;
            if manager_lock.is_none() {
                drop(manager_lock);
                init_model_manager(&app)
                    .await
                    .map_err(|e| format!("Failed to initialize model manager: {}", e))?;
            }
        }

        let manager_lock = state.0.lock().await;
        manager_lock
            .as_ref()
            .ok_or_else(|| "Model manager not initialized".to_string())?
            .clone()
    };

    let refresh_scan = refresh.unwrap_or(false);
    let ready = manager.is_model_ready(&model_name, refresh_scan).await;

    log::info!(
        "Model '{}' ready check (refresh={}): {}",
        model_name,
        refresh_scan,
        ready
    );

    Ok(ready)
}

/// Check if any summary model is available (for onboarding)
/// Returns the first available model name by priority, or None if no models exist
#[tauri::command]
pub async fn builtin_ai_get_available_summary_model<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ModelManagerState>,
) -> Result<Option<String>, String> {
    let manager = {
        // Ensure manager is initialized
        {
            let manager_lock = state.0.lock().await;
            if manager_lock.is_none() {
                drop(manager_lock);
                init_model_manager(&app)
                    .await
                    .map_err(|e| format!("Failed to initialize model manager: {}", e))?;
            }
        }

        let manager_lock = state.0.lock().await;
        manager_lock
            .as_ref()
            .ok_or_else(|| "Model manager not initialized".to_string())?
            .clone()
    };

    // Force fresh scan to ensure accurate state
    manager
        .scan_models()
        .await
        .map_err(|e| format!("Failed to scan models: {}", e))?;

    // Get all available models
    let all_models = manager.list_models().await;

    // Find first available summary model
    let available = all_models
        .iter()
        .filter(|m| matches!(m.status, crate::summary::summary_engine::model_manager::ModelStatus::Available))
        .max_by_key(|m| summary_model_priority(&m.name))
        .map(|m| m.name.clone());

    log::info!("Available summary model check: {:?}", available);
    Ok(available)
}

// ============================================================================
// Startup Initialization & Utility Commands
// ============================================================================

pub async fn init_model_manager_at_startup<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<(), String> {
    let models_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("models")
        .join("summary");

    let manager = ModelManager::new_with_models_dir(Some(models_dir))
        .map_err(|e| format!("Failed to create ModelManager: {}", e))?;

    manager
        .init()
        .await
        .map_err(|e| format!("Failed to initialize ModelManager: {}", e))?;

    let state: State<ModelManagerState> = app.state();
    let mut manager_lock = state.0.lock().await;
    *manager_lock = Some(Arc::new(manager));

    log::info!("ModelManager initialized at startup");
    Ok(())
}


/// Get recommended summary model based on platform and system RAM.
/// §190: 用 Qwen2.5-3B-Instruct 替换 Qwen3.5 系列
///   ≥8GB → qwen2.5:3b
///   <8GB → qwen2.5:1.5b
#[tauri::command]
pub async fn builtin_ai_get_recommended_model() -> Result<String, String> {
    let recommended = get_recommended_summary_model_for_current_system()?;

    log::info!("Recommended summary model: {}", recommended);
    Ok(recommended.to_string())
}

/// Get total system RAM in gigabytes
fn get_system_ram_gb() -> Result<u64, String> {
    use sysinfo::System;

    let mut sys = System::new_all();
    sys.refresh_memory();

    let total_memory_bytes = sys.total_memory();
    let total_memory_gb = total_memory_bytes / (1024 * 1024 * 1024);

    Ok(total_memory_gb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommended_summary_model_uses_qwen35_2b_below_8gb_floor() {
        // §190.2 (2026-08-31): <8GB RAM → qwen3.5:2b (1.2GB, registered, 8GB 设备跑得动)
        // §205.1 (2026-09-02): 8GB 边界不变 — spark 1.1GB Q4_K_M + KV cache + system 太紧
        assert_eq!(recommend_summary_model(true, 7), "qwen3.5:2b");
        assert_eq!(recommend_summary_model(false, 7), "qwen3.5:2b");
        assert_eq!(recommend_summary_model(true, 4), "qwen3.5:2b");
        assert_eq!(recommend_summary_model(false, 4), "qwen3.5:2b");
    }

    #[test]
    fn recommended_summary_model_uses_qwen35_2b_at_exactly_8gb_boundary() {
        // §205.1 (2026-09-02): 8GB boundary 仍走 qwen3.5:2b, spark 从 9GB 开始
        //   8GB 设备 spark 1.1GB + KV 0.5GB + system 3GB + app 1GB = 5.6GB, 余量 2.4GB 易触发 §86 内存降级
        assert_eq!(recommend_summary_model(true, 8), "qwen3.5:2b");
        assert_eq!(recommend_summary_model(false, 8), "qwen3.5:2b"); // Intel 8GB 没 Metal
    }

    #[test]
    fn recommended_summary_model_uses_spark_x25_for_9gb_to_15gb_apple_silicon() {
        // §205.1 (2026-09-02): 9-15GB Apple Silicon (M1/M2/M3) → spark-x2.5:1.7b
        //   替换 §190.2 的 qwen2.5:3b, Spark benchmark 中文 Gaokao +20.8 显著优势
        assert_eq!(recommend_summary_model(true, 9), "spark-x2.5:1.7b");
        assert_eq!(recommend_summary_model(true, 10), "spark-x2.5:1.7b");
        assert_eq!(recommend_summary_model(true, 12), "spark-x2.5:1.7b"); // M2 12GB
        assert_eq!(recommend_summary_model(true, 15), "spark-x2.5:1.7b");
        // Intel 10GB 没 Metal GEMV, spark CPU-bound 太慢, 仍走 qwen3.5:2b
        assert_eq!(recommend_summary_model(false, 10), "qwen3.5:2b");
    }

    #[test]
    fn recommended_summary_model_uses_qwen_3b_for_16gb_or_larger() {
        // §205.1 (2026-09-02): ≥16GB → qwen2.5:3b (2.1GB, 已实测稳定)
        //   Spark 1.7B 比 Qwen 2.5 3B 容量小, 但 Qwen 3B 已实测稳定, 高 RAM 用户优先确定性
        assert_eq!(recommend_summary_model(true, 16), "qwen2.5:3b");
        assert_eq!(recommend_summary_model(false, 16), "qwen2.5:3b");
        assert_eq!(recommend_summary_model(true, 32), "qwen2.5:3b");
        assert_eq!(recommend_summary_model(false, 32), "qwen2.5:3b");
    }

    #[test]
    fn available_summary_model_priority_prefers_spark_x25_over_qwen() {
        // §205.1: spark-x2.5:1.7b (5) > qwen2.5:3b (4) > qwen3.5:4b (3) > qwen3.5:2b (2) > gemma3:4b (1) > gemma3:1b (0)
        assert!(summary_model_priority("spark-x2.5:1.7b") > summary_model_priority("qwen2.5:3b"));
        assert!(summary_model_priority("qwen2.5:3b") > summary_model_priority("qwen3.5:4b"));
        assert!(summary_model_priority("qwen3.5:4b") > summary_model_priority("qwen3.5:2b"));
        assert!(summary_model_priority("qwen3.5:2b") > summary_model_priority("gemma3:4b"));
        assert!(summary_model_priority("gemma3:4b") > summary_model_priority("gemma3:1b"));
    }

    #[test]
    fn section_205_1_unknown_model_falls_through_to_qwen35_2b() {
        // §205.1: 未知 model 名 (e.g. "foobar:1b") 不被 priority 识别, RAM 推荐按 RAM 走
        assert_eq!(summary_model_priority("foobar:1b"), 0);
        // 验证 RAM 推荐仍工作
        assert_eq!(recommend_summary_model(true, 8), "qwen3.5:2b");
        assert_eq!(recommend_summary_model(true, 12), "spark-x2.5:1.7b");
        assert_eq!(recommend_summary_model(true, 16), "qwen2.5:3b");
    }
}
