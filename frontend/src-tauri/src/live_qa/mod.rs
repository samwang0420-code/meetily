// §P2-C Real-time mid-meeting Q&A (71 报告 P2-C "对齐 Charoite ⚡")
// 用户场景: 录音中按 ⌥+Space 弹输入框, 输入问题, 拉取最近 5 分钟 transcript
// 拼 prompt 给 BuiltInAI (Qwen 3.5 2B), 返 3 条简短建议.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::{AppHandle, Runtime};
use tokio::sync::OnceCell;

use crate::state::AppState;
use crate::summary::llm_client::{generate_summary, LLMProvider};
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveQASuggestion {
    pub text: String,
    pub rationale: String,  // 简短解释为什么这样回答
}

#[derive(Debug, Serialize)]
pub struct LiveQAResult {
    pub suggestions: Vec<LiveQASuggestion>,
    pub context_chars: usize,
    pub model: String,
}

pub const MAX_SUGGESTIONS: usize = 3;
pub const CONTEXT_WINDOW_SECS: i64 = 300; // 最近 5 分钟
pub const MAX_CONTEXT_CHARS: usize = 4000;
pub const MAX_TOKENS_PER_SUGGESTION: u32 = 120; // 3 条 × 120 = 360 token, < §52 上限

const QA_PROMPT_INSTRUCTIONS: &str = r#"你是一个实时会议助手. 用户会问一个具体问题.

基于最近会议上下文, 给出 3 条简短建议 (每条 ≤ 60 字), 按可能性高低排序.
每条格式: <建议内容> | <简短理由>

3 条用一行 --- 分隔. 不要标题, 不要编号, 不要 markdown."#;

static HTTP_CLIENT: OnceCell<reqwest::Client> = OnceCell::const_new();

async fn get_http_client() -> &'static reqwest::Client {
    HTTP_CLIENT
        .get_or_init(|| async {
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new())
        })
        .await
}

/// 拉最近 CONTEXT_WINDOW_SECS 秒 transcript, 拼成上下文.
async fn build_context(
    pool: &SqlitePool,
    meeting_id: &str,
) -> Result<String, String> {
    let cutoff = chrono::Utc::now() - chrono::Duration::seconds(CONTEXT_WINDOW_SECS);
    let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();

    let mut rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT speaker, transcript, COALESCE(audio_end_time, '') FROM transcripts
         WHERE meeting_id = ?1 AND created_at > ?2
         ORDER BY audio_end_time DESC, id DESC
         LIMIT 50",
    )
    .bind(meeting_id)
    .bind(&cutoff_str)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("live_qa context query: {e}"))?;

    if rows.is_empty() {
        return Err("no_recent_transcript".to_string());
    }

    // 旧→新顺序 (LLM 推理需要时间顺序)
    rows.reverse();
    let mut body = String::new();
    let mut total = 0usize;
    for (speaker, text, end_ts) in rows {
        if text.trim().is_empty() {
            continue;
        }
        let line = if end_ts.is_empty() {
            format!("{}: {}
", speaker, text)
        } else {
            format!("[{}s] {}: {}
", &end_ts[..end_ts.len().min(8)], speaker, text)
        };
        // 中文 UTF-8 3 byte/char, 按字符算
        let line_chars = line.chars().count();
        if total + line_chars > MAX_CONTEXT_CHARS {
            break;
        }
        body.push_str(&line);
        total += line_chars;
    }

    if body.trim().is_empty() {
        return Err("no_recent_transcript".to_string());
    }
    Ok(body)
}

/// 主入口: 给定 meeting_id + 用户 question, 返 3 条建议.
pub async fn ask_live_qa(
    pool: SqlitePool,
    meeting_id: String,
    question: String,
    provider: LLMProvider,    // §137.5: 用用户选的 provider (不再硬编码 Ollama)
    model_name: &str,        // §137.5: 用用户选的 model_name (不再硬编码 qwen3.5:2b)
) -> Result<LiveQAResult, String> {
    let q = question.trim();
    if q.is_empty() {
        return Err("empty_question".to_string());
    }
    if q.chars().count() > 500 {
        return Err("question_too_long".to_string());
    }

    let context = build_context(&pool, &meeting_id).await?;
    let context_chars = context.chars().count();

    let user_prompt = format!(
        "{QA_PROMPT_INSTRUCTIONS}\n\n最近会议上下文 (旧→新, ≤ {MAX_CONTEXT_CHARS} 字):\n{context}\n\n问题: {q}\n\n请给 3 条建议:"
    );

    let client = get_http_client().await;
    let app_data_dir = app_data_dir_for_built_in_ai(); // §137.5: BuiltInAI 需要
    let response = generate_summary(
        client,
        &provider,                    // §137.5: 用用户选的 provider (不再硬编码 Ollama)
        model_name,                   // §137.5: 用用户选的 model_name (不再硬编码 qwen3.5:2b)
        "",
        "",
        &user_prompt,
        None, None,
        Some(MAX_TOKENS_PER_SUGGESTION * MAX_SUGGESTIONS as u32 + 40), // 留 buffer
        Some(0.7),  // 稍高 temperature 多样性
        None,                          // top_p
        app_data_dir.as_ref(),         // app_data_dir (BuiltInAI 需要)
        None,                          // cancellation_token
    )
    .await?;

    let suggestions = parse_suggestions(&response);
    if suggestions.is_empty() {
        return Err("no_suggestions".to_string());
    }

    Ok(LiveQAResult {
        suggestions,
        context_chars,
        model: model_name.to_string(),
    })
}

fn parse_suggestions(response: &str) -> Vec<LiveQASuggestion> {
    let raw: Vec<&str> = response.split("\n---\n").collect();
    raw.iter()
        .take(MAX_SUGGESTIONS)
        .filter_map(|s| parse_single_suggestion(s.trim()))
        .collect()
}

fn parse_single_suggestion(line: &str) -> Option<LiveQASuggestion> {
    if line.is_empty() {
        return None;
    }
    // 格式: "<text> | <rationale>"
    let parts: Vec<&str> = line.splitn(2, '|').collect();
    let text = parts.first()?.trim().to_string();
    let rationale = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
    if text.is_empty() {
        None
    } else {
        Some(LiveQASuggestion { text, rationale })
    }
}

/// Tauri command wrapper.
#[tauri::command]
pub async fn api_meeting_live_qa<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: String,
    question: String,
    provider: String,         // §137.5: 前端传当前 modelConfig.provider
    model_name: String,       // §137.5: 前端传当前 modelConfig.model
) -> Result<LiveQAResult, String> {
    let pool = app.state::<AppState>().db_manager.pool().clone();
    let llm_provider = LLMProvider::from_str(&provider)
        .map_err(|e| format!("unsupported provider: {e}"))?;
    ask_live_qa(pool, meeting_id, question, llm_provider, &model_name).await
}

/// §137.5: BuiltInAI 需要 app_data_dir 路径, 其它 provider 返 None.
fn app_data_dir_for_built_in_ai() -> Option<std::path::PathBuf> {
    if let Ok(app_data_dir) = std::env::var("YANJINGAI_APP_DATA_DIR") {
        return Some(std::path::PathBuf::from(app_data_dir));
    }
    // 兜底: 标准 Library/Application Support 路径
    if let Ok(home) = std::env::var("HOME") {
        return Some(std::path::PathBuf::from(home).join("Library/Application Support/tech.yanjingai.app"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_suggestions_three() {
        let raw = "先用 A 方案 | 风险小\n---\n再讨论 B | 更稳\n---\n最后备选 C | 兜底";
        let s = parse_suggestions(raw);
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].text, "先用 A 方案");
        assert_eq!(s[0].rationale, "风险小");
        assert_eq!(s[2].text, "最后备选 C");
    }

    #[test]
    fn test_parse_suggestions_two_only() {
        let raw = "建议 1 | 理由 1\n---\n建议 2 | 理由 2";
        let s = parse_suggestions(raw);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_parse_suggestions_empty_rationale() {
        let raw = "单条无理由\n---\n第二条 | 有理由";
        let s = parse_suggestions(raw);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].rationale, "");
        assert_eq!(s[1].rationale, "有理由");
    }

    #[test]
    fn test_parse_suggestions_skip_empty() {
        let raw = "\n---\n实际建议 | 实际理由\n---\n";
        let s = parse_suggestions(raw);
        // 第一个是空, parse_single 返回 None; 第二个 OK; 第三个空 OK 也返回 None
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].text, "实际建议");
    }

    #[test]
    fn test_parse_suggestions_cap_at_max() {
        let raw = "1 | a\n---\n2 | b\n---\n3 | c\n---\n4 | d";
        let s = parse_suggestions(raw);
        assert_eq!(s.len(), 3); // MAX_SUGGESTIONS cap
    }

    #[test]
    fn test_parse_suggestions_whitespace_only_filtered() {
        let raw = "   \n---\n真实 | 答案\n---\n  ";
        let s = parse_suggestions(raw);
        assert_eq!(s.len(), 1);
    }
}
