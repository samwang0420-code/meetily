use crate::user::commands::SessionStore;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex as StdMutex;
// Removed unused import

// Performance optimization: Conditional logging macros for hot paths
#[cfg(debug_assertions)]
macro_rules! perf_debug {
    ($($arg:tt)*) => {
        log::debug!($($arg)*)
    };
}

#[cfg(not(debug_assertions))]
macro_rules! perf_debug {
    ($($arg:tt)*) => {};
}

#[cfg(debug_assertions)]
macro_rules! perf_trace {
    ($($arg:tt)*) => {
        log::trace!($($arg)*)
    };
}

#[cfg(not(debug_assertions))]
macro_rules! perf_trace {
    ($($arg:tt)*) => {};
}

// Make these macros available to other modules
pub(crate) use perf_debug;
pub(crate) use perf_trace;

// Re-export async logging macros for external use (removed due to macro conflicts)

// Declare audio module
pub mod analytics;
pub mod api;
pub mod audio;
pub mod config;
pub mod console_utils;
pub mod database;
pub mod notifications;
pub mod ollama;
pub mod onboarding;
pub mod openai;
pub mod anthropic;
pub mod groq;
pub mod openrouter;
pub mod parakeet_engine;
pub mod state;
pub mod summary;
pub mod tray;
pub mod utils;
pub mod user;
pub mod whisper_engine;
pub mod speaker_auto_attach;

pub mod hardware;
pub mod action_items;
pub mod obsidian_export;
pub mod speaker_aliases;
pub mod topic_graph;
pub mod live_qa;

use audio::{list_audio_devices, AudioDevice, trigger_audio_permission};
use log::{error as log_error, info as log_info};
use notifications::commands::NotificationManagerState;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::RwLock;

static RECORDING_FLAG: AtomicBool = AtomicBool::new(false);

// Global language preference storage (default to "auto-translate" for automatic translation to English)
static LANGUAGE_PREFERENCE: std::sync::LazyLock<StdMutex<String>> =
    std::sync::LazyLock::new(|| StdMutex::new("zh".to_string()));

#[derive(Debug, Deserialize)]
struct RecordingArgs {
    save_path: String,
}

#[derive(Debug, Serialize, Clone)]
struct TranscriptionStatus {
    chunks_in_queue: usize,
    is_processing: bool,
    last_activity_ms: u64,
}

#[tauri::command]
async fn start_recording<R: Runtime>(
    app: AppHandle<R>,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
    meeting_name: Option<String>,
) -> Result<(), String> {
    log_info!("🔥 CALLED start_recording with meeting: {:?}", meeting_name);
    log_info!(
        "📋 Backend received parameters - mic: {:?}, system: {:?}, meeting: {:?}",
        mic_device_name,
        system_device_name,
        meeting_name
    );

    if is_recording().await {
        return Err("Recording already in progress".to_string());
    }

    // Call the actual audio recording system with meeting name
    match audio::recording_commands::start_recording_with_devices_and_meeting(
        app.clone(),
        mic_device_name,
        system_device_name,
        meeting_name.clone(),
    )
    .await
    {
        Ok(_) => {
            RECORDING_FLAG.store(true, Ordering::SeqCst);
            tray::update_tray_menu(&app);

            log_info!("Recording started successfully");

            // Show recording started notification through NotificationManager
            // This respects user's notification preferences
            let notification_manager_state = app.state::<NotificationManagerState<R>>();
            if let Err(e) = notifications::commands::show_recording_started_notification(
                &app,
                &notification_manager_state,
                meeting_name.clone(),
            )
            .await
            {
                log_error!(
                    "Failed to show recording started notification: {}",
                    e
                );
            } else {
                log_info!("Successfully showed recording started notification");
            }

            Ok(())
        }
        Err(e) => {
            log_error!("Failed to start audio recording: {}", e);
            Err(format!("Failed to start recording: {}", e))
        }
    }
}

#[tauri::command]
async fn stop_recording<R: Runtime>(app: AppHandle<R>, args: RecordingArgs) -> Result<(), String> {
    log_info!("Attempting to stop recording...");

    // Check the actual audio recording system state instead of the flag
    if !audio::recording_commands::is_recording().await {
        log_info!("Recording is already stopped");
        return Ok(());
    }

    // Call the actual audio recording system to stop
    match audio::recording_commands::stop_recording(
        app.clone(),
        audio::recording_commands::RecordingArgs {
            save_path: args.save_path.clone(),
        },
    )
    .await
    {
        Ok(_) => {
            RECORDING_FLAG.store(false, Ordering::SeqCst);
            tray::update_tray_menu(&app);

            // v0.8.5 §23: Release sherpa daemon to free ~700MB Python + onnx models
            crate::audio::sherpa_daemon::shutdown_global_daemon();

            // Create the save directory if it doesn't exist
            if let Some(parent) = std::path::Path::new(&args.save_path).parent() {
                if !parent.exists() {
                    log_info!("Creating directory: {:?}", parent);
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        let err_msg = format!("Failed to create save directory: {}", e);
                        log_error!("{}", err_msg);
                        return Err(err_msg);
                    }
                }
            }

            // Show recording stopped notification through NotificationManager
            // This respects user's notification preferences
            let notification_manager_state = app.state::<NotificationManagerState<R>>();
            if let Err(e) = notifications::commands::show_recording_stopped_notification(
                &app,
                &notification_manager_state,
            )
            .await
            {
                log_error!(
                    "Failed to show recording stopped notification: {}",
                    e
                );
            } else {
                log_info!("Successfully showed recording stopped notification");
            }

            Ok(())
        }
        Err(e) => {
            log_error!("Failed to stop audio recording: {}", e);
            // Still update the flag even if stopping failed
            RECORDING_FLAG.store(false, Ordering::SeqCst);
            tray::update_tray_menu(&app);
            Err(format!("Failed to stop recording: {}", e))
        }
    }
}

#[tauri::command]
async fn is_recording() -> bool {
    audio::recording_commands::is_recording().await
}

#[tauri::command]
fn get_transcription_status() -> TranscriptionStatus {
    TranscriptionStatus {
        chunks_in_queue: 0,
        is_processing: false,
        last_activity_ms: 0,
    }
}

#[tauri::command]
fn read_audio_file(file_path: String) -> Result<Vec<u8>, String> {
    match std::fs::read(&file_path) {
        Ok(data) => Ok(data),
        Err(e) => Err(format!("Failed to read audio file: {}", e)),
    }
}

#[tauri::command]
async fn save_transcript(file_path: String, content: String) -> Result<(), String> {
    log_info!("Saving transcript to: {}", file_path);

    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&file_path).parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }

    // Write content to file
    std::fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write transcript: {}", e))?;

    log_info!("Transcript saved successfully");
    Ok(())
}

// Audio level monitoring commands
#[tauri::command]
async fn start_audio_level_monitoring<R: Runtime>(
    app: AppHandle<R>,
    device_names: Vec<String>,
) -> Result<(), String> {
    log_info!(
        "Starting audio level monitoring for devices: {:?}",
        device_names
    );

    audio::simple_level_monitor::start_monitoring(app, device_names)
        .await
        .map_err(|e| format!("Failed to start audio level monitoring: {}", e))
}

#[tauri::command]
async fn stop_audio_level_monitoring() -> Result<(), String> {
    log_info!("Stopping audio level monitoring");

    audio::simple_level_monitor::stop_monitoring()
        .await
        .map_err(|e| format!("Failed to stop audio level monitoring: {}", e))
}

#[tauri::command]
async fn is_audio_level_monitoring() -> bool {
    audio::simple_level_monitor::is_monitoring()
}

// Analytics commands are now handled by analytics::commands module

// Whisper commands are now handled by whisper_engine::commands module

#[tauri::command]
async fn get_audio_devices() -> Result<Vec<AudioDevice>, String> {
    list_audio_devices()
        .await
        .map_err(|e| format!("Failed to list audio devices: {}", e))
}

#[tauri::command]
async fn trigger_microphone_permission() -> Result<bool, String> {
    trigger_audio_permission()
        .map_err(|e| format!("Failed to trigger microphone permission: {}", e))
}

#[tauri::command]
async fn start_recording_with_devices<R: Runtime>(
    app: AppHandle<R>,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
) -> Result<(), String> {
    start_recording_with_devices_and_meeting(app, mic_device_name, system_device_name, None).await
}

#[tauri::command]
async fn start_recording_with_devices_and_meeting<R: Runtime>(
    app: AppHandle<R>,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
    meeting_name: Option<String>,
) -> Result<(), String> {
    log_info!("🚀 CALLED start_recording_with_devices_and_meeting - Mic: {:?}, System: {:?}, Meeting: {:?}",
             mic_device_name, system_device_name, meeting_name);

    // Clone meeting_name for notification use later
    let meeting_name_for_notification = meeting_name.clone();

    // Call the recording module functions that support meeting names
    let recording_result = match (mic_device_name.clone(), system_device_name.clone()) {
        (None, None) => {
            log_info!(
                "No devices specified, starting with defaults and meeting: {:?}",
                meeting_name
            );
            audio::recording_commands::start_recording_with_meeting_name(app.clone(), meeting_name)
                .await
        }
        _ => {
            log_info!(
                "Starting with specified devices: mic={:?}, system={:?}, meeting={:?}",
                mic_device_name,
                system_device_name,
                meeting_name
            );
            audio::recording_commands::start_recording_with_devices_and_meeting(
                app.clone(),
                mic_device_name,
                system_device_name,
                meeting_name,
            )
            .await
        }
    };

    match recording_result {
        Ok(_) => {
            log_info!("Recording started successfully via tauri command");

            // Show recording started notification through NotificationManager
            // This respects user's notification preferences
            let notification_manager_state = app.state::<NotificationManagerState<R>>();
            if let Err(e) = notifications::commands::show_recording_started_notification(
                &app,
                &notification_manager_state,
                meeting_name_for_notification.clone(),
            )
            .await
            {
                log_error!(
                    "Failed to show recording started notification: {}",
                    e
                );
            }

            Ok(())
        }
        Err(e) => {
            log_error!("Failed to start recording via tauri command: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
async fn set_language_preference(language: String) -> Result<(), String> {
    let mut lang_pref = LANGUAGE_PREFERENCE
        .lock()
        .map_err(|e| format!("Failed to set language preference: {}", e))?;
    log_info!("Setting language preference to: {}", language);
    *lang_pref = language;
    Ok(())
}

// Internal helper function to get language preference (for use within Rust code)
pub fn get_language_preference_internal() -> Option<String> {
    LANGUAGE_PREFERENCE.lock().ok().map(|lang| lang.clone())
}

// §97 (2026-08-09): identifier 改造后, 首次启动自动从旧 Bundle 目录 (cn.lixianhuiji.app)
/// 复制 SQLite + decode_cache + models 到新目录 (tech.yanjingai.app).
///
/// 设计原则 (§97 立铁律):
/// - 仅 COPY, 不删除旧目录 (§65 老数据观察期)
/// - 新目录已存在文件 → 跳过 (用户已有部分迁过来的不覆盖)
/// - 失败仅 warn, 不阻塞启动 (best-effort)
/// - 只复制 db 文件 (meeting_minutes.sqlite + -shm + -wal) + decode_cache (用户已有大文件不复制)
pub fn migrate_legacy_app_data() -> anyhow::Result<()> {
    use std::path::PathBuf;

    // 新旧路径
    let new_dir = if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| {
            let mut p = PathBuf::from(h);
            p.push(format!("Library/Application Support/{}", crate::config::APP_BUNDLE_ID));
            p
        })
    } else if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(|v| PathBuf::from(v).join(crate::config::APP_BUNDLE_ID))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(|v| PathBuf::from(v).join(crate::config::APP_BUNDLE_ID))
            .or_else(|| {
                std::env::var_os("HOME").map(|h| {
                    PathBuf::from(h).join(format!(".local/share/{}", crate::config::APP_BUNDLE_ID))
                })
            })
    };
    let new_dir = match new_dir {
        Some(d) => d,
        None => {
            log::info!("§97 migrate_legacy_app_data: cannot resolve new data dir, skip");
            return Ok(());
        }
    };

    let legacy_dir = if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| {
            let mut p = PathBuf::from(h);
            p.push(format!("Library/Application Support/{}", crate::config::APP_BUNDLE_ID_LEGACY));
            p
        })
    } else if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(|v| PathBuf::from(v).join(crate::config::APP_BUNDLE_ID_LEGACY))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(|v| PathBuf::from(v).join(crate::config::APP_BUNDLE_ID_LEGACY))
            .or_else(|| {
                std::env::var_os("HOME").map(|h| {
                    PathBuf::from(h).join(format!(".local/share/{}", crate::config::APP_BUNDLE_ID_LEGACY))
                })
            })
    };
    let legacy_dir = match legacy_dir {
        Some(d) => d,
        None => return Ok(()),
    };

    if !legacy_dir.exists() {
        log::info!("§97 migrate_legacy_app_data: legacy dir {:?} not found, skip", legacy_dir);
        return Ok(());
    }

    std::fs::create_dir_all(&new_dir).map_err(|e| anyhow::anyhow!("create_dir_all {:?}: {}", new_dir, e))?;

    // §97: 仅复制 db 文件 (3 个: sqlite + shm + wal), 不复制 decode_cache / models 大文件
    // 用户机器新目录已有 4G decode_cache + models, 复制 4.5G 老目录无意义且可能爆磁盘
    let files_to_copy = ["meeting_minutes.sqlite", "meeting_minutes.sqlite-shm", "meeting_minutes.sqlite-wal"];
    let mut copied: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    for fname in &files_to_copy {
        let src = legacy_dir.join(fname);
        let dst = new_dir.join(fname);
        if !src.exists() {
            continue;
        }
        if dst.exists() {
            skipped.push(fname.to_string());
            continue;
        }
        std::fs::copy(&src, &dst).map_err(|e| anyhow::anyhow!("copy {:?}: {}", src, e))?;
        copied.push(fname.to_string());
    }

    // §99: §97 漏复制 models/. 用户机器新目录 tech.yanjingai.app/models/ 存在但 sherpa 子目录不存在,
    // 旧目录 cn.lixianhuiji.app/models/sherpa/{funasr-nano-int8,paraformer-zh-int8} 是真模型 (~1.2GB),
    // 不复制 = sherpa_asr.py daemon 启动后 discovered 0 model packs, 导入转录 0 段识别.
    // 策略: 新目录 models/sherpa/ 不存在 OR 为空时, 递归复制旧目录整个 models/ 树.
    let src_models = legacy_dir.join("models");
    let dst_models = new_dir.join("models");
    let mut models_copied = 0usize;
    if src_models.is_dir() {
        let dst_sherpa = dst_models.join("sherpa");
        let need_copy = !dst_sherpa.is_dir()
            || std::fs::read_dir(&dst_sherpa).map(|mut it| it.next().is_none()).unwrap_or(true);
        if need_copy {
            copy_dir_recursive(&src_models, &dst_models, &mut models_copied)
                .map_err(|e| anyhow::anyhow!("copy models {:?} -> {:?}: {}", src_models, dst_models, e))?;
        } else {
            log::info!("§99 migrate_legacy_app_data: skip models/ (new dir already populated)");
        }
    }

    log::info!(
        "§99 migrate_legacy_app_data: copied_db={:?} skipped_db={:?} models_files_copied={} (legacy={:?}, new={:?})",
        copied, skipped, models_copied, legacy_dir, new_dir
    );
    Ok(())
}

/// §99.2 (2026-08-10): 一次性回填 user_id IS NULL 或 user_id = -1 的老 meetings/transcripts
/// 到当前登录用户 (latest_session_in_db). 跟 §26 §49 一致, 哨兵 -1 改为真实 user_id.
/// best-effort, 失败 warn 不阻塞启动.
async fn backfill_meeting_user_ids<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> anyhow::Result<()> {
    let app_state = app.try_state::<crate::state::AppState>()
        .ok_or_else(|| anyhow::anyhow!("AppState not available"))?;
    let pool = app_state.db_manager.pool();
    let user_id: i64 = match crate::user::commands::latest_session_in_db(app).await {
        Ok(Some((_, uid))) => uid,
        _ => {
            log::info!("§99.2 backfill: no active session, skip (避免误把数据挂到错误用户)");
            return Ok(());
        }
    };
    let m = sqlx::query("UPDATE meetings SET user_id = ? WHERE user_id IS NULL OR user_id = -1")
        .bind(user_id).execute(pool).await
        .map_err(|e| anyhow::anyhow!("UPDATE meetings: {}", e))?
        .rows_affected();
    let t = sqlx::query("UPDATE transcripts SET user_id = ? WHERE user_id IS NULL OR user_id = -1")
        .bind(user_id).execute(pool).await
        .map_err(|e| anyhow::anyhow!("UPDATE transcripts: {}", e))?
        .rows_affected();
    log::info!("§99.2 backfill_meeting_user_ids: meetings={} transcripts={} → user_id={}", m, t, user_id);
    Ok(())
}

/// §99: 递归复制目录树 (用于 §97 §99 models/ 迁移).
/// src/models/sherpa/funasr-nano-int8/*.onnx ~ 947MB, 必须用 copy (不能用 hardlink,
/// 因为 APFS 跨卷 hardlink 失败; 同卷 hardlink 反而让"复制"语义不清, fail 时排查难).
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path, count: &mut usize) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let src_child = entry.path();
        let dst_child = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir_recursive(&src_child, &dst_child, count)?;
        } else if ft.is_file() {
            if dst_child.exists() {
                continue;
            }
            std::fs::copy(&src_child, &dst_child)?;
            *count += 1;
        }
    }
    Ok(())
}

pub fn run() {
    log::set_max_level(log::LevelFilter::Info);

    // v0.7.0+: panic hook (写本地 crash log)
    install_panic_hook();

    let mut builder = tauri::Builder::default();

    #[cfg(any(target_os = "macos", windows, target_os = "linux"))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            log_info!(
                "Second app instance requested with args: {:?}, cwd: {:?}",
                args,
                cwd
            );

            tray::focus_main_window(app);
        }));
    }

    builder
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(whisper_engine::parallel_commands::ParallelProcessorState::new())
        .manage(Arc::new(RwLock::new(
            None::<notifications::manager::NotificationManager<tauri::Wry>>,
        )) as NotificationManagerState<tauri::Wry>)
        .manage(audio::init_system_audio_state())
        .manage(summary::summary_engine::ModelManagerState(Arc::new(tokio::sync::Mutex::new(None))))
        .manage(SessionStore::default())
        .setup(|_app| {
            // §97 (2026-08-09): identifier 改造后, 首次启动自动从旧 Bundle 目录 (cn.lixianhuiji.app)
            // 复制 SQLite + decode_cache + models 到新目录 (tech.yanjingai.app). 旧目录保留, 不删除.
            if let Err(e) = migrate_legacy_app_data() {
                log::warn!("§97 migrate_legacy_app_data failed (best-effort, continue): {}", e);
            }

            // §99.2/§101 (2026-08-10): backfill 移到 database init 之后 (line 700+),
            // 否则 race condition: backfill spawn 在 AppState 注册之前, try_state::<AppState> 返 None.
            // 之前版本: "§99.2 backfill_meeting_user_ids failed: AppState not available" warn 一直打.

            log::info!("Application setup complete");

            // Initialize system tray
            if let Err(e) = tray::create_tray(_app.handle()) {
                log::error!("Failed to create system tray: {}", e);
            }

            // Initialize notification system with proper defaults
            log::info!("Initializing notification system...");
            let app_for_notif = _app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let notif_state = app_for_notif.state::<NotificationManagerState<tauri::Wry>>();
                match notifications::commands::initialize_notification_manager(app_for_notif.clone()).await {
                    Ok(manager) => {
                        // Set default consent and permissions on first launch
                        if let Err(e) = manager.set_consent(true).await {
                            log::error!("Failed to set initial consent: {}", e);
                        }
                        if let Err(e) = manager.request_permission().await {
                            log::error!("Failed to request initial permission: {}", e);
                        }

                        // Store the initialized manager
                        let mut state_lock = notif_state.write().await;
                        *state_lock = Some(manager);
                        log::info!("Notification system initialized with default permissions");
                    }
                    Err(e) => {
                        log::error!("Failed to initialize notification manager: {}", e);
                    }
                }
            });

            // 离线会记 v0.5.0: Whisper 已彻底移除 (默认 SenseVoice-zh)
            // 保留 whisper_engine 模块代码作为参考, 但不启动, 不注册 invoke 命令
            // 如果未来要回滚, 取消下面两行注释即可

            // Set Parakeet models directory
            parakeet_engine::commands::set_models_directory(&_app.handle());

            // Initialize Parakeet engine on startup
            tauri::async_runtime::spawn(async {
                if let Err(e) = parakeet_engine::commands::parakeet_init().await {
                    log::error!("Failed to initialize Parakeet engine on startup: {}", e);
                }
            });
            // v0.7.0+ P0-2: 后台定时扫描 /tmp/lixianhuiji_diar/ 把后台计算的 diar segments
            // 兜底回填到 transcripts.speaker. 兜底 save_transcript 即时回填,
            // 覆盖 30-90 分钟长会议场景 (diar 后台线程耗时可能 > save_transcript).
            let app_for_pickup = _app.handle().clone();
            api::diar_pickup_loop::spawn_diar_pickup_loop(app_for_pickup);
            log::info!("Diar pickup background loop started (interval=30s)");

            // Initialize ModelManager for summary engine (async, non-blocking)
            let app_handle_for_model_manager = _app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match summary::summary_engine::commands::init_model_manager_at_startup(&app_handle_for_model_manager).await {
                    Ok(_) => log::info!("ModelManager initialized successfully at startup"),
                    Err(e) => {
                        log::warn!("Failed to initialize ModelManager at startup: {}", e);
                        log::warn!("ModelManager will be lazy-initialized on first use");
                    }
                }
            });

            // Trigger system audio permission request on startup (similar to microphone permission)
            // #[cfg(target_os = "macos")]
            // {
            //     tauri::async_runtime::spawn(async {
            //         if let Err(e) = audio::permissions::trigger_system_audio_permission() {
            //             log::warn!("Failed to trigger system audio permission: {}", e);
            //         }
            //     });
            // }

            // Initialize database (handles first launch detection and conditional setup)
            tauri::async_runtime::block_on(async {
                database::setup::initialize_database_on_startup(&_app.handle()).await
            })
            .expect("Failed to initialize database");

            // §99.2/§101 backfill: 必须在 database::setup 之后 spawn, AppState 已 manage 才能 try_state 成功
            // §99.5: 必须用 tauri::async_runtime::spawn, 不能用 tokio::spawn —
            //   Tauri main thread 是 tao event loop, 不是 Tokio runtime,
            //   tokio::spawn 会 panic: "there is no reactor running"
            let app_for_backfill = _app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = backfill_meeting_user_ids(&app_for_backfill).await {
                    log::warn!("§99.2 backfill_meeting_user_ids failed (best-effort, continue): {}", e);
                }
            });
            log::info!("§99.2 user_id backfill scheduled (post-AppState registration)");

            // §129 (2026-08-17): 清理陈旧 PENDING 行 (>30 min 没进展)
            //   之前: api_process_transcript 设 status='PENDING' → 进程被 kill 时永远卡 PENDING
            //   用户体验: "重新生成摘要报错" → 实际是后端早已死, 行残留 DB
            //   现在: 启动时一次性扫, >30 min 的 PENDING 标 failed + "Interrupted by app shutdown"
            //   阈值 30 min = MAX_POLLS 900 × 2s polling 间隔 + buffer
            let app_for_cleanup = _app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Some(app_state) = app_for_cleanup.try_state::<crate::state::AppState>() {
                    let pool = app_state.db_manager.pool();
                    match database::repositories::summary::SummaryProcessesRepository::cleanup_stale_pending_processes(pool, 30).await {
                        Ok(count) if count > 0 => log::info!("§129 startup cleanup: marked {} stale PENDING rows as failed", count),
                        Ok(_) => log::debug!("§129 startup cleanup: no stale PENDING rows"),
                        Err(e) => log::warn!("§129 startup cleanup failed (best-effort, continue): {}", e),
                    }
                } else {
                    log::warn!("§129 startup cleanup: AppState not available, skip");
                }
            });
            log::info!("§129 stale PENDING cleanup scheduled (threshold 30 min)");

            // §P2-B Topic dossier 夜间重建 scheduler (71 报告 P2-B)
            // 启动后 spawn 后台 task, 0-6 点 + 用户 idle + DB 有 stale topic 时跑.
            let app_for_scheduler = _app.handle().clone();
            tauri::async_runtime::spawn(async move {
                topic_graph::scheduler::start_topic_dossier_scheduler(app_for_scheduler).await;
            });
            log::info!("Topic dossier nightly scheduler started (idle-only)");

            // Initialize bundled templates directory for dynamic template discovery
            log::info!("Initializing bundled templates directory...");
            if let Ok(resource_path) = _app.handle().path().resource_dir() {
                let templates_dir = resource_path.join("templates");
                log::info!("Setting bundled templates directory to: {:?}", templates_dir);
                summary::templates::set_bundled_templates_dir(templates_dir);
            } else {
                log::warn!("Failed to resolve resource directory for templates");
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    if let Err(e) = window.hide() {
                        log::error!("Failed to hide main window on close request: {}", e);
                    } else {
                        log::info!("Main window hidden to tray on close request");
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            is_recording,
            get_transcription_status,
            read_audio_file,
            save_transcript,
            analytics::commands::init_analytics,
            analytics::commands::disable_analytics,
            analytics::commands::track_event,
            analytics::commands::identify_user,
            analytics::commands::track_meeting_started,
            analytics::commands::track_recording_started,
            analytics::commands::track_recording_stopped,
            analytics::commands::track_meeting_deleted,
            analytics::commands::track_settings_changed,
            analytics::commands::track_feature_used,
            analytics::commands::is_analytics_enabled,
            analytics::commands::start_analytics_session,
            analytics::commands::end_analytics_session,
            analytics::commands::track_daily_active_user,
            analytics::commands::track_user_first_launch,
            analytics::commands::is_analytics_session_active,
            analytics::commands::track_summary_generation_started,
            analytics::commands::track_summary_generation_completed,
            analytics::commands::track_summary_regenerated,
            analytics::commands::track_model_changed,
            analytics::commands::track_custom_prompt_used,
            analytics::commands::track_meeting_ended,
            analytics::commands::track_analytics_enabled,
            analytics::commands::track_analytics_disabled,
            analytics::commands::track_analytics_transparency_viewed,

            // v0.7.0+ P0-4: 硬件检测
            hardware::device_detect_profile,
            hardware::device_current_memory_mb,
            hardware::device_memory_pressure,
            // §31 P0 长音频内存自动降级
            hardware::memory_watcher::device_get_memory_recommendation,

            // §P0-A 跨会议知识图谱 (Phase 1 §78 + Phase 2 §79)
            topic_graph::api_topic_search,
            topic_graph::api_topic_recent,
            topic_graph::api_topic_extract_missing,
            topic_graph::api_topic_get_dossier,
            topic_graph::api_topic_rebuild_dossier,
            live_qa::api_meeting_live_qa,
            // §P0-B Obsidian vault 写入
            obsidian_export::api_obsidian_get_settings,
            obsidian_export::api_obsidian_set_settings,
            obsidian_export::api_obsidian_export_meeting,
            obsidian_export::api_obsidian_preview_markdown,
            // §P2-A 行动项可点击完成
            action_items::api_action_item_list,
            action_items::api_action_item_toggle,
            // §P1-B speaker alias (MVP)
            speaker_aliases::api_speaker_alias_list,
            speaker_aliases::api_speaker_alias_set,

            // 离线会记 v0.5.0: 用户/会员管理
            user::commands::user_register,
            user::commands::user_login,
            user::commands::user_bootstrap,
            user::commands::user_get_current,
            user::commands::user_logout,
            user::commands::system_machine_id,
            user::commands::user_activate_member,
            user::commands::hotwords_get,
            user::commands::hotwords_save,
            user::commands::hotwords_set_globals,
            user::commands::hotwords_list_packs,

            // v0.6.10+: 商业化
            user::commands::quota_get_status,
            user::commands::quota_increment_after_record,
            user::commands::lead_record_upgrade,
            user::commands::admin_activate_member,
            user::commands::admin_list_activation_orders,
            user::commands::admin_list_upgrade_leads,
            // C4: 激活码
            user::commands::admin_generate_activation_codes,
            user::commands::admin_list_activation_codes,
            user::commands::admin_revoke_activation_code,
            user::commands::user_redeem_activation_code,
            // v0.7.0+: 商业化运营 (退款 / 解绑 / 封号 / 配额)
            user::commands::admin_list_users,
            user::commands::admin_revoke_membership,
            user::commands::admin_unbind_machine,
            user::commands::admin_set_user_active,
            user::commands::admin_reset_user_quota,
            user::commands::admin_refund_user,
            // whisper 已停用 (见上注释)
            // whisper_engine::commands::whisper_init,
            // ... (11 commands, all disabled v0.5.0)
            // Parakeet engine commands
            parakeet_engine::commands::parakeet_init,
            parakeet_engine::commands::parakeet_get_available_models,
            parakeet_engine::commands::parakeet_load_model,
            parakeet_engine::commands::parakeet_get_current_model,
            parakeet_engine::commands::parakeet_is_model_loaded,
            parakeet_engine::commands::parakeet_has_available_models,
            parakeet_engine::commands::parakeet_validate_model_ready,
            parakeet_engine::commands::parakeet_transcribe_audio,
            parakeet_engine::commands::parakeet_get_models_directory,
            parakeet_engine::commands::parakeet_download_model,
            parakeet_engine::commands::parakeet_retry_download,
            parakeet_engine::commands::parakeet_cancel_download,
            parakeet_engine::commands::parakeet_delete_corrupted_model,
            parakeet_engine::commands::open_parakeet_models_folder,
            // Parallel processing commands
            whisper_engine::parallel_commands::initialize_parallel_processor,
            whisper_engine::parallel_commands::start_parallel_processing,
            whisper_engine::parallel_commands::pause_parallel_processing,
            whisper_engine::parallel_commands::resume_parallel_processing,
            whisper_engine::parallel_commands::stop_parallel_processing,
            whisper_engine::parallel_commands::get_parallel_processing_status,
            whisper_engine::parallel_commands::get_system_resources,
            whisper_engine::parallel_commands::check_resource_constraints,
            whisper_engine::parallel_commands::calculate_optimal_workers,
            whisper_engine::parallel_commands::prepare_audio_chunks,
            whisper_engine::parallel_commands::test_parallel_processing_setup,
            get_audio_devices,
            trigger_microphone_permission,
            start_recording_with_devices,
            start_recording_with_devices_and_meeting,
            start_audio_level_monitoring,
            stop_audio_level_monitoring,
            is_audio_level_monitoring,
            // Recording pause/resume commands
            audio::recording_commands::pause_recording,
            audio::recording_commands::resume_recording,
            audio::recording_commands::is_recording_paused,
            audio::recording_commands::get_recording_state,
            audio::recording_commands::get_meeting_folder_path,
            // Reload sync commands (retrieve transcript history and meeting name)
            audio::recording_commands::get_transcript_history,
            audio::recording_commands::get_recording_meeting_name,
            // Device monitoring commands (AirPods/Bluetooth disconnect/reconnect)
            audio::recording_commands::poll_audio_device_events,
            audio::recording_commands::get_reconnection_status,
            audio::recording_commands::attempt_device_reconnect,
            // Playback device detection (Bluetooth warning)
            audio::recording_commands::get_active_audio_output,
            // Audio recovery commands (for transcript recovery feature)
            audio::incremental_saver::recover_audio_from_checkpoints,
            audio::incremental_saver::cleanup_checkpoints,
            audio::incremental_saver::has_audio_checkpoints,
            console_utils::show_console,
            console_utils::hide_console,
            console_utils::toggle_console,
            ollama::get_ollama_models,
            ollama::pull_ollama_model,
            ollama::delete_ollama_model,
            ollama::get_ollama_model_context,
            openai::openai::get_openai_models,
            anthropic::anthropic::get_anthropic_models,
            groq::groq::get_groq_models,
            api::api_get_meetings,
            api::api_search_transcripts,
            api::api_get_profile,
            api::api_save_profile,
            api::api_update_profile,
            api::api_get_model_config,
            api::api_save_model_config,
            api::api_get_api_key,
            // api::api_get_auto_generate_setting,
            // api::api_save_auto_generate_setting,
            api::api_get_transcript_config,
            api::api_save_transcript_config,
            api::api_get_transcript_api_key,
            api::api_delete_meeting,
            api::api_get_meeting,
            api::api_get_meeting_metadata,
            api::api_get_meeting_transcripts,
            api::api_save_meeting_title,
            api::api_save_transcript,
            api::open_meeting_folder,
            api::test_backend_connection,
            api::debug_backend_connection,
            api::open_external_url,
            // Custom OpenAI commands
            api::api_save_custom_openai_config,
            api::api_get_custom_openai_config,
            api::api_test_custom_openai_connection,
            // Summary commands
            summary::commands::api_process_transcript,
            summary::commands::api_get_summary,
            summary::commands::api_save_meeting_summary,
            summary::commands::api_get_meeting_summary_language,
            summary::commands::api_save_meeting_summary_language,
            summary::commands::api_get_meeting_detected_summary_language,
            summary::commands::api_save_meeting_detected_summary_language,
            summary::commands::api_detect_transcript_summary_language,
            summary::commands::api_cancel_summary,
            // Template commands
            summary::template_commands::api_list_templates,
            summary::template_commands::api_get_template_details,
            summary::template_commands::api_validate_template,
            // Built-in AI commands
            summary::summary_engine::commands::builtin_ai_list_models,
            summary::summary_engine::commands::builtin_ai_get_model_info,
            summary::summary_engine::commands::builtin_ai_download_model,
            summary::summary_engine::commands::builtin_ai_cancel_download,
            summary::summary_engine::commands::builtin_ai_delete_model,
            summary::summary_engine::commands::builtin_ai_is_model_ready,
            summary::summary_engine::commands::builtin_ai_get_available_summary_model,
            summary::summary_engine::commands::builtin_ai_get_recommended_model,
            openrouter::get_openrouter_models,
            audio::recording_preferences::get_recording_preferences,
            audio::recording_preferences::set_recording_preferences,
            audio::recording_preferences::get_default_recordings_folder_path,
            audio::recording_preferences::open_recordings_folder,
            audio::recording_preferences::select_recording_folder,
            audio::recording_preferences::get_available_audio_backends,
            audio::recording_preferences::get_current_audio_backend,
            audio::recording_preferences::set_audio_backend,
            audio::recording_preferences::get_audio_backend_info,
            // Language preference commands
            set_language_preference,
            // Notification system commands
            notifications::commands::get_notification_settings,
            notifications::commands::set_notification_settings,
            notifications::commands::request_notification_permission,
            notifications::commands::show_notification,
            notifications::commands::show_test_notification,
            notifications::commands::is_dnd_active,
            notifications::commands::get_system_dnd_status,
            notifications::commands::set_manual_dnd,
            notifications::commands::set_notification_consent,
            notifications::commands::clear_notifications,
            notifications::commands::is_notification_system_ready,
            notifications::commands::initialize_notification_manager_manual,
            notifications::commands::test_notification_with_auto_consent,
            notifications::commands::get_notification_stats,
            // System audio capture commands
            audio::system_audio_commands::start_system_audio_capture_command,
            audio::system_audio_commands::list_system_audio_devices_command,
            audio::system_audio_commands::check_system_audio_permissions_command,
            audio::system_audio_commands::start_system_audio_monitoring,
            audio::system_audio_commands::stop_system_audio_monitoring,
            audio::system_audio_commands::get_system_audio_monitoring_status,
            // Screen Recording permission commands
            audio::permissions::check_screen_recording_permission_command,
            audio::permissions::request_screen_recording_permission_command,
            audio::permissions::trigger_system_audio_permission_command,
            // Database import commands
            database::commands::check_first_launch,
            database::commands::select_legacy_database_path,
            database::commands::detect_legacy_database,
            database::commands::check_default_legacy_database,
            database::commands::check_homebrew_database,
            database::commands::import_and_initialize_database,
            database::commands::initialize_fresh_database,
            // Database and Models path commands
            database::commands::get_database_directory,
            database::commands::open_database_folder,
            whisper_engine::commands::open_models_folder,
            // Onboarding commands
            onboarding::get_onboarding_status,
            onboarding::save_onboarding_status_cmd,
            onboarding::reset_onboarding_status_cmd,
            onboarding::complete_onboarding,
            // System settings commands
            #[cfg(target_os = "macos")]
            utils::open_system_settings,
            // Retranscription commands
            audio::retranscription::start_retranscription_command,
            audio::retranscription::cancel_retranscription_command,
            audio::retranscription::is_retranscription_in_progress_command,
            // §94 fix: 之前 invoke_handler 未注册, 前端 transcriptService.ts:102 调会失败
            // #[tauri::command] macro 注入的 __cmd__ 在 worker module, 需用 worker 路径
            audio::transcription::worker::get_streaming_timing_stats,
            // v0.6.11: streaming pipeline (实时流式识别)
            audio::sherpa_daemon::sherpa_stream_begin,
            audio::sherpa_daemon::sherpa_stream_chunk,
            audio::sherpa_daemon::sherpa_stream_finalize,
            // Import audio commands
            audio::import::select_and_validate_audio_command,
            audio::import::validate_audio_file_command,
            audio::import::start_import_audio_command,
            audio::import::cancel_import_command,
            audio::import::is_import_in_progress_command,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            match event {
                #[cfg(target_os = "macos")]
                tauri::RunEvent::Reopen { .. } => {
                    tray::focus_main_window(_app_handle);
                }
                tauri::RunEvent::Exit => {
                    log::info!("Application exiting, cleaning up resources...");
                    tauri::async_runtime::block_on(async {
                        // Clean up database connection and checkpoint WAL
                        if let Some(app_state) = _app_handle.try_state::<state::AppState>() {
                            log::info!("Starting database cleanup...");
                            if let Err(e) = app_state.db_manager.cleanup().await {
                                log::error!("Failed to cleanup database: {}", e);
                            } else {
                                log::info!("Database cleanup completed successfully");
                            }
                        } else {
                            log::warn!("AppState not available for database cleanup (likely first launch)");
                        }

                        // v0.8.5 §23: Release sherpa daemon (kill Python child + onnx models)
                        log::info!("Cleaning up sherpa daemon...");
                        crate::audio::sherpa_daemon::shutdown_global_daemon();

                        // Clean up sidecar
                        log::info!("Cleaning up sidecar...");
                        if let Err(e) = summary::summary_engine::force_shutdown_sidecar().await {
                            log::error!("Failed to force shutdown sidecar: {}", e);
                        }
                    });
                    log::info!("Application cleanup complete");
                }
                _ => {}
            }
        });
}

/// v0.7.0+: 安装 panic hook.
/// 任何 Rust 线程 panic 时, 把 backtrace 写到本地 crash log 文件, 用户可查.
/// 完全本地, 0 网络, 0 第三方依赖.
fn install_panic_hook() {
    use std::panic;
    use std::fs;
    use std::io::Write;

    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // 1) 调默认 hook (打到 stderr / log)
        default_hook(panic_info);

        // 2) 写本地 crash log
        let app_data = std::env::var("LIXIANHUIJI_DATA_DIR").ok()
            .or_else(|| dirs::data_dir().map(|p| p.join(crate::config::APP_BUNDLE_ID).to_string_lossy().to_string()));
        if let Some(dir) = app_data {
            let crash_dir = std::path::PathBuf::from(&dir).join("crashes");
            let _ = fs::create_dir_all(&crash_dir);
            let now = chrono::Utc::now();
            let filename = format!("crash-{}.txt", now.format("%Y%m%d-%H%M%S"));
            let path = crash_dir.join(&filename);
            if let Ok(mut f) = fs::File::create(&path) {
                let _ = writeln!(f, "=== LixianHuiji Panic Report ===");
                let _ = writeln!(f, "timestamp: {}", now.to_rfc3339());
                let _ = writeln!(f, "version: {}", env!("CARGO_PKG_VERSION"));
                let _ = writeln!(f, "os: {} / arch: {}", std::env::consts::OS, std::env::consts::ARCH);
                let _ = writeln!(f, "
--- panic_info ---");
                let _ = writeln!(f, "{}", panic_info);
                let _ = writeln!(f, "
--- backtrace ---");
                let _ = writeln!(f, "{}", std::backtrace::Backtrace::force_capture());
            }
            // 3) 仅保留最近 50 个 crash 文件
            if let Ok(rd) = fs::read_dir(&crash_dir) {
                let mut entries: Vec<_> = rd.flatten()
                    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("txt"))
                    .collect();
                entries.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
                for old in entries.iter().rev().skip(50) {
                    let _ = fs::remove_file(old.path());
                }
            }
        }
    }));
}
