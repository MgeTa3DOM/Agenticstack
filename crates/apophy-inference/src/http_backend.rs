//! # HTTP Backend — Connect to Ollama, LM Studio, vLLM, or any OpenAI-compatible API
//!
//! This is the bridge from "stub responses" to real inference.
//! Supports any server that speaks the OpenAI `/v1/chat/completions` protocol.
//!
//! ## Supported servers
//! - Ollama (default: http://localhost:11434/v1/chat/completions)
//! - LM Studio (default: http://localhost:1234/v1/chat/completions)
//! - vLLM (default: http://localhost:8000/v1/chat/completions)
//! - llama.cpp server (default: http://localhost:8080/v1/chat/completions)
//! - Any OpenAI-compatible endpoint

use crate::{InferenceError, Result};
use crate::backend::{
    FinishReason, InferenceBackend, InferenceRequest, InferenceResponse,
    TokenUsage,
};
use serde::{Deserialize, Serialize};

/// Configuration for the HTTP backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpBackendConfig {
    /// Base URL of the inference server
    pub base_url: String,
    /// Model name to request
    pub model: String,
    /// Optional API key
    pub api_key: Option<String>,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for HttpBackendConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model: "llama3.2".to_string(),
            api_key: None,
            timeout_secs: 120,
        }
    }
}

impl HttpBackendConfig {
    /// Ollama preset (most common local setup)
    pub fn ollama(model: impl Into<String>) -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model: model.into(),
            api_key: None,
            timeout_secs: 120,
        }
    }

    /// LM Studio preset
    pub fn lm_studio(model: impl Into<String>) -> Self {
        Self {
            base_url: "http://localhost:1234".to_string(),
            model: model.into(),
            api_key: None,
            timeout_secs: 120,
        }
    }

    /// vLLM preset
    pub fn vllm(model: impl Into<String>) -> Self {
        Self {
            base_url: "http://localhost:8000".to_string(),
            model: model.into(),
            api_key: None,
            timeout_secs: 120,
        }
    }

    /// Generic OpenAI-compatible endpoint
    pub fn openai_compatible(base_url: impl Into<String>, model: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            api_key,
            timeout_secs: 120,
        }
    }
}

/// HTTP Backend — connects to any OpenAI-compatible inference server.
/// This is the real inference backend that replaces StubBackend.
pub struct HttpBackend {
    config: HttpBackendConfig,
    client: reqwest::Client,
}

impl HttpBackend {
    pub fn new(config: HttpBackendConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| InferenceError::GenerationFailed(format!("HTTP client error: {}", e)))?;

        tracing::info!(
            "HttpBackend initialized: {} (model: {})",
            config.base_url,
            config.model
        );

        Ok(Self { config, client })
    }

    /// Check if the server is reachable
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/v1/models", self.config.base_url);
        self.client.get(&url).send().await.is_ok()
    }

    fn build_url(&self) -> String {
        format!("{}/v1/chat/completions", self.config.base_url)
    }

    fn generate_sync(&self, request: &InferenceRequest) -> Result<InferenceResponse> {
        let start = std::time::Instant::now();

        let mut messages = Vec::new();

        if let Some(ref sys) = request.system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: sys.clone(),
            });
        }

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: request.prompt.clone(),
        });

        let body = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: Some(request.params.max_tokens),
            temperature: Some(request.params.temperature),
            top_p: Some(request.params.top_p),
            stop: if request.params.stop.is_empty() {
                None
            } else {
                Some(request.params.stop.clone())
            },
        };

        let url = self.build_url();
        let mut req_builder = self.client.post(&url).json(&body);

        if let Some(ref key) = self.config.api_key {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", key));
        }

        // Use tokio block_in_place to run async from sync context
        let response = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                req_builder.send().await
            })
        })
        .map_err(|e| InferenceError::GenerationFailed(format!("HTTP request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    response.text().await.unwrap_or_default()
                })
            });
            return Err(InferenceError::GenerationFailed(format!(
                "Server returned {}: {}",
                status, error_body
            )));
        }

        let resp_body: ChatCompletionResponse = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                response.json().await
            })
        })
        .map_err(|e| InferenceError::GenerationFailed(format!("Failed to parse response: {}", e)))?;

        let elapsed = start.elapsed().as_millis() as u64;

        let text = resp_body
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        let finish_reason = resp_body
            .choices
            .first()
            .and_then(|c| c.finish_reason.as_deref())
            .map(|r| match r {
                "stop" => FinishReason::Stop,
                "length" => FinishReason::Length,
                _ => FinishReason::EndOfSequence,
            })
            .unwrap_or(FinishReason::EndOfSequence);

        let usage = resp_body.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }).unwrap_or_else(|| {
            let pt = request.prompt.len() / 4;
            let ct = text.len() / 4;
            TokenUsage {
                prompt_tokens: pt,
                completion_tokens: ct,
                total_tokens: pt + ct,
            }
        });

        Ok(InferenceResponse {
            text,
            usage,
            finish_reason,
            generation_ms: elapsed,
        })
    }
}

impl InferenceBackend for HttpBackend {
    fn name(&self) -> &str {
        "http"
    }

    fn is_loaded(&self) -> bool {
        true
    }

    fn model_name(&self) -> Option<&str> {
        Some(&self.config.model)
    }

    fn generate(&self, request: &InferenceRequest) -> Result<InferenceResponse> {
        self.generate_sync(request)
    }
}

// === OpenAI-compatible protocol types ===

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
    usage: Option<UsageResponse>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsageResponse {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_presets() {
        let ollama = HttpBackendConfig::ollama("llama3.2");
        assert!(ollama.base_url.contains("11434"));
        assert_eq!(ollama.model, "llama3.2");

        let lm = HttpBackendConfig::lm_studio("mistral");
        assert!(lm.base_url.contains("1234"));

        let vllm = HttpBackendConfig::vllm("qwen2");
        assert!(vllm.base_url.contains("8000"));
    }

    #[test]
    fn test_backend_creation() {
        let config = HttpBackendConfig::ollama("test");
        let backend = HttpBackend::new(config).unwrap();
        assert_eq!(backend.name(), "http");
        assert!(backend.is_loaded());
        assert_eq!(backend.model_name(), Some("test"));
    }

    #[test]
    fn test_url_building() {
        let config = HttpBackendConfig::ollama("test");
        let backend = HttpBackend::new(config).unwrap();
        assert_eq!(backend.build_url(), "http://localhost:11434/v1/chat/completions");
    }
}
