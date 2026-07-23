// audio/sherpa_daemon.rs
//
// 离线会记 W2: sherpa-onnx 后端 daemon (subprocess-based)。
//
// 不引入 sherpa-rs crate(编译时间长、依赖重),改为 spawn 一个长寿命
// Python 子进程,通过 stdin/stdout 行式 JSON 通信。模型只加载一次,reuse。
// 失败回退到 whisper 输出,不阻塞主流程。

use anyhow::{anyhow, Result};
use log::{info, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

/// Python script path. Bundled with frontend/src-tauri/scripts/.
fn script_path() -> std::path::PathBuf {
    // Tauri dev mode: exe is target/debug/meetily, script is ../../src-tauri/scripts
    // Tauri release: same relative
    let candidates = [
        std::path::PathBuf::from("frontend/src-tauri/scripts/sherpa_asr.py"),
        std::path::PathBuf::from("../frontend/src-tauri/scripts/sherpa_asr.py"),
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/sherpa_asr.py"),
    ];
    for p in &candidates {
        if p.exists() {
            return p.clone();
        }
    }
    // fallback: just return the manifest one (will error on call but actionable)
    candidates.last().unwrap().clone()
}

/// Python interpreter: prefer python3 from PATH; fall back to system locations.
fn python_path() -> String {
    if let Ok(out) = Command::new("which").arg("python3").output() {
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() {
                return p;
            }
        }
    }
    "/usr/bin/python3".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SherpaResponse {
    pub id: String,
    pub ok: bool,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub confidence: f32,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub load_ms: u64,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub audio_seconds: f64,
    #[serde(default)]
    pub error: Option<String>,
    /// Level 3: 字级 token (仅 sensevoice-zh 模型 + Pro + RAM>=8GB 时返回)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tokens: Vec<String>,
    /// Level 3: 每个 token 的时间戳(秒),与 tokens 等长
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub timestamps: Vec<f32>,
    /// v0.7.0+: 说话人分离 segments (需要 sherpa-diarize 模型)
    /// 格式: [{"start": 0.5, "end": 3.2, "speaker": 0, "duration": 2.7, "text": "..."}]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<DiarSegment>,
}

/// Single speaker segment from OfflineSpeakerDiarizationResult.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiarSegment {
    pub start: f32,
    pub end: f32,
    pub speaker: i32,
    #[serde(default)]
    pub duration: f32,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Serialize)]
struct SherpaRequest<'a> {
    id: &'a str,
    model: &'a str,
    audio_b64: &'a str,
    sample_rate: u32,
    language: &'a str,
    /// 离线会记 v0.5.0: hotwords 词库
    hotwords_pack: &'a str,
    hotwords_custom: &'a str,
    /// Level 3: 请求字级 timestamps 返回 (默认 false)
    #[serde(default)]
    timestamps: bool,
    /// v0.7.1+: 当前 chunk 归属的 meeting_id (用于长会议 diar pickup 写库)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    meeting_id: Option<&'a str>,
    /// v0.7.1+: chunk 在整段录音中的开始偏移秒数 (用于 diar segment 时间映射)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    audio_start_offset_seconds: Option<f64>,
}

/// Single global daemon, lazily started on first call, kept alive until process exit.
///
/// P1-D (Diar 串行排队) 已知限制:
/// - SherpaDaemon 是单进程串行 stdin/stdout JSON-RPC. 同一时刻只处理 1 个 transcribe 请求.
/// - 多会议并发场景: 第二个会议要等第一个 transcribe 响应才能开始, 体感是"排队".
/// - 当前架构约束: IS_RECORDING 是 AtomicBool 全局单 flag, Rust 端只支持 1 个会议并行
///   (用户 UI 根本开不了多会议), 所以 P1-D 实际不会被触发.
/// - 长会议 diar 16 分钟异步计算由 sherpa_asr.py daemon 内部 background thread 处理,
///   不影响下一个 transcribe 请求的派发.
/// - P2 候选: 多会议支持 + per-meeting daemon pool. 等用户场景真实出现再做 (AGENTS.md §18).
pub struct SherpaDaemon {
    inner: Mutex<Option<SherpaHandle>>,
}

struct SherpaHandle {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl SherpaDaemon {
    fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// Ensure daemon running, return locked handle.
    fn ensure_started(&self) -> Result<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| anyhow!("daemon lock poisoned"))?;
        if guard.is_some() {
            return Ok(());
        }
        let script = script_path();
        info!(
            "[sherpa] spawning daemon: {} {}",
            python_path(),
            script.display()
        );
        let mut cmd = Command::new(python_path());
        cmd.arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| {
            anyhow!(
                "failed to spawn sherpa daemon ({}): {}",
                script.display(),
                e
            )
        })?;
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("no stdin"))?;
        let stdout = BufReader::new(child.stdout.take().ok_or_else(|| anyhow!("no stdout"))?);
        // Take stderr, log it on drop
        let _stderr = child.stderr.take();

        // Drain stderr in background
        if let Some(mut err) = _stderr {
            let h = std::thread::spawn(move || {
                use std::io::Read;
                let mut s = String::new();
                let _ = err.read_to_string(&mut s);
                if !s.trim().is_empty() {
                    warn!("[sherpa stderr] {}", s.trim());
                }
            });
            // detach; thread will exit naturally
            let _ = h;
        }

        *guard = Some(SherpaHandle {
            child,
            stdin,
            stdout,
        });
        info!("[sherpa] daemon ready");
        Ok(())
    }

    /// Send 1 request, read 1 response. Blocking (use block_in_place).
    /// `timestamps=true` 请求 Level 3 字级 timestamp (仅 sensevoice-zh + Pro + RAM>=8GB 实际返回)
    /// v0.6.11: 通用 JSON request / response (用于 streaming actions)
    fn send_request(&self, req_json: &serde_json::Value) -> Result<serde_json::Value> {
        self.ensure_started()?;
        // P0-fix: 每次请求更新 last_activity, 防止 idle killer 误杀
        crate::audio::sherpa_daemon::touch_daemon_activity();
        let mut guard = self.inner.lock().map_err(|_| anyhow!("lock"))?;
        let h = guard
            .as_mut()
            .ok_or_else(|| anyhow!("daemon not started"))?;
        let line = serde_json::to_string(req_json)?;
        writeln!(h.stdin, "{}", line).map_err(|e| anyhow!("write daemon: {}", e))?;
        h.stdin
            .flush()
            .map_err(|e| anyhow!("flush daemon: {}", e))?;
        let mut resp_line = String::new();
        h.stdout
            .read_line(&mut resp_line)
            .map_err(|e| anyhow!("read daemon: {}", e))?;
        let resp: serde_json::Value = serde_json::from_str(&resp_line)
            .map_err(|e| anyhow!("parse daemon '{}': {}", resp_line.trim(), e))?;
        Ok(resp)
    }

    /// v0.6.11: 开 streaming session
    pub fn stream_begin(
        &self,
        session_id: &str,
        model: &str,
        hotwords_pack: &str,
        hotwords_custom: &str,
    ) -> Result<serde_json::Value> {
        let req = serde_json::json!({
            "id": session_id,
            "action": "stream_begin",
            "model": model,
            "hotwords_pack": hotwords_pack,
            "hotwords_custom": hotwords_custom,
            "sample_rate": 16000,
            "chunk_threshold_ms": 600,
            "silence_threshold_ms": 1200,
            "force_final_ms": 8000,
        });
        self.send_request(&req)
    }

    /// v0.6.11: 推一段 audio chunk → 返 partial/final delta
    pub fn stream_chunk(&self, session_id: &str, audio_b64: &str) -> Result<serde_json::Value> {
        let req = serde_json::json!({
            "id": session_id,
            "action": "stream_chunk",
            "audio_b64": audio_b64,
        });
        self.send_request(&req)
    }

    /// v0.6.11: 关闭 streaming session, 推最后一段残留 audio
    pub fn stream_finalize(&self, session_id: &str) -> Result<serde_json::Value> {
        let req = serde_json::json!({
            "id": session_id,
            "action": "stream_finalize",
        });
        self.send_request(&req)
    }

    pub fn transcribe_blocking(
        &self,
        model: &str,
        audio_b64: &str,
        sample_rate: u32,
        timestamps: bool,
        hotwords_pack: &str,
        hotwords_custom: &str,
        // v0.7.1+: 当前 chunk 归属的 meeting_id, None 表示无 (短会议/录制外)
        meeting_id: Option<&str>,
        // v0.7.1+: chunk 在整段录音中的开始偏移秒数, 用于长会议 diar pickup 写库
        audio_start_offset_seconds: Option<f64>,
    ) -> Result<SherpaResponse> {
        let t0 = std::time::Instant::now();
        info!(
            "[sherpa] transcribe_blocking ENTER model={} timestamps={} b64_len={}",
            model,
            timestamps,
            audio_b64.len()
        );
        self.ensure_started()?;
        let t1 = std::time::Instant::now();
        info!("[sherpa] ensure_started OK ({:?})", t1.duration_since(t0));
        let mut guard = self.inner.lock().map_err(|_| anyhow!("lock"))?;
        let h = guard
            .as_mut()
            .ok_or_else(|| anyhow!("daemon not started"))?;

        // v0.7.x 修复: 之前的 id:"1" 写死导致 diar 落盘文件名全是 1.json,
        // 多个 chunk 互相覆盖, pickup loop 只能拿到最后一块的 segments.
        // 现在用 meeting_id + audio_start_offset 组合当 rid, 保证每个 chunk 独立落盘.
        let rid = match (meeting_id, audio_start_offset_seconds) {
            (Some(mid), Some(off)) => format!("{}-{}", mid, off),
            (_, Some(off)) => format!("anon-{}", off),
            (Some(mid), None) => format!("{}-noOff", mid),
            _ => "1".to_string(),
        };
        let req = SherpaRequest {
            id: &rid,
            model,
            audio_b64,
            sample_rate,
            language: "zh",
            timestamps,
            hotwords_pack,
            hotwords_custom,
            meeting_id,
            audio_start_offset_seconds,
        };
        let line = serde_json::to_string(&req)?;
        let t2 = std::time::Instant::now();
        info!(
            "[sherpa] serializing req OK ({:?}), writing to daemon stdin...",
            t2.duration_since(t1)
        );
        writeln!(h.stdin, "{}", line).map_err(|e| anyhow!("write to daemon: {}", e))?;
        h.stdin
            .flush()
            .map_err(|e| anyhow!("flush daemon stdin: {}", e))?;
        let t3 = std::time::Instant::now();
        info!(
            "[sherpa] stdin write+flush OK ({:?}), reading daemon stdout...",
            t3.duration_since(t2)
        );

        let mut resp_line = String::new();
        h.stdout
            .read_line(&mut resp_line)
            .map_err(|e| anyhow!("read daemon stdout: {}", e))?;
        let t4 = std::time::Instant::now();
        info!(
            "[sherpa] stdout read OK ({:?}), parse: {:?} | raw: {}",
            t4.duration_since(t3),
            serde_json::from_str::<SherpaResponse>(&resp_line)
                .map(|_| "OK")
                .map_err(|e| e.to_string()),
            resp_line.trim()
        );

        let resp: SherpaResponse = serde_json::from_str(&resp_line)
            .map_err(|e| anyhow!("parse daemon response '{}': {}", resp_line.trim(), e))?;
        if !resp.ok {
            return Err(anyhow!("sherpa error: {}", resp.error.unwrap_or_default()));
        }
        info!(
            "[sherpa] transcribe_blocking EXIT total={:?}",
            t4.duration_since(t0)
        );
        Ok(resp)
    }

    /// Stop daemon. Safe to call multiple times.
    /// Query daemon capability (Level 3 字级 timestamp 支持).
    /// Returns Ok(true) if RAM>=8GB + daemon reachable. Used at startup to decide whether
    /// to request timestamps from transcribe_blocking.
    pub fn capability(&self) -> Result<bool> {
        self.ensure_started()?;
        let mut guard = self.inner.lock().map_err(|_| anyhow!("lock"))?;
        let h = guard
            .as_mut()
            .ok_or_else(|| anyhow!("daemon not started"))?;

        let req = serde_json::json!({"id": "cap", "action": "capability"});
        let line = serde_json::to_string(&req)?;
        writeln!(h.stdin, "{}", line).map_err(|e| anyhow!("write capability: {}", e))?;
        h.stdin
            .flush()
            .map_err(|e| anyhow!("flush capability: {}", e))?;

        let mut resp_line = String::new();
        h.stdout
            .read_line(&mut resp_line)
            .map_err(|e| anyhow!("read capability: {}", e))?;
        let resp: serde_json::Value = serde_json::from_str(&resp_line)
            .map_err(|e| anyhow!("parse capability '{}': {}", resp_line.trim(), e))?;
        let supported = resp
            .get("level3_supported")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        info!(
            "[sherpa] capability: level3_supported={} (raw: {})",
            supported,
            resp_line.trim()
        );
        Ok(supported)
    }

    pub fn shutdown_blocking(&self) -> Result<()> {
        let mut guard = self.inner.lock().map_err(|_| anyhow!("lock"))?;
        if let Some(mut h) = guard.take() {
            let _ = h.child.kill();
            let _ = h.child.wait();
        }
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        // The actual kill is sync; wrap to satisfy async signature
        tokio::task::spawn_blocking(move || {
            // ignore: shutdown is best-effort
        })
        .await
        .ok();
        // also kill via a separate sync call
        let _ = self.shutdown_blocking();
        Ok(())
    }
}

pub static SHERPA_DAEMON: Lazy<SherpaDaemon> = Lazy::new(SherpaDaemon::new);

pub fn global() -> &'static SherpaDaemon {
    &SHERPA_DAEMON
}

/// P0-fix: 主动杀 sherpa Python 子进程, 释放 ~700M onnx 模型内存.
/// 调一次即可, 后续 transcribe 调用会重新 spawn (ensure_started 幂等).
/// App 退出 (RunEvent::Exit) + 录音停止 idle 超时都会调.
pub fn shutdown_global_daemon() {
    log::info!("[sherpa] shutdown_global_daemon requested");
    SHERPA_DAEMON.shutdown_blocking().ok();
}

// 全局 idle killer 状态, OnceLock<Arc<AtomicU64>> 持有共享引用, 让 init 闭包 move 进 thread
static DAEMON_LAST_ACTIVITY: std::sync::OnceLock<std::sync::Arc<std::sync::atomic::AtomicU64>> =
    std::sync::OnceLock::new();

fn daemon_last_activity() -> &'static std::sync::Arc<std::sync::atomic::AtomicU64> {
    DAEMON_LAST_ACTIVITY.get_or_init(|| {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::{Duration, SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let a = std::sync::Arc::new(AtomicU64::new(now));
        // 启动 idle killer 线程 (单次)
        let a_clone = a.clone();
        std::thread::spawn(move || {
            const IDLE_SECS: u64 = 120; // 2 min, 录完音 2min 没活动就杀 daemon, 释放 ~700M onnx
            loop {
                std::thread::sleep(Duration::from_secs(30));
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let last = a_clone.load(Ordering::SeqCst);
                if last == 0 {
                    continue;
                }
                let idle = now.saturating_sub(last);
                if idle > IDLE_SECS {
                    log::info!(
                        "[sherpa] daemon idle for {}s (>{}s), auto-shutdown to free onnx model RAM",
                        idle,
                        IDLE_SECS
                    );
                    SHERPA_DAEMON.shutdown_blocking().ok();
                    // 重置 last = 0, 下次 transcribe 会重新 spawn, 重新计时
                    a_clone.store(0, Ordering::SeqCst);
                }
            }
        });
        a
    })
}

pub(crate) fn touch_daemon_activity() {
    use std::sync::atomic::Ordering;
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    daemon_last_activity().store(now, Ordering::SeqCst);
}

// ============= Tauri commands (streaming pipeline) =============

#[tauri::command]
pub async fn sherpa_stream_begin(
    session_id: String,
    model: String,
    hotwords_pack: String,
    hotwords_custom: String,
) -> Result<serde_json::Value, String> {
    let daemon = global();
    tokio::task::block_in_place(|| {
        daemon
            .stream_begin(&session_id, &model, &hotwords_pack, &hotwords_custom)
            .map_err(|e| format!("{}", e))
    })
}

#[tauri::command]
pub async fn sherpa_stream_chunk(
    session_id: String,
    audio_b64: String,
) -> Result<serde_json::Value, String> {
    let daemon = global();
    tokio::task::block_in_place(|| {
        daemon
            .stream_chunk(&session_id, &audio_b64)
            .map_err(|e| format!("{}", e))
    })
}

#[tauri::command]
pub async fn sherpa_stream_finalize(session_id: String) -> Result<serde_json::Value, String> {
    let daemon = global();
    tokio::task::block_in_place(|| {
        daemon
            .stream_finalize(&session_id)
            .map_err(|e| format!("{}", e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// v0.7.0+ regression test: the request payload sent to the Python daemon must
    /// actually carry the hotwords_pack and hotwords_custom strings. A previous bug
    /// dropped these silently on the Rust side and the user never knew hotwords
    /// were not in effect. Keep this assertion in lockstep with transcribe_blocking.
    #[test]
    fn transcribe_blocking_request_carries_hotwords_payload() {
        let req = SherpaRequest {
            id: "ut",
            model: "paraformer-zh",
            audio_b64: "AAAA",
            sample_rate: 16000,
            language: "zh",
            timestamps: false,
            hotwords_pack: "tech",
            hotwords_custom: "Meetily,SenseVoice,Paraformer",
            meeting_id: Some("meeting-ut"),
            audio_start_offset_seconds: Some(12.5),
        };
        let v = serde_json::to_value(&req).expect("serialize");
        assert_eq!(
            v.get("model").and_then(|x| x.as_str()),
            Some("paraformer-zh")
        );
        assert_eq!(
            v.get("hotwords_pack").and_then(|x| x.as_str()),
            Some("tech")
        );
        assert_eq!(
            v.get("hotwords_custom").and_then(|x| x.as_str()),
            Some("Meetily,SenseVoice,Paraformer")
        );
        assert!(v.get("audio_b64").is_some(), "audio payload still present");
    }

    /// Stream session must also carry hotwords: the user picks "cross_border" once,
    /// then records a 30-minute meeting — every chunk must reuse the same custom
    /// vocabulary, otherwise chunk 5 would silently lose the biasing.
    #[test]
    fn stream_begin_request_carries_hotwords_payload() {
        let req = serde_json::json!({
            "id": "sess-1",
            "action": "stream_begin",
            "model": "paraformer-zh",
            "hotwords_pack": "cross_border",
            "hotwords_custom": "USDT,Shopee,Lazada,FBA,FBM",
            "sample_rate": 16000,
            "chunk_threshold_ms": 600,
            "silence_threshold_ms": 1200,
            "force_final_ms": 8000,
        });
        assert_eq!(
            req.get("hotwords_pack").and_then(|x| x.as_str()),
            Some("cross_border")
        );
        assert_eq!(
            req.get("force_final_ms").and_then(|x| x.as_i64()),
            Some(8000)
        );
        assert!(req
            .get("hotwords_custom")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .contains("USDT"));
        assert!(req
            .get("hotwords_custom")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .contains("Shopee"));
    }

    /// v0.7.0+ regression test: the diar segments payload deserialises correctly
    /// so the front-end can render "speaker_00" / "speaker_01" headers per turn.
    #[test]
    fn diar_segment_payload_roundtrips() {
        let json_str = r#"{
            "id": "ut",
            "ok": true,
            "text": "speaker_00 段 1 文本 speaker_01 段 2 文本",
            "num_speakers": 2,
            "segments": [
                {"start": 0.5, "end": 3.2, "speaker": 0, "duration": 2.7, "text": "段 1 文本"},
                {"start": 3.5, "end": 5.0, "speaker": 1, "duration": 1.5, "text": "段 2 文本"}
            ]
        }"#;
        let resp: SherpaResponse = serde_json::from_str(json_str).expect("parse");
        assert_eq!(resp.segments.len(), 2);
        assert_eq!(resp.segments[0].speaker, 0);
        assert!((resp.segments[0].start - 0.5).abs() < 1e-6);
        assert_eq!(resp.segments[1].speaker, 1);
        assert_eq!(resp.text, "speaker_00 段 1 文本 speaker_01 段 2 文本");
    }

    /// Backwards-compat: when segments are absent (e.g. < 10s audio or diar disabled),
    /// SherpaResponse still deserialises with empty segments Vec.
    #[test]
    fn diar_segments_absent_is_backward_compatible() {
        let json_str = r#"{"id":"ut","ok":true,"text":"hello"}"#;
        let resp: SherpaResponse = serde_json::from_str(json_str).expect("parse");
        assert!(resp.segments.is_empty());
        assert_eq!(resp.text, "hello");
    }
}
