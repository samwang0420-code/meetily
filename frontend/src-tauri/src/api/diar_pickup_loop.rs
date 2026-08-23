//! v0.7.0+ P0-2: 后台定时扫描 /tmp/lixianhuiji_diar/, 把已生成的 diar segments
//! 回填到 transcripts.speaker. 这是 save_transcript 即时回填之外的"兜底"路径,
//! 覆盖长会议场景: sherpa 后台 diar 在 transcripts INSERT 完成前/后都安全
//! (即时回填会错过 30-90 分钟会议, 定时回填保证最终一致).
//!
//! 设计要点:
//! - Tokio 常驻 task, 每 30 秒循环扫一次
//! - 用 Python 子进程一次性脚本跑 overlap 算法 (复用 sherpa_daemon 旁路的 Python helper)
//! - 已处理的 json rename 成 `*.applied.{unix_ts}.json`, **不删除** (用户可手工恢复)
//! - 失败 non-fatal: 不抛给 Tauri 主线程, 仅 stderr 日志 + warn!
//! - 成功回填后 emit `transcripts-updated` 事件, 前端 listen 后 reload 当前 meeting

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};

const PICKUP_DIR: &str = "/tmp/lixianhuiji_diar";
const SCAN_INTERVAL_SECS: u64 = 30;

#[derive(Serialize, Clone)]
struct TranscriptsUpdatedEvent {
    meeting_ids: Vec<String>,
    source: &'static str,
}

/// v0.7.0+ P0-2: 在 lib.rs setup 里调一次. 启动后台 tokio loop.
pub fn spawn_diar_pickup_loop<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        // 首次启动延迟 5s, 等 db ready
        tokio::time::sleep(Duration::from_secs(5)).await;
        loop {
            if let Err(e) = scan_and_apply_once(&app).await {
                log::warn!("[diar_pickup_loop] scan failed (non-fatal): {}", e);
            }
            tokio::time::sleep(Duration::from_secs(SCAN_INTERVAL_SECS)).await;
        }
    });
}

/// 单次扫描 + apply. 公开以便测试和手动触发.
pub async fn scan_and_apply_once<R: Runtime>(app: &AppHandle<R>) -> Result<usize, String> {
    let pickup = Path::new(PICKUP_DIR);
    if !pickup.exists() {
        return Ok(0);
    }

    // 列目录, 过滤 .json (排除 .applied.*)
    let mut candidates: Vec<PathBuf> = Vec::new();
    let entries = std::fs::read_dir(pickup)
        .map_err(|e| format!("read_dir failed: {}", e))?;
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let name = match p.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if !name.ends_with(".json") {
            continue;
        }
        // 跳过 .applied.{ts}.json
        if name.contains(".applied.") {
            continue;
        }
        candidates.push(p);
    }

    if candidates.is_empty() {
        return Ok(0);
    }

    // 调 Python 一次性脚本, 传所有候选 json 路径
    let paths_str: Vec<String> = candidates
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    let output = run_apply_script(&paths_str)?;
    let result: ApplyResult = serde_json::from_str(&output)
        .map_err(|e| format!("python output not JSON: {} / raw={}", e, output))?;

    if result.updated_meetings.is_empty() {
        return Ok(0);
    }

    // §P1-A6 (audit 2026-08-23): only archive files the Python helper
    // confirmed it processed. Failing files (no meeting_id, no transcripts in
    // DB, JSON parse error, etc.) stay on disk and the next scan retries
    // them. Before this fix we renamed every candidate to .applied.*.json,
    // which permanently hid failures and lost speaker assignments.
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let processed: std::collections::HashSet<String> = result
        .processed_files
        .iter()
        .map(|s| std::fs::canonicalize(s).unwrap_or_else(|_| std::path::PathBuf::from(s))
            .to_string_lossy().into_owned())
        .collect();
    let mut archived = 0usize;
    let mut left = 0usize;
    for path in &candidates {
        let canonical = std::fs::canonicalize(path)
            .unwrap_or_else(|_| path.clone())
            .to_string_lossy()
            .into_owned();
        if !processed.contains(&canonical) {
            left += 1;
            log::warn!(
                "[diar_pickup_loop] leaving {} on disk for retry (not in processed_files)",
                path.display()
            );
            continue;
        }
        let applied_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.replace(".json", &format!(".applied.{}.json", now_ts)),
            None => continue,
        };
        let applied_path = path.with_file_name(applied_name);
        if let Err(e) = std::fs::rename(path, &applied_path) {
            log::warn!(
                "[diar_pickup_loop] rename {} -> {} failed: {}",
                path.display(),
                applied_path.display(),
                e
            );
        } else {
            archived += 1;
        }
    }
    log::info!(
        "[diar_pickup_loop] archived={} left_for_retry={}",
        archived, left
    );

    // 埋点: 写 analytics_events (运维埋点, 不受 opt-in 控制)
    write_pickup_audit(app, &result.updated_meetings, result.updated_rows);

    // 通知前端: emit transcripts-updated
    let _ = app.emit(
        "transcripts-updated",
        TranscriptsUpdatedEvent {
            meeting_ids: result.updated_meetings.clone(),
            source: "diar_pickup_loop",
        },
    );

    Ok(result.updated_meetings.len())
}

#[derive(serde::Deserialize, Default)]
struct ApplyResult {
    #[serde(default)]
    updated_meetings: Vec<String>,
    #[serde(default)]
    updated_rows: i64,
    #[serde(default)]
    #[allow(dead_code)] // §F: 历史字段
    matched_files: i64,
    #[serde(default)]
    #[allow(dead_code)] // §F: 历史字段
    unmatched_files: i64,
    /// §P1-A6: list of candidate paths that the Python helper successfully
    /// processed and which are therefore safe to archive into .applied.*.json.
    /// Anything not in this list must stay on disk so the next scan retries.
    #[serde(default)]
    processed_files: Vec<String>,
}

/// v0.7.0+ P0-2: 调 Python stdlib 一次性脚本.
fn run_apply_script(json_paths: &[String]) -> Result<String, String> {
    let script = r#"
import os, sqlite3, json, sys, glob, time

paths = sys.argv[1:]
# §97 (2026-08-09): YANJINGAI env var 优先, 旧 LIXIANHUIJI 兼容, 最后 fallback 新 bundle id
db_path = (
    os.environ.get("YANJINGAI_DIAR_DB_PATH")
    or os.environ.get("LIXIANHUIJI_DIAR_DB_PATH")
    or os.path.expanduser("~/Library/Application Support/tech.yanjingai.app/meeting_minutes.sqlite")
)
# §P1-A6 (audit 2026-08-23): we must report per-file success so the Rust caller
# only renames matched files into the .applied namespace. Before this fix the
# Rust side renamed *every* candidate to .applied.*.json — failing files (no
# meeting_id, no transcripts in DB, JSON parse error) got marked as processed
# and their speaker assignments were permanently lost.
result = {"updated_meetings": [], "updated_rows": 0, "matched_files": 0, "unmatched_files": 0, "processed_files": []}
if not os.path.exists(db_path):
    print(json.dumps(result))
    sys.exit(0)
conn = sqlite3.connect(db_path, timeout=5.0)
try:
    cur = conn.cursor()
    updated_meetings_set = set()
    for path in paths:
        try:
            with open(path) as f: payload = json.load(f)
        except Exception as e:
            result["unmatched_files"] += 1
            continue
        meeting_id = payload.get("meeting_id")
        offset = payload.get("audio_start_offset_seconds")
        segments = payload.get("segments") or []
        if not meeting_id or offset is None or not segments:
            result["unmatched_files"] += 1
            continue
        offset_f = float(offset)
        glob_segs = [
            (float(s.get("start", 0.0)) + offset_f, float(s.get("end", 0.0)) + offset_f, int(s.get("speaker", 0)))
            for s in segments
        ]
        cur.execute(
            "SELECT id, audio_start_time, audio_end_time FROM transcripts "
            "WHERE meeting_id = ? AND speaker IS NULL ORDER BY audio_start_time",
            (meeting_id,),
        )
        rows = cur.fetchall()
        if not rows:
            result["unmatched_files"] += 1
            continue
        result["matched_files"] += 1
        result["processed_files"].append(path)
        for row_id, t_start, t_end in rows:
            if t_start is None or t_end is None:
                continue
            best_ov, best_sp = 0.0, None
            for s_start, s_end, s_idx in glob_segs:
                ov = max(0.0, min(t_end, s_end) - max(t_start, s_start))
                if ov > best_ov:
                    best_ov, best_sp = ov, s_idx
            if best_sp is None or best_ov <= 0.0:
                continue
            label = f"speaker_{int(best_sp):02d}"
            cur.execute("UPDATE transcripts SET speaker = ? WHERE id = ? AND speaker IS NULL",
                        (label, row_id))
            if cur.rowcount > 0:
                result["updated_rows"] += cur.rowcount
                updated_meetings_set.add(meeting_id)
    conn.commit()
    result["updated_meetings"] = sorted(updated_meetings_set)
    print(json.dumps(result))
finally:
    conn.close()
"#;
    let mut cmd = std::process::Command::new("python3");
    cmd.arg("-c").arg(script);
    for p in json_paths {
        cmd.arg(p);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("python3 spawn failed: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("python helper exited with {:?}: {}", output.status.code(), stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// v0.7.0+ P0-2: 运维埋点. 写到 analytics_events 表 (绕过 opt-in,
/// 因为这是系统事件不是用户行为). 失败静默.
fn write_pickup_audit<R: Runtime>(app: &AppHandle<R>, updated_meetings: &[String], updated_rows: i64) {
    let state: tauri::State<crate::state::AppState> = app.state();
    let pool = state.db_manager.pool().clone();
    let meeting_list = updated_meetings.join(",");
    let props = serde_json::json!({
        "updated_meetings": meeting_list,
        "updated_rows": updated_rows,
        "source": "diar_pickup_loop",
    });
    let props_str = props.to_string();
    // fire-and-forget
    tauri::async_runtime::spawn(async move {
        let _ = sqlx::query(
            "INSERT INTO analytics_events (event_name, properties_json) VALUES (?1, ?2)",
        )
        .bind("diar_pickup_loop")
        .bind(props_str)
        .execute(&pool)
        .await;
    });
    log::info!(
        "[diar_pickup_loop] applied: meetings={} rows={}",
        meeting_list,
        updated_rows
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pickup_dir_constant_is_absolute() {
        assert!(PICKUP_DIR.starts_with('/'));
        assert_eq!(SCAN_INTERVAL_SECS, 30);
    }

    #[test]
    fn test_apply_result_default_empty() {
        let r: ApplyResult = serde_json::from_str("{}").unwrap();
        assert!(r.updated_meetings.is_empty());
        assert_eq!(r.updated_rows, 0);
        assert_eq!(r.matched_files, 0);
    }

    #[test]
    fn test_apply_result_parses_full() {
        let r: ApplyResult = serde_json::from_str(
            r#"{"updated_meetings":["m1","m2"],"updated_rows":42,"matched_files":3,"unmatched_files":1}"#,
        )
        .unwrap();
        assert_eq!(r.updated_meetings, vec!["m1", "m2"]);
        assert_eq!(r.updated_rows, 42);
        assert_eq!(r.matched_files, 3);
        assert_eq!(r.unmatched_files, 1);
    }
}
