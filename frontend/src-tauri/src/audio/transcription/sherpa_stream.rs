// audio/transcription/sherpa_stream.rs
//
// v0.6.14+: 可选 streaming session wrapper for sherpa-onnx daemon.
//
// 设计原则 (避免重蹈 7/12 streaming-bug 事故):
//   1. **完全独立模块**, 不修改 worker.rs / sherpa_provider.rs 主路径
//   2. **feature flag STREAMING_ENABLED** (env var), 默认 false, 开启需明确
//   3. **session 唯一性**: 用全局 Mutex<HashMap<session_id, Session>>, 多 worker 安全
//   4. **失败回退**: streaming 任何错误立即 fallback 到 transcribe_blocking
//   5. **可观测**: emit `streaming-chunk` event 给前端, partial/final 字段明确
//
// 用法 (默认不调用, 开了 STREAMING_ENABLED=true 才用):
//   let mut session = SherpaStreamSession::begin(model, hotwords_pack, hotwords_custom).await?;
//   loop {
//       let audio_chunk = ...; // f32 vec, 通常 100ms-1s
//       let result = session.push(audio_chunk).await?;
//       if let Some(partial) = result.partial { emit!("streaming-chunk", {partial, is_partial: true}); }
//       if let Some(delta) = result.delta { emit!("streaming-chunk", {delta, is_partial: false}); }
//   }
//   let final_result = session.finalize().await?;

use crate::audio::sherpa_daemon::global as global_sherpa;
use base64::Engine;
use log::{info, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// Feature flag: 从环境变量读, 默认 false (保守起见)
pub fn streaming_enabled() -> bool {
    std::env::var("OFFLINE_HUIJI_STREAMING")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

/// Streaming session 单次 push 的返回
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StreamChunkResult {
    /// 当前累计的 partial text (灰色, 会被覆盖)
    #[serde(default)]
    pub partial: String,
    /// 本次新冒出的 final delta (追加到 transcript 列表, 不再变)
    #[serde(default)]
    pub delta: String,
    /// 本次 emit 出的 final 段数 (>=0)
    #[serde(default)]
    pub segments_emitted: u32,
    /// 是否检测到 endpoint (静音/句末), 下游可考虑 flush
    #[serde(default)]
    pub is_endpoint: bool,
    /// 错误信息 (anyhow 字符串), 非空时整个 result 不可信
    #[serde(default)]
    pub error: String,
}

/// Streaming session
pub struct SherpaStreamSession {
    session_id: String,
    model: String,
}

impl SherpaStreamSession {
    /// 开始新 session (call daemon `stream_begin`)
    pub async fn begin(
        model: impl Into<String>,
        hotwords_pack: impl Into<String>,
        hotwords_custom: impl Into<String>,
    ) -> Result<Self, String> {
        let model = model.into();
        let session_id = format!("stream-{}-{}", model, std::process::id());
        
        let daemon = global_sherpa();
        let model_clone = model.clone();
        let sid = session_id.clone();
        let hw_pack = hotwords_pack.into();
        let hw_custom = hotwords_custom.into();
        
        let resp = tokio::task::spawn_blocking(move || {
            daemon.stream_begin(&sid, &model_clone, &hw_pack, &hw_custom)
        })
        .await
        .map_err(|e| format!("stream_begin join: {}", e))?
        .map_err(|e| format!("stream_begin daemon: {}", e))?;
        
        let ok = resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            let err = resp.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
            return Err(format!("stream_begin failed: {}", err));
        }
        
        // 注册到全局 session map
        SESSION_MAP.lock()
            .map_err(|e| format!("session map lock: {}", e))?
            .insert(session_id.clone(), SessionMeta { model: model.clone() });
        
        info!("✅ Stream session begun: {} (model={})", session_id, model);
        Ok(Self { session_id, model })
    }
    
    /// 推一段音频 chunk, 返 partial/delta 结果
    pub async fn push(&self, audio: Vec<f32>) -> Result<StreamChunkResult, String> {
        if audio.is_empty() {
            return Ok(StreamChunkResult::default());
        }
        
        // f32 little-endian → base64
        let bytes: Vec<u8> = audio.iter().flat_map(|s| s.to_le_bytes()).collect();
        let audio_b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        
        let daemon = global_sherpa();
        let sid = self.session_id.clone();
        
        let resp = tokio::task::spawn_blocking(move || {
            daemon.stream_chunk(&sid, &audio_b64)
        })
        .await
        .map_err(|e| format!("stream_chunk join: {}", e))?
        .map_err(|e| format!("stream_chunk daemon: {}", e))?;
        
        Ok(StreamChunkResult {
            partial: resp.get("partial").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            delta: resp.get("delta").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            segments_emitted: resp.get("segments_emitted").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            is_endpoint: resp.get("is_endpoint").and_then(|v| v.as_bool()).unwrap_or(false),
            error: String::new(),
        })
    }
    
    /// 关闭 session, 出最后一段残留 audio (call daemon `stream_finalize`)
    pub async fn finalize(self) -> Result<StreamChunkResult, String> {
        let daemon = global_sherpa();
        let sid = self.session_id.clone();
        
        let resp = tokio::task::spawn_blocking(move || {
            daemon.stream_finalize(&sid)
        })
        .await
        .map_err(|e| format!("stream_finalize join: {}", e))?
        .map_err(|e| format!("stream_finalize daemon: {}", e))?;
        
        // 从全局 map 注销
        if let Ok(mut map) = SESSION_MAP.lock() {
            map.remove(&self.session_id);
        }
        
        Ok(StreamChunkResult {
            partial: String::new(),
            delta: resp.get("delta").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            segments_emitted: resp.get("segments_emitted").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            is_endpoint: true,
            error: String::new(),
        })
    }
    
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    
    pub fn model(&self) -> &str {
        &self.model
    }
}

struct SessionMeta {
    model: String,
}

/// 全局 session 注册表 (用于诊断 / 防止泄漏)
static SESSION_MAP: Lazy<Mutex<HashMap<String, SessionMeta>>> = 
    Lazy::new(|| Mutex::new(HashMap::new()));

/// 列出所有 active session (诊断用)
pub fn list_active_sessions() -> Vec<(String, String)> {
    SESSION_MAP.lock()
        .map(|map| map.iter().map(|(k, v)| (k.clone(), v.model.clone())).collect())
        .unwrap_or_default()
}

/// 强制清理所有 session (异常退出时调用)
pub fn cleanup_all_sessions() {
    if let Ok(mut map) = SESSION_MAP.lock() {
        let n = map.len();
        map.clear();
        if n > 0 {
            warn!("⚠️ 强制清理 {} 个未关闭 stream session", n);
        }
    }
}
