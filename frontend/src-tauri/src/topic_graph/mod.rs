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
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let response = match generate_summary(
        &client,
        &LLMProvider::Ollama,        // §121: 改用 Ollama (localhost:11434,qwen3.5:2b)
                                    //      BuiltInAI 强制要 app_data_dir (sidecar binary),
                                    //      trigger 链路传 None -> llm 永远 fail -> swallow log
        "qwen3.5:2b",
        "",          // api_key unused for Ollama
        "",          // system_prompt (instructions already in user prompt)
        &prompt,
        None,        // ollama_endpoint (用默认 localhost:11434)
        None,        // custom_openai_endpoint
        Some(800),   // max_tokens (per AGENTS.md §52)
        None,        // temperature
        None,        // top_p
        None,        // app_data_dir (unused for Ollama)
        None,        // cancellation_token
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            log::warn!("[topic_graph] {meeting_id} llm extract failed: {e} (skip)");
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
            log::warn!("[topic_graph] dossier LLM failed for {topic_id}: {e}");
            return Ok(()); // swallow
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
