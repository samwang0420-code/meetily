// Tauri commands for built-in AI model management
// Exposes model download, status, and management functionality to frontend

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tokio::sync::Mutex;

use super::model_manager::{DownloadProgress, ModelInfo, ModelManager};

const QWEN35_4B_RECOMMENDED_RAM_GB: u64 = 14;

pub(crate) fn summary_model_priority(model_name: &str) -> u8 {
    match model_name {
        "qwen2.5:3b" => 4,  // §190: qwen2.5:3b is higher priority (replaces qwen3.5:4b)
        "qwen3.5:4b" => 3,  // legacy
        "qwen2.5:1.5b" => 2, // §190: fallback for <8GB RAM
        "qwen3.5:2b" => 1,   // legacy fallback
        "gemma3:4b" => 0,
        "gemma3:1b" => 0,
        _ => 0,
    }
}

pub(crate) fn recommend_summary_model(_is_macos: bool, system_ram_gb: u64) -> &'static str {
    // §190: 用 Qwen2.5-3B-Instruct 替换 Qwen3.5-2B
    //   ≥16GB → qwen2.5:3b (高质量)
    //   ≥8GB → qwen2.5:3b (主流 8GB 机型, ~4GB RAM 加载)
    //   <8GB → qwen2.5:1.5b (轻量)
    if system_ram_gb >= 8 {
        "qwen2.5:3b"
    } else {
        "qwen2.5:1.5b"
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
    fn recommended_summary_model_uses_qwen_1_5b_below_8gb_floor() {
        // §190: <8GB RAM → qwen2.5:1.5b (轻量)
        assert_eq!(recommend_summary_model(true, 7), "qwen2.5:1.5b");
        assert_eq!(recommend_summary_model(false, 7), "qwen2.5:1.5b");
    }

    #[test]
    fn recommended_summary_model_uses_qwen_3b_at_8gb_floor() {
        // §190: ≥8GB RAM → qwen2.5:3b
        assert_eq!(recommend_summary_model(true, 8), "qwen2.5:3b");
        assert_eq!(recommend_summary_model(false, 8), "qwen2.5:3b");
        assert_eq!(recommend_summary_model(true, 16), "qwen2.5:3b");
        assert_eq!(recommend_summary_model(false, 32), "qwen2.5:3b");
    }

    #[test]
    fn available_summary_model_priority_prefers_qwen_2_5_3b() {
        // §190: qwen2.5:3b > qwen2.5:1.5b > legacy qwen3.5 系列
        assert!(summary_model_priority("qwen2.5:3b") > summary_model_priority("qwen2.5:1.5b"));
        assert!(summary_model_priority("qwen2.5:3b") > summary_model_priority("qwen3.5:4b"));
        assert!(summary_model_priority("qwen2.5:1.5b") > summary_model_priority("qwen3.5:2b"));
    }
}
