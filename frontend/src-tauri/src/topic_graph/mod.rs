// §P0-A 跨会议知识图谱 (Phase 1: schema + DB CRUD + Tauri command + LLM extract 骨架)
//
// 4 张表 (see migration 20260806000001):
//   topic_node              — 持久化 topic 实体 (canonical_name 去重)
//   meeting_episode_node    — 单场会议对某 topic 的提及 (excerpt + sentiment)
//   relates_to              — topic 之间的关系 (related / causes / supersedes / ...)
//   topic_dossier           — topic 的累积状态 (status / summary / open_questions)
//
// Phase 1 范围:
//   - 纯函数 LLM extract 骨架 (extract.rs) — 不实际调 LLM, 仅 prompt + JSON 解析
//   - DB CRUD: upsert_topic, link_meeting, search_topics, get_topic_dossier
//   - Tauri command: api_topic_search(query, limit) 返 [{name, mention_count, last_decided, sample_excerpts}]
//
// Phase 2 后续 (估时 5-10 天):
//   - service.rs 摘要完成时 spawn extract_and_link
//   - LLM 调 BuiltInAI (Qwen 3.5 2B) 实际提取
//   - topic_dossier 夜间增量 rebuild
//   - 会议开始时弹"⏮ 7/15 讨论过 Topic X, 当时状态 Y"

pub mod extract;
pub mod scheduler;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::{Emitter, Manager, Runtime, State};

use crate::state::AppState;
use crate::summary::llm_client::{generate_summary, LLMProvider};

pub use extract::{ExtractedTopic, ExtractPromptBuilder, parse_extract_response};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicNode {
    pub id: i64,
    pub canonical_name: String,
    pub topic_type: String,
    pub first_seen_at: String,
    pub last_touched_at: String,
    pub mention_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingEpisode {
    pub id: i64,
    pub topic_id: i64,
    pub meeting_id: String,
    pub excerpt: Option<String>,
    pub sentiment: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicSearchHit {
    pub topic_id: i64,
    pub canonical_name: String,
    pub topic_type: String,
    pub mention_count: i64,
    pub last_touched_at: String,
    pub last_decided: Option<String>,
    pub status: Option<String>,
    pub sample_excerpts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicDossier {
    pub topic_id: i64,
    pub canonical_name: String,
    pub status: String,
    pub summary: Option<String>,
    pub open_questions: Option<String>,
    pub last_decided: Option<String>,
    pub last_updated_at: String,
    pub rebuild_count: i64,
    pub episodes: Vec<MeetingEpisode>,
}

// ============== DB CRUD ==============

/// 规范化 topic 名: lowercase + 去除多余空白 + 去除常见标点, 用于 dedupe.
/// 例: "API 限流" -> "api 限流", "API 限流!" -> "api 限流"
pub fn normalize_topic_name(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || (c >= '\u{4e00}' && c <= '\u{9fff}') || c.is_whitespace() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// upsert 一条 topic (canonical_name 唯一), 返 (id, was_created)
pub async fn upsert_topic(pool: &SqlitePool, canonical_name: &str, topic_type: &str) -> Result<(i64, bool), String> {
    let now = Utc::now().to_rfc3339();
    let normalized = normalize_topic_name(canonical_name);
    if normalized.is_empty() {
        return Err("empty topic name".to_string());
    }
    // Try insert
    let inserted = sqlx::query(
        "INSERT OR IGNORE INTO topic_node (canonical_name, topic_type, first_seen_at, last_touched_at, mention_count) \
         VALUES (?1, ?2, ?3, ?3, 1)",
    )
    .bind(&normalized)
    .bind(topic_type)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| format!("upsert_topic insert: {e}"))?;
    if inserted.rows_affected() == 1 {
        let id: i64 = sqlx::query_scalar("SELECT id FROM topic_node WHERE canonical_name = ?1")
            .bind(&normalized)
            .fetch_one(pool)
            .await
            .map_err(|e| format!("upsert_topic fetch id: {e}"))?;
        return Ok((id, true));
    }
    // Conflict -> bump mention_count + last_touched_at
    sqlx::query(
        "UPDATE topic_node SET mention_count = mention_count + 1, last_touched_at = ?1 WHERE canonical_name = ?2",
    )
    .bind(&now)
    .bind(&normalized)
    .execute(pool)
    .await
    .map_err(|e| format!("upsert_topic update: {e}"))?;
    let id: i64 = sqlx::query_scalar("SELECT id FROM topic_node WHERE canonical_name = ?1")
        .bind(&normalized)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("upsert_topic fetch id 2: {e}"))?;
    Ok((id, false))
}

/// 链接 meeting 到 topic (幂等, UNIQUE(topic_id, meeting_id))
pub async fn link_meeting(
    pool: &SqlitePool,
    topic_id: i64,
    meeting_id: &str,
    excerpt: Option<&str>,
    sentiment: &str,
) -> Result<(), String> {
    sqlx::query(
        "INSERT OR IGNORE INTO meeting_episode_node (topic_id, meeting_id, excerpt, sentiment) \
         VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(topic_id)
    .bind(meeting_id)
    .bind(excerpt)
    .bind(sentiment)
    .execute(pool)
    .await
    .map_err(|e| format!("link_meeting: {e}"))?;
    Ok(())
}

/// 搜索 topic (LIKE 模糊匹配), 返 mention_count 排序, 带 sample_excerpts (前 3 段)
pub async fn search_topics(
    pool: &SqlitePool,
    query: &str,
    limit: usize,
) -> Result<Vec<TopicSearchHit>, String> {
    let like = format!("%{}%", normalize_topic_name(query));
    let limit = limit.min(50) as i64;
    let topics: Vec<(i64, String, String, i64, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT t.id, t.canonical_name, t.topic_type, t.mention_count, t.last_touched_at, \
                d.last_decided, d.status \
         FROM topic_node t LEFT JOIN topic_dossier d ON d.topic_id = t.id \
         WHERE t.canonical_name LIKE ?1 \
         ORDER BY t.mention_count DESC, t.last_touched_at DESC LIMIT ?2",
    )
    .bind(&like)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("search_topics query: {e}"))?;
    let mut hits = Vec::new();
    for (id, name, ty, count, last_touch, decided, status) in topics {
        let excerpts: Vec<Option<String>> = sqlx::query_scalar(
            "SELECT excerpt FROM meeting_episode_node WHERE topic_id = ?1 \
             AND excerpt IS NOT NULL ORDER BY created_at DESC LIMIT 3",
        )
        .bind(id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("search_topics excerpts: {e}"))?;
        let sample: Vec<String> = excerpts.into_iter().flatten().collect();
        hits.push(TopicSearchHit {
            topic_id: id,
            canonical_name: name,
            topic_type: ty,
            mention_count: count,
            last_touched_at: last_touch,
            last_decided: decided,
            status,
            sample_excerpts: sample,
        });
    }
    Ok(hits)
}

pub async fn get_topic_dossier(pool: &SqlitePool, topic_id: i64) -> Result<Option<TopicDossier>, String> {
    let topic: Option<(i64, String)> = sqlx::query_as("SELECT id, canonical_name FROM topic_node WHERE id = ?1")
        .bind(topic_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("get_topic_dossier topic: {e}"))?;
    let (id, name) = match topic {
        Some(t) => t,
        None => return Ok(None),
    };
    let dossier: Option<(String, Option<String>, Option<String>, Option<String>, String, i64)> = sqlx::query_as(
        "SELECT status, summary, open_questions, last_decided, last_updated_at, rebuild_count \
         FROM topic_dossier WHERE topic_id = ?1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("get_topic_dossier dossier: {e}"))?;
    let (status, summary, questions, decided, updated, rebuild) = dossier
        .unwrap_or_else(|| ("open".to_string(), None, None, None, Utc::now().to_rfc3339(), 0));
    let episodes: Vec<(i64, i64, String, Option<String>, String, String)> = sqlx::query_as(
        "SELECT id, topic_id, meeting_id, excerpt, sentiment, created_at \
         FROM meeting_episode_node WHERE topic_id = ?1 ORDER BY created_at DESC",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("get_topic_dossier episodes: {e}"))?;
    let episodes = episodes
        .into_iter()
        .map(|(eid, tid, mid, exc, sent, ts)| MeetingEpisode {
            id: eid, topic_id: tid, meeting_id: mid, excerpt: exc, sentiment: sent, created_at: ts,
        })
        .collect();
    Ok(Some(TopicDossier {
        topic_id: id,
        canonical_name: name,
        status,
        summary,
        open_questions: questions,
        last_decided: decided,
        last_updated_at: updated,
        rebuild_count: rebuild,
        episodes,
    }))
}

// ============== Tauri commands ==============

// ============== 近期活跃 topic（last_touched_at DESC）==============
/// 跨会议知识图谱里, 找最近被 mention 的 topic.
/// 用于 Sidebar / Header "Topic Search" 面板的初始显示 + "近期相关" 提示.
pub async fn recent_topics(pool: &SqlitePool, limit: i64) -> Result<Vec<TopicSearchHit>, String> {
    let rows: Vec<(i64, String, String, i64, String)> = sqlx::query_as(
        "SELECT id, canonical_name, topic_type, mention_count, last_touched_at          FROM topic_node ORDER BY last_touched_at DESC LIMIT ?1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("recent_topics: {e}"))?;

    let mut hits = Vec::with_capacity(rows.len());
    for (id, name, ty, count, last_touch) in rows {
        // 兼容 schema: meeting_episode_node 与 topic_dossier 都可能有 last_decided / status
        let decided: Option<String> = sqlx::query_scalar(
            "SELECT last_decided FROM topic_dossier WHERE topic_id = ?1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
        let status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM topic_dossier WHERE topic_id = ?1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
        let excerpts: Vec<String> = sqlx::query_as::<_, (String,)>(
            "SELECT excerpt FROM meeting_episode_node WHERE topic_id = ?1              AND excerpt IS NOT NULL ORDER BY created_at DESC LIMIT 3",
        )
        .bind(id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("recent_topics excerpts: {e}"))?
        .into_iter()
        .map(|(s,)| s)
        .collect();
        hits.push(TopicSearchHit {
            topic_id: id,
            canonical_name: name,
            topic_type: ty,
            mention_count: count,
            last_touched_at: last_touch,
            last_decided: decided,
            status,
            sample_excerpts: excerpts,
        });
    }
    Ok(hits)
}



#[tauri::command]
pub async fn api_topic_recent<R: Runtime>(
    app: tauri::AppHandle<R>,
    limit: Option<i64>,
) -> Result<Vec<TopicSearchHit>, String> {
    let state: State<'_, AppState> = app.state();
    let pool = state.db_manager.pool();
    recent_topics(pool, limit.unwrap_or(8)).await
}

#[tauri::command]
pub async fn api_topic_search<R: Runtime>(
    app: tauri::AppHandle<R>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<TopicSearchHit>, String> {
    let state: State<'_, AppState> = app.state();
    let pool = state.db_manager.pool();
    search_topics(pool, &query, limit.unwrap_or(20)).await
}

#[tauri::command]
pub async fn api_topic_rebuild_dossier<R: Runtime>(
    app: tauri::AppHandle<R>,
    topic_id: i64,
) -> Result<(), String> {
    let pool = {
        let state: State<'_, AppState> = app.state();
        state.db_manager.pool().clone()
    };
    rebuild_topic_dossier(app, pool, topic_id).await
}

#[tauri::command]
pub async fn api_topic_get_dossier<R: Runtime>(
    app: tauri::AppHandle<R>,
    topic_id: i64,
) -> Result<Option<TopicDossier>, String> {
    let state: State<'_, AppState> = app.state();
    let pool = state.db_manager.pool();
    get_topic_dossier(pool, topic_id).await
}


// ============== Phase 2: spawn hook (LLM extract + DB link) ==============

/// 摘要完成时由 summary/service.rs spawn 调用.
/// 实际调 BuiltInAI (Qwen 3.5 2B) 提取 topic, upsert 进 topic_node + link meeting_episode_node.
/// 失败 / 用户没启用 LLM / 模型尚未下载 都 swallow log, 永不 panic.
pub async fn trigger_after_summary<R: Runtime>(
    #[allow(unused_variables)] app: tauri::AppHandle<R>,
    pool: SqlitePool,
    meeting_id: String,
    summary_markdown: String,
) {
    log::info!("[topic_graph] spawn for meeting={} (summary={} chars)", meeting_id, summary_markdown.len());

    // 1) Dedup: 已经 link 过 episode 的 meeting 不再提取.
    let already: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM meeting_episode_node WHERE meeting_id = ?1 LIMIT 1",
    )
    .bind(&meeting_id)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();
    if already.is_some() {
        log::info!("[topic_graph] {meeting_id} already has episodes, skip extract");
        return;
    }

    // 2) 摘要太短 (< 50 字) 不提取, 提示用户开会时间太短.
    if summary_markdown.trim().len() < 50 {
        log::info!("[topic_graph] {meeting_id} summary too short, skip");
        return;
    }

    // 3) Build prompt.
    let prompt = ExtractPromptBuilder::build(&summary_markdown);

    // 4) Call BuiltInAI (best-effort). 失败仅 log warn, 不影响主流程.
    // §132: 120s 太长 — Ollama 不可用每场等 120s, 9 场 = 18 分钟一直转. 改 30s.
    //       Ollama connect refuse 通常 3s, qwen3.5:2b 推理 800 token ≤ 25s.
    // §111: §132 推理时间低估 — qwen3.5:2b thinking mode 默认开 (实测 40s/800 token 空 content).
    //       已加 think:false (§111 llm_client.rs), 实测 1.8s. 但冷启动 + 大摘要仍可能 30s+.
    //       给到 90s 缓冲, 仍 fail 就走 topic-extract-failed emit, 不阻塞主流程.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    // §137.3: 优先 BuiltInAI (本机已装 Qwen3.5-2B-Q4_K_M.gguf), fallback Ollama
    let app_data_dir = app.path().app_data_dir().ok();
    let use_builtin_ai = app_data_dir
        .as_ref()
        .map(|d| builtin_ai_model_exists(d))
        .unwrap_or(false);
    let (provider, model_name) = if use_builtin_ai {
        log::info!("[topic_graph] {meeting_id} using BuiltInAI (Qwen3.5 2B, local sidecar)");
        (LLMProvider::BuiltInAI, "qwen3.5:2b")
    } else {
        log::info!("[topic_graph] {meeting_id} using Ollama (localhost:11434, qwen3.5:2b)");
        (LLMProvider::Ollama, "qwen3.5:2b")
    };
    let response = match generate_summary(
        &client,
        &provider,
        model_name,
        "",          // api_key unused for local providers
        "",          // system_prompt (instructions already in user prompt)
        &prompt,
        None,        // ollama_endpoint
        None,        // custom_openai_endpoint
        Some(800),   // max_tokens (per AGENTS.md §52)
        None,        // temperature
        None,        // top_p
        app_data_dir.as_ref(), // app_data_dir (BuiltInAI 需要)
        None,        // cancellation_token
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            // §121 铁律 #3: 失败升级 error + emit Tauri 事件, 不再 swallow
            log::error!("[topic_graph] {meeting_id} llm extract failed: {e}");
            let _ = app.emit(
                "topic-extract-failed",
                serde_json::json!({
                    "meeting_id": meeting_id,
                    "error": e.to_string(),
                    "at": chrono::Utc::now().to_rfc3339(),
                }),
            );
            return;
        }
    };

    // 5) Parse response.
    let topics = parse_extract_response(&response);
    if topics.is_empty() {
        log::info!("[topic_graph] {meeting_id} parsed 0 topics");
        return;
    }
    log::info!("[topic_graph] {meeting_id} parsed {} topics", topics.len());

    // 6) Upsert topic_node + link meeting_episode_node (cap 8 per meeting).
    let mut linked = 0usize;
    for t in topics.into_iter().take(8) {
        let topic_type = if matches!(t.topic_type.as_str(), "general" | "project" | "person" | "decision") {
            t.topic_type
        } else {
            "general".to_string()
        };
        let sentiment = if matches!(t.sentiment.as_str(), "positive" | "negative" | "neutral") {
            t.sentiment
        } else {
            "neutral".to_string()
        };
        match upsert_topic(&pool, &t.canonical_name, &topic_type).await {
            Ok((tid, _created)) => {
                if let Err(e) = link_meeting(&pool, tid, &meeting_id, Some(&t.excerpt), &sentiment).await {
                    log::warn!("[topic_graph] link_meeting failed for tid={tid}: {e}");
                } else {
                    linked += 1;
                }
            }
            Err(e) => log::warn!("[topic_graph] upsert_topic failed for '{}': {e}", t.canonical_name),
        }
    }
    log::info!("[topic_graph] {meeting_id} linked {linked} episodes to topic_node");
}


/// §137.3: helper — 检查 app_data_dir/models/summary/ 是否有 .gguf 文件.
/// BuiltInAI 路径判定 — 有就 OK, 没有就 fallback Ollama.
fn builtin_ai_model_exists(app_data_dir: &std::path::Path) -> bool {
    let models_dir = app_data_dir.join("models").join("summary");
    if !models_dir.is_dir() {
        return false;
    }
    match std::fs::read_dir(&models_dir) {
        Ok(entries) => entries.filter_map(|e| e.ok()).any(|e| {
            e.path().extension().and_then(|s| s.to_str()) == Some("gguf")
        }),
        Err(_) => false,
    }
}

/// §137.3: preflight LLM 检查 — 优先 BuiltInAI (本机内置, 零下载), fallback Ollama.
/// BuiltInAI: 检查 `app_data_dir/models/summary/` 是否有 .gguf 文件.
/// Ollama: 3s timeout ping localhost:11434/api/tags.
/// Returns Ok(provider_name) 任一就绪, Err(reason) 都不可用.
pub async fn preflight_llm_async<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<&'static str, String> {
    // 1) BuiltInAI 路径: 检查 app_data_dir/models/summary/*.gguf
    if let Some(app_data_dir) = app.path().app_data_dir().ok() {
        let models_dir = app_data_dir.join("models").join("summary");
        if models_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&models_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("gguf") {
                        log::info!("[preflight] BuiltInAI model found: {}", path.display());
                        return Ok("builtin_ai");
                    }
                }
            }
        }
    }
    // 2) Ollama fallback
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let url = "http://127.0.0.1:11434/api/tags";
    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => {
            log::info!("[preflight] Ollama available at localhost:11434");
            Ok("ollama")
        }
        Ok(resp) => Err(format!("ollama http {}", resp.status())),
        Err(e) => Err(format!("ollama unreachable: {e}")),
    }
}

/// §126: 手动批量补提 — 找所有 status='completed' 但无 meeting_episode_node 的会议,
/// 逐个调 trigger_after_summary 补 topic extract. 用于 history recovery (LLM 第一次失败 /
/// Ollama 未启动 / 模型未下载 / 其他 silent fail 后 retry). 串行执行避免 Ollama 连接抖动.
///
/// Returns (processed_count, linked_count) — 调用方可用 processed>0 判断是否还有继续补的.
pub async fn extract_missing_topics<R: Runtime>(
    app: tauri::AppHandle<R>,
    pool: &SqlitePool,
    max_meetings: i64,
) -> Result<(usize, usize), String> {
    // 1) 找所有 completed summary 但还没 episode 的 meeting
    // §132: preflight — Ollama 没启就立刻 return (-1, 0), 前端不再转 18 分钟.
    if let Err(reason) = preflight_llm_async(&app).await {
        log::warn!("[topic_graph] §132 preflight failed: {reason}, skip history recovery");
        let _ = app.emit(
            "topic-recover-skipped",
            serde_json::json!({
                "reason": reason,
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );
        return Ok((0, 0));
    }

    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT sp.meeting_id, sp.result
         FROM summary_processes sp
         LEFT JOIN meeting_episode_node ep ON ep.meeting_id = sp.meeting_id
         WHERE sp.status = 'completed' AND sp.result IS NOT NULL AND ep.id IS NULL
         ORDER BY sp.updated_at DESC
         LIMIT ?1",
    )
    .bind(max_meetings)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("extract_missing_topics query: {e}"))?;

    log::info!("[topic_graph] §126 manual recover: found {} meetings to reprocess", rows.len());
    let total = rows.len();
    let _ = app.emit(
        "topic-recover-progress",
        serde_json::json!({
            "phase": "start",
            "total": total,
            "processed": 0,
            "at": chrono::Utc::now().to_rfc3339(),
        }),
    );
    let mut processed = 0usize;
    for (meeting_id, result_json) in rows {
        // 2) parse result.english_cache.markdown (or fallback .markdown)
        let markdown = match serde_json::from_str::<serde_json::Value>(&result_json) {
            Ok(v) => v
                .get("english_cache")
                .and_then(|c| c.get("markdown"))
                .and_then(|m| m.as_str())
                .or_else(|| v.get("markdown").and_then(|m| m.as_str()))
                .or_else(|| v.get("raw_summary").and_then(|m| m.as_str()))
                .map(|s| s.to_string())
                .unwrap_or_default(),
            Err(_) => String::new(),
        };
        if markdown.trim().len() < 50 {
            log::info!("[topic_graph] §126 skip {} (markdown < 50 chars)", meeting_id);
            continue;
        }
        log::info!("[topic_graph] §126 reprocessing {} ({} chars markdown)", meeting_id, markdown.len());
        let app_clone = app.clone();
        let mid = meeting_id.clone();
        let md = markdown.clone();
        // 串行: 直接 await trigger_after_summary (内部就 await Ollama). 不再 clone pool.
        trigger_after_summary(app_clone, pool.clone(), mid, md).await;
        processed += 1;
        let _ = app.emit(
            "topic-recover-progress",
            serde_json::json!({
                "phase": "step",
                "total": total,
                "processed": processed,
                "current_meeting": meeting_id,
                "at": chrono::Utc::now().to_rfc3339(),
            }),
        );
    }
    let _ = app.emit(
        "topic-recover-progress",
        serde_json::json!({
            "phase": "done",
            "total": total,
            "processed": processed,
            "at": chrono::Utc::now().to_rfc3339(),
        }),
    );
    // 3) linked count = upsert_topic 实际写了多少 — 简易: query topic_node count delta
    let total_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM topic_node")
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    log::info!("[topic_graph] §126 done: processed={} total_topics={}", processed, total_after);
    Ok((processed, total_after as usize))
}


#[tauri::command]
pub async fn api_topic_extract_missing<R: Runtime>(
    app: tauri::AppHandle<R>,
    max_meetings: Option<i64>,
) -> Result<(usize, usize), String> {
    // §126: 把 app clone 到 let 绑定, 让临时值在 state/Pool 借用结束后再 drop, 避免 E0716.
    let app_for_state = app.clone();
    let state: State<'_, AppState> = app_for_state.state();
    let pool = state.db_manager.pool();
    extract_missing_topics(app, pool, max_meetings.unwrap_or(10)).await
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_topic_name() {
        assert_eq!(normalize_topic_name("API 限流"), "api 限流");
        assert_eq!(normalize_topic_name("  API  限流!  "), "api 限流");
        assert_eq!(normalize_topic_name("Q3 OKR"), "q3 okr");
        assert_eq!(normalize_topic_name(""), "");
        // 标点 + 空白归一
        assert_eq!(normalize_topic_name("API限流（性能）"), "api限流 性能");
    }
}


// ============== §P2-B Topic dossier 重建 ==============

/// 跨会议知识图谱的累积 dossier 重建.
/// 拉取某 topic 所有 episode + sample excerpts, 调 BuiltInAI 合成
/// summary / open_questions / last_decided, 写回 topic_dossier.
/// 失败 swallow log warn, never panic.
pub async fn rebuild_topic_dossier<R: Runtime>(
    app: tauri::AppHandle<R>,
    pool: SqlitePool,
    topic_id: i64,
) -> Result<(), String> {
    // 1) topic name
    let canonical: Option<(String,)> = sqlx::query_as(
        "SELECT canonical_name FROM topic_node WHERE id = ?1",
    )
    .bind(topic_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| format!("rebuild_topic_dossier name: {e}"))?;
    let canonical = match canonical {
        Some(t) => t.0,
        None => return Err(format!("topic {topic_id} not found")),
    };

    // 2) aggregate all episodes (limit 64 latest) excerpts into a single input
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT meeting_id, COALESCE(excerpt, ''), sentiment          FROM meeting_episode_node WHERE topic_id = ?1          ORDER BY created_at DESC LIMIT 64",
    )
    .bind(topic_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("rebuild_topic_dossier eps: {e}"))?;
    if rows.is_empty() {
        log::info!("[topic_graph] rebuild {} no episodes", canonical);
        return Ok(());
    }
    let mut body = String::new();
    body.push_str(&format!("主题: {canonical}\n\n跨会议片段 (新→旧, 最多 64 条):\n\n"));
    for (mid, exc, sent) in rows.iter().rev() {
        body.push_str(&format!(
            "[meeting={} sentiment={}] {}\n",
            &mid[..12.min(mid.len())],
            sent,
            exc.chars().take(140).collect::<String>()
        ));
    }

    // 3) prompt LLM
    const DOSSIER_PROMPT_INSTRUCTIONS: &str = r#"你是跨会议知识助手。

请基于下面提供的历史会议片段, 用中文输出三段, 每段不超过 80 字:
 1. summary - 这个主题的累积背景 (只描述新进展)
 2. open_questions - 仍未解决的问题 (没有就写 "无待解决问题")
 3. last_decided - 上次明确做出的决议 (没有就写 "尚无决议")

三段用一行 --- 分隔, 不要 markdown 标题, 不要其他废话."#;
    let prompt = format!("{DOSSIER_PROMPT_INSTRUCTIONS}{body}");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let response = match generate_summary(
        &client,
        &LLMProvider::Ollama,        // §121: 改用 Ollama,见 trigger_after_summary 注释
        "qwen3.5:2b",
        "",
        "",
        &prompt,
        None, None,                  // ollama_endpoint (default localhost:11434) + custom_openai_endpoint
        Some(400),
        None, None, None, None,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            // §121 铁律 #3: 失败升级 error + emit Tauri 事件, 不再 swallow
            log::error!("[topic_graph] dossier LLM failed for {topic_id}: {e}");
            let _ = app.emit(
                "topic-dossier-failed",
                serde_json::json!({
                    "topic_id": topic_id,
                    "error": e.to_string(),
                    "at": chrono::Utc::now().to_rfc3339(),
                }),
            );
            return Ok(()); // 不阻塞 summary 完成, 但用户能在 UI 看到失败
        }
    };

    // 4) parse 3 sections split by "---"
    let mut sections = response.splitn(3, "\n---\n");
    let summary = sections.next().unwrap_or("").trim().to_string();
    let open_q = sections.next().unwrap_or("").trim().to_string();
    let last_dec = sections.next().unwrap_or("").trim().to_string();
    if summary.is_empty() && open_q.is_empty() && last_dec.is_empty() {
        log::info!("[topic_graph] dossier LLM returned empty for {topic_id}");
        return Ok(());
    }
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO topic_dossier (topic_id, status, summary, open_questions, last_decided, last_updated_at, rebuild_count)          VALUES (?1, 'open', ?2, ?3, ?4, ?5, 1)          ON CONFLICT(topic_id) DO UPDATE SET            summary = excluded.summary,            open_questions = excluded.open_questions,            last_decided = excluded.last_decided,            last_updated_at = excluded.last_updated_at,            rebuild_count = topic_dossier.rebuild_count + 1",
    )
    .bind(topic_id)
    .bind(&summary)
    .bind(&open_q)
    .bind(&last_dec)
    .bind(&now)
    .execute(&pool)
    .await
    .map_err(|e| format!("rebuild_topic_dossier upsert: {e}"))?;

    log::info!(
        "[topic_graph] dossier rebuilt tid={} ({}) summary={}c openq={}c decided={}c",
        topic_id,
        canonical,
        summary.len(),
        open_q.len(),
        last_dec.len()
    );

    let _ = app.emit(
        "topic-dossier-updated",
        serde_json::json!({ "topic_id": topic_id, "at": now }),
    );
    Ok(())
}
