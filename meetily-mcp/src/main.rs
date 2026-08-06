// §P1-A MCP server for 言镜 AI
// stdio JSON-RPC transport implementing Model Context Protocol (MCP) 2024-11-05
//
// Tools exposed:
//   1. search_meetings(query: string, limit?: number) -> Meeting[]
//   2. get_meeting_summary(meeting_id: string) -> {title, created_at, summary_markdown, action_items}
//   3. get_action_items(date_from?: string, date_to?: string) -> ActionItem[]
//
// Usage:
//   meetily-mcp                              # stdio mode (Claude Desktop / Cursor / Cline)
//   MEETILY_DB_PATH=... meetily-mcp          # override default DB location
//
// Storage:
//   Default: ~/Library/Application Support/cn.lixianhuiji.app/meeting_minutes.sqlite
//   Override: MEETILY_DB_PATH env var

use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Meeting {
    id: String,
    title: String,
    created_at: String,
    duration_minutes: Option<i64>,
    transcript_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActionItem {
    meeting_id: String,
    meeting_title: String,
    action: String,
    meeting_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MeetingSummary {
    meeting_id: String,
    title: String,
    created_at: String,
    summary_markdown: Option<String>,
    action_items: Vec<String>,
    key_points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl JsonRpcResponse {
    fn success(id: Option<Value>, result: Value) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: Some(result), error: None }
    }
    fn error(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError { code, message: message.into(), data: None }),
        }
    }
}

const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "meetily-mcp";
const SERVER_VERSION: &str = "0.1.0";

struct AppState {
    conn: Arc<Mutex<Connection>>,
}

impl AppState {
    fn new(db_path: PathBuf) -> Result<Self> {
        let conn = Connection::open_with_flags(
            &db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )
        .with_context(|| format!("open db: {}", db_path.display()))?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    async fn search_meetings(&self, query: &str, limit: usize) -> Result<Vec<Meeting>> {
        let conn = self.conn.lock().await;
        let like = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT id, title, created_at, \
                    CAST((julianday(updated_at) - julianday(created_at)) * 24 * 60 AS INTEGER) AS dur, \
                    (SELECT COUNT(*) FROM transcripts t WHERE t.meeting_id = m.id) AS tc \
             FROM meetings m \
             WHERE title LIKE ?1 OR id LIKE ?1 \
             ORDER BY created_at DESC LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![like, limit as i64], |row| {
                Ok(Meeting {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    duration_minutes: row.get(3)?,
                    transcript_count: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    async fn get_meeting_summary(&self, meeting_id: &str) -> Result<Option<MeetingSummary>> {
        let conn = self.conn.lock().await;
        // Get meeting title + created_at
        let meeting: Option<(String, String)> = conn
            .query_row(
                "SELECT title, created_at FROM meetings WHERE id = ?1",
                rusqlite::params![meeting_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();
        let (title, created_at) = match meeting {
            Some(m) => m,
            None => return Ok(None),
        };

        // Get summary_processes.result JSON
        let summary_md: Option<String> = conn
            .query_row(
                "SELECT result FROM summary_processes WHERE meeting_id = ?1 AND status = 'completed'",
                rusqlite::params![meeting_id],
                |row| row.get(0),
            )
            .ok()
            .flatten()
            .and_then(|json_str: String| {
                serde_json::from_str::<Value>(&json_str)
                    .ok()
                    .and_then(|v| v.get("markdown").and_then(|m| m.as_str()).map(String::from))
            });

        // Get first transcript (transcript column holds summary + key_points)
        let first_transcript: Option<(Option<String>, Option<String>, Option<String>)> = conn
            .query_row(
                "SELECT summary, action_items, key_points FROM transcripts WHERE meeting_id = ?1 LIMIT 1",
                rusqlite::params![meeting_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok();

        let (action_items_str, key_points_str) = match first_transcript {
            Some((_, ai, kp)) => (ai, kp),
            None => (None, None),
        };

        let action_items = parse_csv_field(action_items_str.as_deref());
        let key_points = parse_csv_field(key_points_str.as_deref());

        Ok(Some(MeetingSummary {
            meeting_id: meeting_id.into(),
            title,
            created_at,
            summary_markdown: summary_md,
            action_items,
            key_points,
        }))
    }

    async fn get_action_items(
        &self,
        date_from: Option<&str>,
        date_to: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ActionItem>> {
        let conn = self.conn.lock().await;
        // Pull action_items from all meetings (it's a CSV-ish string in transcripts)
        let from_clause = match (date_from, date_to) {
            (Some(_), Some(_)) => " AND m.created_at >= ?2 AND m.created_at <= ?3",
            (Some(_), None) => " AND m.created_at >= ?2",
            (None, Some(_)) => " AND m.created_at <= ?2",
            _ => "",
        };
        let sql = format!(
            "SELECT m.id, m.title, m.created_at, t.action_items \
             FROM meetings m LEFT JOIN transcripts t ON t.meeting_id = m.id \
             WHERE t.action_items IS NOT NULL AND t.action_items != '' {} \
             ORDER BY m.created_at DESC LIMIT ?1",
            from_clause
        );
        let mut stmt = conn.prepare(&sql)?;
        let map_row = |row: &rusqlite::Row| -> rusqlite::Result<(String, String, String, String)> {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        };
        let rows: Vec<(String, String, String, String)> = if from_clause.is_empty() {
            stmt.query_map(rusqlite::params![limit as i64], map_row)?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(rusqlite::params![limit as i64, date_from.unwrap_or(""), date_to.unwrap_or("")], map_row)?
                .collect::<Result<Vec<_>, _>>()?
        };
        let mut out = Vec::new();
        for (id, title, date, ai_str) in rows {
            for ai in parse_csv_field(Some(&ai_str)) {
                out.push(ActionItem {
                    meeting_id: id.clone(),
                    meeting_title: title.clone(),
                    action: ai,
                    meeting_date: date.clone(),
                });
            }
        }
        Ok(out)
    }
}

fn parse_csv_field(s: Option<&str>) -> Vec<String> {
    s.map(|raw| {
        raw.split(|c: char| c == '\n' || c == '|' || c == ';' || c == ',')
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect()
    })
    .unwrap_or_default()
}

// ====== JSON-RPC method dispatch ======

fn handle_initialize(_req: &JsonRpcRequest) -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION }
    })
}

fn handle_tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "search_meetings",
                "description": "Search past meetings by title or id. Returns a list of meetings with id / title / created_at / duration / transcript count. Use this to find which meeting to query next.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search keyword (matches title or meeting id, case-insensitive partial match)" },
                        "limit": { "type": "number", "description": "Max results to return (default 20, max 100)" }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "get_meeting_summary",
                "description": "Get the full summary + action items + key points for a single meeting by id. Use this when the user asks 'summarize my last meeting about X' or 'what were the action items for meeting Y'.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "meeting_id": { "type": "string", "description": "Meeting id (use search_meetings first to find the id)" }
                    },
                    "required": ["meeting_id"]
                }
            },
            {
                "name": "get_action_items",
                "description": "List all action items across meetings, optionally filtered by date range. Returns array of {meeting_id, meeting_title, action, meeting_date}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "date_from": { "type": "string", "description": "ISO date (YYYY-MM-DD) inclusive lower bound" },
                        "date_to": { "type": "string", "description": "ISO date (YYYY-MM-DD) inclusive upper bound" },
                        "limit": { "type": "number", "description": "Max action items to return (default 100, max 500)" }
                    }
                }
            }
        ]
    })
}

async fn handle_tools_call(state: Arc<AppState>, req: &JsonRpcRequest) -> Result<Value, (i32, String)> {
    let params = &req.params;
    let tool = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    match tool {
        "search_meetings" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            if query.is_empty() {
                return Err((-32602, "search_meetings: 'query' is required".into()));
            }
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            let limit = limit.min(100);
            match state.search_meetings(query, limit).await {
                Ok(meetings) => Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&meetings).unwrap_or_default() }] })),
                Err(e) => Err((-32603, format!("search_meetings failed: {e}"))),
            }
        }
        "get_meeting_summary" => {
            let meeting_id = args.get("meeting_id").and_then(|v| v.as_str()).unwrap_or("");
            if meeting_id.is_empty() {
                return Err((-32602, "get_meeting_summary: 'meeting_id' is required".into()));
            }
            match state.get_meeting_summary(meeting_id).await {
                Ok(Some(s)) => Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&s).unwrap_or_default() }] })),
                Ok(None) => Err((-32004, format!("meeting {meeting_id} not found"))),
                Err(e) => Err((-32603, format!("get_meeting_summary failed: {e}"))),
            }
        }
        "get_action_items" => {
            let date_from = args.get("date_from").and_then(|v| v.as_str());
            let date_to = args.get("date_to").and_then(|v| v.as_str());
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;
            let limit = limit.min(500);
            match state.get_action_items(date_from, date_to, limit).await {
                Ok(items) => Ok(json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&items).unwrap_or_default() }] })),
                Err(e) => Err((-32603, format!("get_action_items failed: {e}"))),
            }
        }
        _ => Err((-32601, format!("tool '{tool}' not implemented"))),
    }
}

async fn dispatch(state: Arc<AppState>, req: JsonRpcRequest) -> JsonRpcResponse {
    debug!("RPC << {}", req.method);
    let id = req.id.clone();
    match req.method.as_str() {
        "initialize" => JsonRpcResponse::success(id, handle_initialize(&req)),
        "ping" => JsonRpcResponse::success(id, json!({})),
        "tools/list" => JsonRpcResponse::success(id, handle_tools_list()),
        "tools/call" => match handle_tools_call(state, &req).await {
            Ok(v) => JsonRpcResponse::success(id, v),
            Err((code, msg)) => JsonRpcResponse::error(id, code, msg),
        },
        "notifications/initialized" | "notifications/cancelled" => {
            // Per spec, notifications don't get responses; we still emit empty id=null success
            JsonRpcResponse::success(None, json!({}))
        }
        other => JsonRpcResponse::error(id, -32601, format!("method '{other}' not found")),
    }
}

fn default_db_path() -> PathBuf {
    if let Ok(p) = std::env::var("MEETILY_DB_PATH") {
        return PathBuf::from(p);
    }
    if cfg!(target_os = "macos") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("cn.lixianhuiji.app")
                .join("meeting_minutes.sqlite");
        }
    } else if cfg!(target_os = "windows") {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata)
                .join("cn.lixianhuiji.app")
                .join("meeting_minutes.sqlite");
        }
    } else if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("cn.lixianhuiji.app")
            .join("meeting_minutes.sqlite");
    }
    PathBuf::from("meeting_minutes.sqlite")
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("meetily_mcp=info,warn")),
        )
        .with_writer(io::stderr)
        .init();

    let db_path = default_db_path();
    info!("meetily-mcp {} starting; db={}", SERVER_VERSION, db_path.display());
    if !db_path.exists() {
        warn!("db not found at {}; tools will return errors until 言镜 AI creates it", db_path.display());
    }
    let state = Arc::new(AppState::new(db_path)?);

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut lines = stdin.lock().lines();

    while let Some(Ok(line)) = lines.next() {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse::error(None, -32700, format!("parse error: {e}"));
                let s = serde_json::to_string(&resp).unwrap_or_default();
                writeln!(stdout, "{s}")?;
                stdout.flush()?;
                continue;
            }
        };
        let resp = dispatch(state.clone(), req).await;
        let s = serde_json::to_string(&resp).unwrap_or_default();
        writeln!(stdout, "{s}")?;
        stdout.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_csv_field_basic() {
        assert_eq!(parse_csv_field(Some("a | b | c")), vec!["a", "b", "c"]);
        assert_eq!(parse_csv_field(Some("a,b;c\nd")), vec!["a", "b", "c", "d"]);
        assert_eq!(parse_csv_field(None), Vec::<String>::new());
        assert_eq!(parse_csv_field(Some("")), Vec::<String>::new());
    }

    #[test]
    fn jsonrpc_success_format() {
        let r = JsonRpcResponse::success(Some(json!(1)), json!({"ok": true}));
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("\"result\""));
        assert!(!s.contains("\"error\""));
    }

    #[test]
    fn jsonrpc_error_format() {
        let r = JsonRpcResponse::error(Some(json!(1)), -32601, "method not found");
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("\"error\""));
        assert!(s.contains("-32601"));
    }

    #[test]
    fn default_db_path_is_under_application_support_on_macos() {
        let p = default_db_path();
        if cfg!(target_os = "macos") {
            assert!(p.to_string_lossy().contains("Application Support"));
            assert!(p.to_string_lossy().ends_with("meeting_minutes.sqlite"));
        }
    }
}
