// §P2-B Topic Dossier 夜间重建 scheduler (2026-08-07)
// 71 报告 P2-B: "每天 0-6 点本地无操作时, 跑未处理的 topic dossier 增量更新".
// 设计: tokio interval polling, idle detection, sequential rebuild with cap.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Local, Timelike, Utc};
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager, Runtime};
use tokio::sync::RwLock;

/// 默认 polling interval = 30 min.
pub const DEFAULT_POLL_INTERVAL_SECS: u64 = 1800;
/// 同一 topic dossier 重建最小间隔 = 12 h.
pub const DEFAULT_REBUILD_COOLDOWN_HOURS: i64 = 12;
/// 单次 pass 最多重建 topic 数.
pub const DEFAULT_MAX_PER_NIGHT: i64 = 3;
/// 用户视为 "idle" 的阈值 (最后录音/摘要 距今 分钟数).
pub const DEFAULT_IDLE_MINUTES: i64 = 30;
/// 默认夜间窗口 (本地时区) = 0:00 - 6:00.
pub const DEFAULT_NIGHT_WINDOW_START_HOUR: u32 = 0;
pub const DEFAULT_NIGHT_WINDOW_END_HOUR: u32 = 6;

static SCHEDULER_RUNNING: AtomicBool = AtomicBool::new(false);

/// Scheduler 状态 (用于测试与前端观测).
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    pub started_at: Option<DateTime<Utc>>,
    pub last_pass_at: Option<DateTime<Utc>>,
    pub last_pass_rebuilt: i64,
    pub total_rebuilt: i64,
    pub total_skipped_busy: i64,
    pub total_skipped_window: i64,
}

static STATS: once_cell::sync::Lazy<Arc<RwLock<SchedulerStats>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(SchedulerStats::default())));

/// 启动后台 task. App 启动时调一次, 重复调 idempotent.
pub async fn start_topic_dossier_scheduler<R: Runtime>(app: AppHandle<R>) {
    if SCHEDULER_RUNNING.swap(true, Ordering::SeqCst) {
        log::info!("[topic_graph.scheduler] already running, skip");
        return;
    }
    {
        let mut stats = STATS.write().await;
        stats.started_at = Some(Utc::now());
    }

    let poll_secs = env_or("MEETILY_DOSSIER_POLL_SECS", DEFAULT_POLL_INTERVAL_SECS);
    let cooldown_hours = env_or_i64(
        "MEETILY_DOSSIER_COOLDOWN_HOURS",
        DEFAULT_REBUILD_COOLDOWN_HOURS,
    );
    let max_per_night = env_or_i64("MEETILY_DOSSIER_MAX_PER_NIGHT", DEFAULT_MAX_PER_NIGHT);
    let idle_minutes = env_or_i64("MEETILY_DOSSIER_IDLE_MIN", DEFAULT_IDLE_MINUTES);
    let window_start = env_or_u32(
        "MEETILY_DOSSIER_WINDOW_START",
        DEFAULT_NIGHT_WINDOW_START_HOUR,
    );
    let window_end = env_or_u32(
        "MEETILY_DOSSIER_WINDOW_END",
        DEFAULT_NIGHT_WINDOW_END_HOUR,
    );

    log::info!(
        "[topic_graph.scheduler] starting: poll={}s cooldown={}h max={} idle={}min window={}:00-{}:00",
        poll_secs, cooldown_hours, max_per_night, idle_minutes, window_start, window_end
    );

    let pool: SqlitePool = app
        .state::<crate::state::AppState>()
        .db_manager
        .pool()
        .clone();

    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(poll_secs));
        // 第一次 tick 立即跑 (启动后先尝试一次)
        ticker.tick().await;
        loop {
            run_one_pass(
                &app_clone,
                &pool,
                cooldown_hours,
                max_per_night,
                idle_minutes,
                window_start,
                window_end,
            )
            .await;
            ticker.tick().await;
        }
    });
}

/// 单次 pass. 测试也直接调这个 (跳过 ticker).
pub async fn run_one_pass<R: Runtime>(
    app: &AppHandle<R>,
    pool: &SqlitePool,
    cooldown_hours: i64,
    max_per_night: i64,
    idle_minutes: i64,
    window_start: u32,
    window_end: u32,
) {
    let now_utc = Utc::now();

    // 1) 窗口检查 (0-6 默认)
    let hour = Local::now().hour();
    if !in_window(hour, window_start, window_end) {
        let mut s = STATS.write().await;
        s.total_skipped_window += 1;
        log::debug!(
            "[topic_graph.scheduler] skip: hour={} outside window {}:00-{}:00",
            hour, window_start, window_end
        );
        return;
    }

    // 2) idle 检查
    if !is_user_idle(pool, idle_minutes).await {
        let mut s = STATS.write().await;
        s.total_skipped_busy += 1;
        log::debug!("[topic_graph.scheduler] skip: user busy");
        return;
    }

    // 3) 选 stale topics
    let cutoff = now_utc - chrono::Duration::hours(cooldown_hours);
    let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();
    let topics: Vec<(i64, String)> = match sqlx::query_as(
        "SELECT t.id, t.canonical_name FROM topic_node t
         LEFT JOIN topic_dossier d ON d.topic_id = t.id
         WHERE d.last_updated_at IS NULL OR d.last_updated_at < ?1
         ORDER BY d.last_updated_at ASC NULLS FIRST, t.last_touched_at DESC
         LIMIT ?2",
    )
    .bind(&cutoff_str)
    .bind(max_per_night)
    .fetch_all(pool)
    .await
    {
        Ok(v) => v,
        Err(e) => {
            log::warn!("[topic_graph.scheduler] query stale topics failed: {e}");
            return;
        }
    };

    if topics.is_empty() {
        log::debug!("[topic_graph.scheduler] no stale topics, idle");
        let mut s = STATS.write().await;
        s.last_pass_at = Some(now_utc);
        s.last_pass_rebuilt = 0;
        return;
    }

    log::info!(
        "[topic_graph.scheduler] pass: {} stale topics (cap={})",
        topics.len(),
        max_per_night
    );

    let mut rebuilt = 0;
    for (tid, name) in topics {
        log::info!("[topic_graph.scheduler] rebuild {} (id={})", name, tid);
        // §137.5: 用 settings 表里用户选的 provider + model (不再硬编码 qwen3.5:2b)
        let (provider, model_name) = match crate::database::repositories::setting::SettingsRepository::get_model_config(&pool).await {
            Ok(Some(setting)) => (setting.provider, setting.model),
            _ => ("ollama".to_string(), "llama3.2:latest".to_string()),  // 兜底默认
        };
        let llm_provider = crate::summary::llm_client::LLMProvider::from_str(&provider)
            .unwrap_or(crate::summary::llm_client::LLMProvider::Ollama);
        match super::rebuild_topic_dossier(app.clone(), pool.clone(), tid, llm_provider, &model_name).await {
            Ok(()) => rebuilt += 1,
            Err(e) => log::warn!("[topic_graph.scheduler] {} failed: {}", tid, e),
        }
        // LLM call 之间留 2s 让 UI/CPU 喘息
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    {
        let mut s = STATS.write().await;
        s.last_pass_at = Some(now_utc);
        s.last_pass_rebuilt = rebuilt;
        s.total_rebuilt += rebuilt;
    }
    log::info!("[topic_graph.scheduler] pass done: rebuilt={}", rebuilt);
}

fn in_window(hour: u32, start: u32, end: u32) -> bool {
    // 支持跨午夜 (start=22, end=6)
    if start <= end {
        hour >= start && hour < end
    } else {
        hour >= start || hour < end
    }
}

async fn is_user_idle(pool: &SqlitePool, idle_minutes: i64) -> bool {
    let cutoff = Utc::now() - chrono::Duration::minutes(idle_minutes);
    let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();
    // 查最近录音 + 最近 summary_processes 任一有活动 = busy
    let rec_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM recordings WHERE created_at > ?1",
    )
    .bind(&cutoff_str)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if rec_count > 0 {
        return false;
    }
    let sum_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM summary_processes WHERE updated_at > ?1",
    )
    .bind(&cutoff_str)
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    sum_count == 0
}

pub async fn get_stats() -> SchedulerStats {
    STATS.read().await.clone()
}

fn env_or(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
fn env_or_i64(key: &str, default: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
fn env_or_u32(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_window_normal() {
        assert!(in_window(0, 0, 6));
        assert!(in_window(3, 0, 6));
        assert!(in_window(5, 0, 6));
        assert!(!in_window(6, 0, 6));
        assert!(!in_window(12, 0, 6));
        assert!(!in_window(23, 0, 6));
    }

    #[test]
    fn test_in_window_cross_midnight() {
        // 22:00 - 06:00
        assert!(in_window(22, 22, 6));
        assert!(in_window(0, 22, 6));
        assert!(in_window(5, 22, 6));
        assert!(!in_window(6, 22, 6));
        assert!(!in_window(12, 22, 6));
        assert!(!in_window(21, 22, 6));
    }

    #[test]
    fn test_in_window_single_hour() {
        assert!(in_window(3, 3, 4));
        assert!(!in_window(4, 3, 4));
        assert!(!in_window(2, 3, 4));
    }

    #[test]
    fn test_env_or_default() {
        // 不设环境变量, 走 default
        assert_eq!(env_or("MEETILY_NONEXISTENT_KEY_XYZ", 42), 42);
    }

    #[test]
    fn test_env_or_override() {
        std::env::set_var("MEETILY_TEST_KEY_OVERRIDE", "123");
        assert_eq!(env_or("MEETILY_TEST_KEY_OVERRIDE", 42), 123);
        std::env::remove_var("MEETILY_TEST_KEY_OVERRIDE");
    }

    #[tokio::test]
    async fn test_stats_initial_zero() {
        // STATS 是全局 static, 第一次访问是 default
        let s = get_stats().await;
        // 不强断言 started_at 是 None (可能被其它测试初始化过)
        // 但 total_rebuilt 应该是 0
        assert_eq!(s.total_rebuilt, 0);
    }
}
