use reqwest::{header, Client};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::info;

const REQUEST_TIMEOUT_DURATION: Duration = Duration::from_secs(300);

// Generic structure for OpenAI-compatible API chat messages
#[derive(Debug, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

// Generic structure for OpenAI-compatible API chat requests
#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

pub type StreamSink = Arc<dyn Fn(&str) + Send + Sync>;

#[derive(Deserialize, Debug)]
struct StreamChatResponse {
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize, Debug)]
struct StreamChoice {
    delta: StreamDelta,
}

#[derive(Deserialize, Debug)]
struct StreamDelta {
    #[serde(default)]
    content: String,
}

// Generic structure for OpenAI-compatible API chat responses
#[derive(Deserialize, Debug)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
pub struct Choice {
    pub message: MessageContent,
}

#[derive(Deserialize, Debug)]
pub struct MessageContent {
    pub content: String,
}

// Claude-specific request structure
#[derive(Debug, Serialize)]
pub struct ClaudeRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: String,
    pub messages: Vec<ChatMessage>,
}

// Claude-specific response structure
#[derive(Deserialize, Debug)]
pub struct ClaudeChatResponse {
    pub content: Vec<ClaudeChatContent>,
}

#[derive(Deserialize, Debug)]
pub struct ClaudeChatContent {
    pub text: String,
}

/// LLM Provider enumeration for multi-provider support
#[derive(Debug, Clone, PartialEq)]
pub enum LLMProvider {
    OpenAI,
    Claude,
    Groq,
    Ollama,
    OpenRouter,
    BuiltInAI,
    CustomOpenAI,
}

impl LLMProvider {
    /// Parse provider from string (case-insensitive)
    pub fn from_str(s: &str) -> Result<Self, String> {
        // 离线会记: 只接受 Ollama (本地) 与 BuiltInAI (本地内置)。
        // OpenAI/Claude/Groq/OpenRouter/CustomOpenAI 全部禁用,云端 API 不允许调用。
        match s.to_lowercase().as_str() {
            "ollama" => Ok(Self::Ollama),
            "builtin-ai" | "local-llama" | "localllama" => Ok(Self::BuiltInAI),
            _ => Err(format!(
                "离线会记仅支持本地 LLM (Ollama / BuiltInAI),云端 provider '{}' 已禁用",
                s
            )),
        }
    }
}

/// Generates a summary using the specified LLM provider
///
/// # Arguments
/// * `client` - Reqwest HTTP client (reused for performance)
/// * `provider` - The LLM provider to use
/// * `model_name` - The specific model to use (e.g., "gpt-4", "claude-3-opus")
/// * `api_key` - API key for the provider (not needed for Ollama)
/// * `system_prompt` - System instructions for the LLM
/// * `user_prompt` - User query/content to process
/// * `ollama_endpoint` - Optional custom Ollama endpoint (defaults to localhost:11434)
/// * `custom_openai_endpoint` - Optional custom OpenAI-compatible endpoint
/// * `max_tokens` - Optional max tokens (for CustomOpenAI provider)
/// * `temperature` - Optional temperature (for CustomOpenAI provider)
/// * `top_p` - Optional top_p (for CustomOpenAI provider)
/// * `app_data_dir` - Optional app data directory (for BuiltInAI provider)
/// * `cancellation_token` - Optional token to cancel the request
///
/// # Returns
/// The generated summary text or an error message
pub async fn generate_summary(
    client: &Client,
    provider: &LLMProvider,
    model_name: &str,
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
    ollama_endpoint: Option<&str>,
    custom_openai_endpoint: Option<&str>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    app_data_dir: Option<&PathBuf>,
    cancellation_token: Option<&CancellationToken>,
) -> Result<String, String> {
    generate_summary_with_stream(
        client,
        provider,
        model_name,
        api_key,
        system_prompt,
        user_prompt,
        ollama_endpoint,
        custom_openai_endpoint,
        max_tokens,
        temperature,
        top_p,
        app_data_dir,
        cancellation_token,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn generate_summary_with_stream(
    client: &Client,
    provider: &LLMProvider,
    model_name: &str,
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
    ollama_endpoint: Option<&str>,
    custom_openai_endpoint: Option<&str>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    app_data_dir: Option<&PathBuf>,
    cancellation_token: Option<&CancellationToken>,
    stream_sink: Option<StreamSink>,
) -> Result<String, String> {
    // Check if cancelled before starting
    if let Some(token) = cancellation_token {
        if token.is_cancelled() {
            return Err("Summary generation was cancelled".to_string());
        }
    }

    // 离线会记硬守卫: 只允许 Ollama / BuiltInAI 调用 LLM
    if !matches!(provider, LLMProvider::Ollama | LLMProvider::BuiltInAI) {
        return Err("离线会记仅支持本地 LLM (Ollama / BuiltInAI),云端 provider 不可用".to_string());
    }

    // Handle BuiltInAI provider separately (uses local sidecar, no HTTP API)
    if provider == &LLMProvider::BuiltInAI {
        let app_data_dir = app_data_dir
            .ok_or_else(|| "app_data_dir is required for BuiltInAI provider".to_string())?;

        return crate::summary::summary_engine::client::generate_with_builtin_stream(
            app_data_dir,
            model_name,
            system_prompt,
            user_prompt,
            cancellation_token,
            stream_sink,
        )
        .await
        .map_err(|e| e.to_string());
    }

    let (api_url, mut headers) = match provider {
        LLMProvider::OpenAI => (
            "https://api.openai.com/v1/chat/completions".to_string(),
            header::HeaderMap::new(),
        ),
        LLMProvider::Groq => (
            "https://api.groq.com/openai/v1/chat/completions".to_string(),
            header::HeaderMap::new(),
        ),
        LLMProvider::OpenRouter => (
            "https://openrouter.ai/api/v1/chat/completions".to_string(),
            header::HeaderMap::new(),
        ),
        LLMProvider::Ollama => {
            let host = ollama_endpoint
                .map(|s| s.to_string())
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            // §111: 用 Ollama 原生 /api/chat + think:false. OpenAI-compat wrapper 对 qwen3.5 等
            //       thinking 模型会返回空 content + 800 token thinking 耗时 30+s
            (
                format!("{}/api/chat", host),
                header::HeaderMap::new(),
            )
        }
        LLMProvider::CustomOpenAI => {
            let endpoint = custom_openai_endpoint
                .ok_or_else(|| "Custom OpenAI endpoint not configured".to_string())?;
            (
                format!("{}/chat/completions", endpoint.trim_end_matches('/')),
                header::HeaderMap::new(),
            )
        }
        LLMProvider::Claude => {
            let mut header_map = header::HeaderMap::new();
            header_map.insert(
                "x-api-key",
                api_key
                    .parse()
                    .map_err(|_| "Invalid API key format".to_string())?,
            );
            header_map.insert(
                "anthropic-version",
                "2023-06-01"
                    .parse()
                    .map_err(|_| "Invalid anthropic version".to_string())?,
            );
            ("https://api.anthropic.com/v1/messages".to_string(), header_map)
        }
        LLMProvider::BuiltInAI => {
            // This case is handled earlier with early returns
            unreachable!("BuiltInAI is handled before this match statement")
        }
    };

    // Add authorization header for non-Claude providers
    if provider != &LLMProvider::Claude {
        headers.insert(
            header::AUTHORIZATION,
            format!("Bearer {}", api_key)
                .parse()
                .map_err(|_| "Invalid authorization header".to_string())?,
        );
    }
    headers.insert(
        header::CONTENT_TYPE,
        "application/json"
            .parse()
            .map_err(|_| "Invalid content type".to_string())?,
    );

    // Build request body based on provider
    let request_body = if provider == &LLMProvider::Ollama {
        // §111: Ollama 原生 /api/chat schema — 必加 think:false 关掉 qwen3.5 thinking mode
        let mut ollama_messages = vec![];
        if !system_prompt.is_empty() {
            ollama_messages.push(serde_json::json!({"role": "system", "content": system_prompt}));
        }
        ollama_messages.push(serde_json::json!({"role": "user", "content": user_prompt}));
        let mut body = serde_json::json!({
            "model": model_name,
            "messages": ollama_messages,
            "stream": stream_sink.is_some(),
            "think": false,  // 关掉 thinking mode (qwen3.5:2b 等 thinking 模型必需)
        });
        if let Some(mt) = max_tokens {
            body["options"] = serde_json::json!({"num_predict": mt});
        }
        body
    } else if provider != &LLMProvider::Claude {
        // For CustomOpenAI, apply optional parameters if provided
        let (max_tokens_val, temperature_val, top_p_val) = if provider == &LLMProvider::CustomOpenAI {
            (max_tokens, temperature, top_p)
        } else {
            (None, None, None)
        };

        serde_json::json!(ChatRequest {
            model: model_name.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                }
            ],
            max_tokens: max_tokens_val,
            temperature: temperature_val,
            top_p: top_p_val,
            stream: Some(stream_sink.is_some()),
        })
    } else {
        serde_json::json!(ClaudeRequest {
            system: system_prompt.to_string(),
            model: model_name.to_string(),
            max_tokens: 2048,
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: user_prompt.to_string(),
            }]
        })
    };

    info!("🐞 LLM Request to {}: model={}", provider_name(provider), model_name);

    // Send request with timeout and cancellation support
    let request_future = client
        .post(api_url)
        .headers(headers)
        .json(&request_body)
        .timeout(REQUEST_TIMEOUT_DURATION)
        .send();

    // Use tokio::select to race between cancellation and request completion
    let response = if let Some(token) = cancellation_token {
        tokio::select! {
            result = request_future => {
                result.map_err(|e| {
                    if e.is_timeout() {
                        format!("LLM request timed out after {} seconds", REQUEST_TIMEOUT_DURATION.as_secs())
                    } else {
                        format!("Failed to send request to LLM: {}", e)
                    }
                })?
            }
            _ = token.cancelled() => {
                return Err("Summary generation was cancelled".to_string());
            }
        }
    } else {
        request_future.await.map_err(|e| {
            if e.is_timeout() {
                format!("LLM request timed out after {} seconds", REQUEST_TIMEOUT_DURATION.as_secs())
            } else {
                format!("Failed to send request to LLM: {}", e)
            }
        })?
    };

    if !response.status().is_success() {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("LLM API request failed: {}", error_body));
    }

    // Parse response based on provider
    if provider == &LLMProvider::Ollama {
        // §111: Ollama 原生 /api/chat 返回 {message: {content, ...}, done, ...}
        if stream_sink.is_some() {
            // streaming path
            let sink = stream_sink.expect("stream sink checked above");
            let mut bytes_stream = response.bytes_stream();
            let mut pending = String::new();
            let mut output = String::new();
            while let Some(next) = bytes_stream.next().await {
                if cancellation_token.is_some_and(CancellationToken::is_cancelled) {
                    return Err("Summary generation was cancelled".to_string());
                }
                let bytes = next.map_err(|e| format!("Failed to read streaming response: {}", e))?;
                pending.push_str(&String::from_utf8_lossy(&bytes));
                while let Some(newline) = pending.find('\n') {
                    let line = pending[..newline].trim().to_string();
                    pending.drain(..=newline);
                    if let Some(content) = parse_ollama_stream_line(&line)? {
                        if !content.is_empty() {
                            output.push_str(&content);
                            sink(&content);
                        }
                    }
                }
            }
            if !pending.trim().is_empty() {
                if let Some(content) = parse_ollama_stream_line(pending.trim())? {
                    if !content.is_empty() {
                        output.push_str(&content);
                        sink(&content);
                    }
                }
            }
            return Ok(output.trim().to_string());
        }
        // non-streaming: Ollama 原生 schema
        let ollama_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;
        info!("🐞 LLM Response received from Ollama");
        let content = ollama_response
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .trim();
        return Ok(content.to_string());
    } else if provider == &LLMProvider::Ollama && stream_sink.is_some() {
        let sink = stream_sink.expect("stream sink checked above");
        let mut bytes_stream = response.bytes_stream();
        let mut pending = String::new();
        let mut output = String::new();

        while let Some(next) = bytes_stream.next().await {
            if cancellation_token.is_some_and(CancellationToken::is_cancelled) {
                return Err("Summary generation was cancelled".to_string());
            }
            let bytes = next.map_err(|e| format!("Failed to read streaming response: {}", e))?;
            pending.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(newline) = pending.find('\n') {
                let line = pending[..newline].trim().to_string();
                pending.drain(..=newline);
                if let Some(content) = parse_stream_line(&line)? {
                    output.push_str(&content);
                    sink(&content);
                }
            }
        }
        if !pending.trim().is_empty() {
            if let Some(content) = parse_stream_line(pending.trim())? {
                output.push_str(&content);
                sink(&content);
            }
        }
        Ok(output.trim().to_string())
    } else if provider == &LLMProvider::Claude {
        let chat_response = response
            .json::<ClaudeChatResponse>()
            .await
            .map_err(|e| format!("Failed to parse LLM response: {}", e))?;

        info!("🐞 LLM Response received from Claude");

        let content = chat_response
            .content
            .get(0)
            .ok_or("No content in LLM response")?
            .text
            .trim();
        Ok(content.to_string())
    } else {
        let chat_response = response
            .json::<ChatResponse>()
            .await
            .map_err(|e| format!("Failed to parse LLM response: {}", e))?;

        info!("🐞 LLM Response received from {}", provider_name(provider));

        let content = chat_response
            .choices
            .get(0)
            .ok_or("No content in LLM response")?
            .message
            .content
            .trim();
        Ok(content.to_string())
    }
}

fn parse_stream_line(line: &str) -> Result<Option<String>, String> {
    let payload = line.strip_prefix("data:").map(str::trim).unwrap_or(line);
    if payload.is_empty() || payload == "[DONE]" {
        return Ok(None);
    }
    let response: StreamChatResponse = serde_json::from_str(payload)
        .map_err(|e| format!("Failed to parse streaming response: {}", e))?;
    Ok(response.choices.first().map(|choice| choice.delta.content.clone()).filter(|value| !value.is_empty()))
}

/// §111: Ollama 原生 /api/chat streaming 解析 (JSON Lines, 每行一个 {message:{content:"..."}, done})
fn parse_ollama_stream_line(line: &str) -> Result<Option<String>, String> {
    if line.is_empty() {
        return Ok(None);
    }
    let value: serde_json::Value = serde_json::from_str(line)
        .map_err(|e| format!("Failed to parse Ollama stream line: {}", e))?;
    Ok(value.get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string()))
}

#[cfg(test)]
mod streaming_tests {
    use super::*;

    #[test]
    fn parses_openai_compatible_stream_line() {
        let value = parse_stream_line(r#"data: {"choices":[{"delta":{"content":"你好"}}]}"#).unwrap();
        assert_eq!(value.as_deref(), Some("你好"));
        assert_eq!(parse_stream_line("data: [DONE]").unwrap(), None);
    }
}

/// Helper function to get provider name for logging
fn provider_name(provider: &LLMProvider) -> &str {
    match provider {
        LLMProvider::OpenAI => "OpenAI",
        LLMProvider::Claude => "Claude",
        LLMProvider::Groq => "Groq",
        LLMProvider::Ollama => "Ollama",
        LLMProvider::BuiltInAI => "Built-in AI",
        LLMProvider::OpenRouter => "OpenRouter",
        LLMProvider::CustomOpenAI => "Custom OpenAI",
    }
}
