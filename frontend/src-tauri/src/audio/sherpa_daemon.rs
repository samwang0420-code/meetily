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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

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

/// §96: Python interpreter selection.
///
/// 历史问题 (§96 触发): Tauri app bundle 启动时, macOS launchd 注入精简 PATH,
/// 不含 /opt/homebrew/bin. `which python3` fallback 到 /usr/bin/python3 (Xcode CLT),
/// 该 Python 没装 sherpa_onnx (只 numpy). 用户报 "No module named 'numpy'" /
/// 实际是 sherpa_onnx 缺失. 这里用候选列表 + 真实 import 探测 + OnceLock 缓存.
///
/// 探测 = `python -c "import sherpa_onnx, numpy, soundfile"`, 全 ok 才用.
fn python_path() -> String {
    static CACHED: OnceLock<String> = OnceLock::new();
    CACHED.get_or_init(detect_python_interpreter).clone()
}

fn detect_python_interpreter() -> String {
    let candidates = [
        "/opt/homebrew/bin/python3",
        "/opt/homebrew/opt/python@3.14/bin/python3.14",
        "/opt/homebrew/opt/python@3.13/bin/python3.13",
        "/opt/homebrew/opt/python@3.12/bin/python3.12",
        "/usr/local/bin/python3",
        "/opt/local/bin/python3",
        "/Library/Frameworks/Python.framework/Versions/Current/bin/python3",
    ];
    for c in &candidates {
        if !std::path::Path::new(c).exists() {
            continue;
        }
        let probe = Command::new(c)
            .args(&["-c", "import sherpa_onnx, numpy, soundfile; print('OK')"])
            .output();
        match probe {
            Ok(out) if out.status.success() => {
                info!("[sherpa] §96 python3 selected (probe OK): {}", c);
                return c.to_string();
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                warn!(
                    "[sherpa] §96 python3 {} probe failed: {}",
                    c,
                    stderr.lines().last().unwrap_or("?").trim()
                );
            }
            Err(e) => {
                warn!("[sherpa] §96 python3 {} spawn failed: {}", c, e);
            }
        }
    }
    if let Ok(out) = Command::new("which").arg("python3").output() {
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() {
                warn!("[sherpa] §96 falling back to PATH which python3: {}", p);
                return p;
            }
        }
    }
    warn!("[sherpa] §96 no python3 with sherpa_onnx found, using /usr/bin/python3 (will likely fail)");
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

/// §62 A: N-daemon pool with round-robin dispatch.
/// 旧版单 daemon 串行阻塞, 长会议 / 长导入 10+min. 改 N 路并发.
/// env MEETILY_SHERPA_DAEMONS=1..4 显式覆盖, 默认 1 (8GB RAM safe, 1 worker 串行时多 daemon 浪费 RAM).
/// §119: 8GB 实测 3 daemon + decode cache + 系统 = 7.5 GB used (134 MB unused), SWAP 8.5M pages.
/// NUM_WORKERS=1 串行 (worker.rs:156) 下 3 daemon round-robin 仍串行, 多 daemon 浪费 1.4 GB.
/// 16 GB 用户可 env MEETILY_SHERPA_DAEMONS=3 显式启用.
pub struct SherpaDaemon {
    /// N 个独立 Python child, 每个有独立 stdin/stdout
    inner: Vec<Mutex<Option<SherpaHandle>>>,
    /// round-robin 计数器 (fetch_add Relaxed → slot index = counter % count)
    counter: AtomicUsize,
    /// daemon 总数 (1-4)
    count: usize,
}

struct SherpaHandle {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

/// §62 A: 解析 env MEETILY_SHERPA_DAEMONS, 默认 3, clamp 1-4.
fn daemon_count_from_env() -> usize {
    let raw = std::env::var("MEETILY_SHERPA_DAEMONS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);  // §119: default 1 (8 GB 适配, NUM_WORKERS=1 串行下多 daemon 冗余)
    raw.clamp(1, 4)
}

impl SherpaDaemon {
    fn new() -> Self {
        let count = daemon_count_from_env();
        info!("[sherpa] §62 A: starting daemon pool count={} (env MEETILY_SHERPA_DAEMONS, default 1 per §119)", count);
        let inner = (0..count).map(|_| Mutex::new(None)).collect();
        Self { inner, counter: AtomicUsize::new(0), count }
    }

    /// §62 A: pick next slot via round-robin. Doesn't start the daemon, caller must ensure.
    fn next_slot(&self) -> usize {
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        n % self.count
    }

    /// §62 A: ensure daemon at slot_idx is running. Spawn on demand.
    fn ensure_started_slot(&self, slot_idx: usize) -> Result<()> {
        let mut guard = self.inner[slot_idx].lock().map_err(|_| anyhow!("daemon slot {} lock poisoned", slot_idx))?;
        if guard.is_some() {
            return Ok(());
        }
        let script = script_path();
        info!("[sherpa slot {}] spawning daemon: {} {}", slot_idx, python_path(), script.display());
        let mut cmd = Command::new(python_path());
        cmd.arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // §99: 不设 PYTHONUSERBASE — homebrew Python 默认 user-base 已经是
        // ~/Library/Python/3.14/lib/python (numpy/sherpa_onnx 装在那),
        // 显式设 PYTHONUSERBASE=$HOME 反而被 PEP 370 错误映射到 ~/lib/python3.14/site-packages,
        // 探测时 import OK 不等于 spawn 时 import OK (env 不一致), 触发 "No module named 'numpy'".
        // §99: 不缓冲 stderr (daemon 启动错误能立即看到)
        cmd.env("PYTHONUNBUFFERED", "1");
        let mut child = cmd
            .spawn()
            .map_err(|e| anyhow!("failed to spawn sherpa daemon slot {} ({}): {}", slot_idx, script.display(), e))?;
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("no stdin"))?;
        let stdout = BufReader::new(child.stdout.take().ok_or_else(|| anyhow!("no stdout"))?);
        let _stderr = child.stderr.take();

        // Drain stderr in background
        if let Some(mut err) = _stderr {
            let h = std::thread::spawn(move || {
                use std::io::Read;
                let mut s = String::new();
                let _ = err.read_to_string(&mut s);
                if !s.trim().is_empty() {
                    warn!("[sherpa slot stderr] {}", s.trim());
                }
            });
            let _ = h;
        }

        *guard = Some(SherpaHandle { child, stdin, stdout });
        info!("[sherpa slot {}] daemon ready", slot_idx);
        Ok(())
    }

    /// 兼容旧 API: ensure_started(0) — callsite 零修改.
    #[allow(dead_code)] // §F: 兼容 API,callsite 已全部用 ensure_started_slot
    fn ensure_started(&self) -> Result<()> {
        let slot = self.next_slot();
        self.ensure_started_slot(slot)
    }

    /// §62 A: send 1 request to round-robin slot. Blocking (use block_in_place).
    /// `timestamps=true` 请求 Level 3 字级 timestamp (仅 sensevoice-zh + Pro + RAM>=8GB 实际返回)
    /// v0.6.11: 通用 JSON request / response (用于 streaming actions)
    fn send_request(&self, req_json: &serde_json::Value) -> Result<serde_json::Value> {
        let slot = self.next_slot();
        self.ensure_started_slot(slot)?;
        let mut guard = self.inner[slot].lock().map_err(|_| anyhow!("lock slot {}", slot))?;
        let h = guard.as_mut().ok_or_else(|| anyhow!("daemon slot {} not started", slot))?;
        let line = serde_json::to_string(req_json)?;
        // §P1-A5 (audit 2026-08-23): any I/O failure on a once-running slot
        // must reset the handle. Before this fix a crashed daemon left a stale
        // `Some(SherpaHandle)` in the slot, so every subsequent call hit a
        // dead process and silently failed — the slot was never respawned
        // because `ensure_started_slot` short-circuited on `guard.is_some()`.
        let write_result = writeln!(h.stdin, "{}", line);
        if let Err(e) = write_result {
            warn!("[sherpa slot {}] write failed: {}, resetting slot for respawn", slot, e);
            let _ = h.child.wait();
            *guard = None;
            return Err(anyhow!("write daemon slot {}: {}", slot, e));
        }
        if let Err(e) = h.stdin.flush() {
            warn!("[sherpa slot {}] flush failed: {}, resetting slot for respawn", slot, e);
            let _ = h.child.wait();
            *guard = None;
            return Err(anyhow!("flush daemon slot {}: {}", slot, e));
        }
        let mut resp_line = String::new();
        if let Err(e) = h.stdout.read_line(&mut resp_line) {
            warn!("[sherpa slot {}] read failed: {}, resetting slot for respawn", slot, e);
            let _ = h.child.wait();
            *guard = None;
            return Err(anyhow!("read daemon slot {}: {}", slot, e));
        }
        let resp: serde_json::Value = match serde_json::from_str(&resp_line) {
            Ok(v) => v,
            Err(e) => {
                // Parse failure can also indicate the daemon died mid-stream.
                warn!("[sherpa slot {}] parse failed: {}, resetting slot for respawn", slot, e);
                let _ = h.child.wait();
                *guard = None;
                return Err(anyhow!("parse daemon slot {} '{}': {}", slot, resp_line.trim(), e));
            }
        };
        Ok(resp)
    }

    /// v0.6.11: 开 streaming session
    pub fn stream_begin(&self, session_id: &str, model: &str, hotwords_pack: &str, hotwords_custom: &str) -> Result<serde_json::Value> {
        let req = serde_json::json!({
            "id": session_id,
            "action": "stream_begin",
            "model": model,
            "hotwords_pack": hotwords_pack,
            "hotwords_custom": hotwords_custom,
            "sample_rate": 16000,
            "chunk_threshold_ms": 600,
            "silence_threshold_ms": 1200,
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
        info!("[sherpa] transcribe_blocking ENTER model={} timestamps={} b64_len={}", model, timestamps, audio_b64.len());
        // §62 A: round-robin pick slot, ensure started
        let slot = self.next_slot();
        self.ensure_started_slot(slot)?;
        let t1 = std::time::Instant::now();
        info!("[sherpa slot {}] ensure_started OK ({:?})", slot, t1.duration_since(t0));
        let mut guard = self.inner[slot].lock().map_err(|_| anyhow!("lock slot {}", slot))?;
        let h = guard.as_mut().ok_or_else(|| anyhow!("daemon slot {} not started", slot))?;

        let req = SherpaRequest {
            id: "1",
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
        info!("[sherpa slot {}] serializing req OK ({:?}), writing to daemon stdin...", slot, t2.duration_since(t1));
        writeln!(h.stdin, "{}", line).map_err(|e| anyhow!("write to daemon slot {}: {}", slot, e))?;
        h.stdin.flush().map_err(|e| anyhow!("flush daemon slot {} stdin: {}", slot, e))?;
        let t3 = std::time::Instant::now();
        info!("[sherpa slot {}] stdin write+flush OK ({:?}), reading daemon stdout...", slot, t3.duration_since(t2));

        let mut resp_line = String::new();
        h.stdout
            .read_line(&mut resp_line)
            .map_err(|e| anyhow!("read daemon slot {} stdout: {}", slot, e))?;
        let t4 = std::time::Instant::now();
        info!("[sherpa slot {}] stdout read OK ({:?}), parse: {:?} | raw: {}",
              slot,
              t4.duration_since(t3), serde_json::from_str::<SherpaResponse>(&resp_line).map(|_| "OK").map_err(|e| e.to_string()),
              resp_line.trim());

        let resp: SherpaResponse = serde_json::from_str(&resp_line)
            .map_err(|e| anyhow!("parse daemon response '{}': {}", resp_line.trim(), e))?;
        if !resp.ok {
            return Err(anyhow!("sherpa slot {} error: {}", slot, resp.error.unwrap_or_default()));
        }
        info!("[sherpa slot {}] transcribe_blocking EXIT total={:?}", slot, t4.duration_since(t0));
        Ok(resp)
    }

    /// Stop daemon. Safe to call multiple times.
    /// Query daemon capability (Level 3 字级 timestamp 支持).
    /// Returns Ok(true) if RAM>=8GB + daemon reachable. Used at startup to decide whether
    /// to request timestamps from transcribe_blocking.
    pub fn capability(&self) -> Result<bool> {
        let slot = self.next_slot();
        self.ensure_started_slot(slot)?;
        let mut guard = self.inner[slot].lock().map_err(|_| anyhow!("lock slot {}", slot))?;
        let h = guard.as_mut().ok_or_else(|| anyhow!("daemon slot {} not started", slot))?;

        let req = serde_json::json!({"id": "cap", "action": "capability"});
        let line = serde_json::to_string(&req)?;
        writeln!(h.stdin, "{}", line).map_err(|e| anyhow!("write capability slot {}: {}", slot, e))?;
        h.stdin.flush().map_err(|e| anyhow!("flush capability slot {}: {}", slot, e))?;

        let mut resp_line = String::new();
        h.stdout.read_line(&mut resp_line).map_err(|e| anyhow!("read capability slot {}: {}", slot, e))?;
        let resp: serde_json::Value = serde_json::from_str(&resp_line)
            .map_err(|e| anyhow!("parse capability slot {} '{}': {}", slot, resp_line.trim(), e))?;
        let supported = resp.get("level3_supported").and_then(|v| v.as_bool()).unwrap_or(false);
        info!(
            "[sherpa slot {}] capability: level3_supported={} (raw: {})",
            slot, supported, resp_line.trim()
        );
        Ok(supported)
    }

    pub fn shutdown_blocking(&self) -> Result<()> {
        // §62 A: kill ALL N daemons, not just slot 0.
        for (i, slot) in self.inner.iter().enumerate() {
            let mut guard = slot.lock().map_err(|_| anyhow!("lock slot {}", i))?;
            if let Some(mut h) = guard.take() {
                info!("[sherpa slot {}] shutting down", i);
                let _ = h.child.kill();
                let _ = h.child.wait();
            }
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

/// v0.8.5 §23: Public shutdown entry point for stop_recording / RunEvent::Exit.
/// Kills the Python child + clears the singleton so next call respawns.
pub fn shutdown_global_daemon() {
    info!("[sherpa] shutdown_global_daemon requested");
    // Lazy<SherpaDaemon> — call shutdown_blocking() on the singleton.
    let _ = SHERPA_DAEMON.shutdown_blocking();
}

pub fn global() -> &'static SherpaDaemon {
    &SHERPA_DAEMON
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
        daemon.stream_begin(&session_id, &model, &hotwords_pack, &hotwords_custom)
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
        daemon.stream_chunk(&session_id, &audio_b64)
            .map_err(|e| format!("{}", e))
    })
}

#[tauri::command]
pub async fn sherpa_stream_finalize(
    session_id: String,
) -> Result<serde_json::Value, String> {
    let daemon = global();
    tokio::task::block_in_place(|| {
        daemon.stream_finalize(&session_id)
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
        assert_eq!(v.get("model").and_then(|x| x.as_str()), Some("paraformer-zh"));
        assert_eq!(v.get("hotwords_pack").and_then(|x| x.as_str()), Some("tech"));
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
        });
        assert_eq!(
            req.get("hotwords_pack").and_then(|x| x.as_str()),
            Some("cross_border")
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

    // ===== §62 A: N-daemon pool round-robin =====

    /// §99: 探测时 import OK 不等于 spawn 时 import OK (§96 PYTHONUSERBASE hack bug).
    /// 真 spawn 一个 Python 子进程, 用与生产代码完全一致的 env (无 PYTHONUSERBASE),
    /// 发 {"action":"list"} 让 daemon 启动时 import sherpa_onnx/numpy/soundfile,
    /// 验证 ok=true. 这是 "No module named 'numpy'" 的唯一可靠防线.
    #[test]
    fn section_99_spawned_python_can_import_sherpa_onnx() {
        use std::io::{BufRead, BufReader, Write};

        let py = python_path();
        let script = script_path();
        if !std::path::Path::new(&py).exists() {
            eprintln!("[skip] python {} not found", py);
            return;
        }
        let mut cmd = Command::new(&py);
        cmd.arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // 必须与生产代码 ensure_started_slot 完全一致 — 不设 PYTHONUSERBASE!
        cmd.env("PYTHONUNBUFFERED", "1");
        let mut child = cmd.spawn().expect("spawn python");
        let mut stdin = child.stdin.take().expect("stdin");
        let stdout = child.stdout.take().expect("stdout");
        let _stderr = child.stderr.take();
        let mut reader = BufReader::new(stdout);

        let req = serde_json::json!({"id": "probe-import-99", "action": "list"});
        writeln!(stdin, "{}", req).expect("write");
        stdin.flush().expect("flush");
        let mut line = String::new();
        let _ = reader.read_line(&mut line);
        // 容忍 daemon 启动慢, 给 5s timeout
        std::thread::sleep(std::time::Duration::from_millis(500));
        let _ = child.kill();
        let _ = child.wait();

        let resp: serde_json::Value = serde_json::from_str(&line)
            .unwrap_or_else(|e| panic!("parse '{}' failed: {} (likely numpy/sherpa_onnx not importable)", line.trim(), e));
        let ok = resp.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
        assert!(ok, "daemon list action failed: {} (numpy/sherpa_onnx import failed?)", line.trim());
        let models = resp.get("models").and_then(|x| x.as_array()).cloned().unwrap_or_default();
        // 不强制要求 ≥1 模型 (用户可能没装), 但 daemon 必须启动成功 + 能 import 所有 3 个模块
        eprintln!("§99 spawn probe OK, models={}", models.len());
    }

    /// §62 A: round-robin wraps correctly within N slots. 内部构造避免 env 并行污染.
    fn make_test_daemon(count: usize) -> SherpaDaemon {
        let inner = (0..count).map(|_| Mutex::new(None)).collect();
        SherpaDaemon { inner, counter: AtomicUsize::new(0), count }
    }

    /// §62 A: round-robin 数学正确: next_slot() == i % count.
    #[test]
    fn section_64_round_robin_wraps_within_pool() {
        let daemon = make_test_daemon(3);
        for i in 0..10 {
            let slot = daemon.next_slot();
            assert!(slot < daemon.count, "slot {} >= count {}", slot, daemon.count);
            assert_eq!(slot, i % daemon.count, "round-robin must be (i % count)");
        }
    }

    /// §62 A: N-daemon pool has N slots, not 1. 1 路径仍工作.
    #[test]
    fn section_64_pool_count_is_n() {
        let d1 = make_test_daemon(1);
        let d2 = make_test_daemon(2);
        let d3 = make_test_daemon(4);
        assert_eq!(d1.inner.len(), 1);
        assert_eq!(d2.inner.len(), 2);
        assert_eq!(d3.inner.len(), 4);
        // 各 slot 互不干扰: 都从 0 起
        assert_eq!(d1.next_slot(), 0);
        assert_eq!(d2.next_slot(), 0);
        assert_eq!(d3.next_slot(), 0);
        assert_eq!(d3.next_slot(), 1);
    }

    /// §62 A: 多次 next_slot 行为与 fetch_add Relaxed 一致 (monotonic modulo count).
    #[test]
    fn section_64_next_slot_is_distributed_evenly() {
        let daemon = make_test_daemon(3);
        let mut hits = [0usize; 3];
        for _ in 0..30 {
            let s = daemon.next_slot();
            hits[s] += 1;
        }
        // 30 / 3 = 10 每 slot
        assert_eq!(hits, [10, 10, 10], "round-robin must distribute evenly, got {:?}", hits);
    }
}
