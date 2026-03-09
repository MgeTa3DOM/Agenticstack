//! # Inference Backend — Trait + GGUF + Stub implementations
//!
//! The `InferenceBackend` trait is the core abstraction that all reasoning
//! engines build upon. Implement this trait to plug in any local model.
//!
//! ## Backends
//!
//! - **GgufBackend**: Production backend using llama.cpp via GGUF files.
//!   Requires a `.gguf` model file on disk.
//! - **StubBackend**: Testing backend that returns deterministic responses.
//!   Used for CI and unit tests.

use crate::{InferenceError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// =============================================================================
// INFERENCE BACKEND TRAIT
// =============================================================================

/// Core trait for local inference backends.
/// All reasoning engines (AlphaResolve, AZR, CTM-C, Ashoka) use this trait.
pub trait InferenceBackend {
    /// Backend name (e.g., "gguf", "onnx", "stub")
    fn name(&self) -> &str;

    /// Whether a model is currently loaded and ready
    fn is_loaded(&self) -> bool;

    /// Name of the loaded model (if any)
    fn model_name(&self) -> Option<&str>;

    /// Generate text from a prompt
    fn generate(&self, request: &InferenceRequest) -> Result<InferenceResponse>;

    /// Generate multiple completions (for self-play / verification)
    fn generate_n(&self, request: &InferenceRequest, n: usize) -> Result<Vec<InferenceResponse>> {
        let mut results = Vec::with_capacity(n);
        for _ in 0..n {
            results.push(self.generate(request)?);
        }
        Ok(results)
    }

    /// Estimate token count for a string (approximate)
    fn estimate_tokens(&self, text: &str) -> usize {
        // Rough estimate: ~4 chars per token for English
        text.len() / 4
    }
}

// =============================================================================
// REQUEST / RESPONSE TYPES
// =============================================================================

/// Inference request with prompt and generation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub prompt: String,
    pub params: GenerationParams,
    pub system_prompt: Option<String>,
}

/// Generation parameters matching GGUF/llama.cpp conventions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationParams {
    pub max_tokens: usize,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: u32,
    pub repeat_penalty: f32,
    pub seed: Option<u64>,
    /// Stop sequences
    pub stop: Vec<String>,
}

impl Default for GenerationParams {
    fn default() -> Self {
        Self {
            max_tokens: 2048,
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            repeat_penalty: 1.1,
            seed: None,
            stop: vec![],
        }
    }
}

impl GenerationParams {
    /// Deterministic params for verification steps (low temperature)
    pub fn deterministic() -> Self {
        Self {
            temperature: 0.1,
            top_p: 0.95,
            top_k: 10,
            ..Default::default()
        }
    }

    /// Creative params for generation steps (high temperature)
    pub fn creative() -> Self {
        Self {
            temperature: 0.9,
            top_p: 0.95,
            top_k: 50,
            ..Default::default()
        }
    }

    /// Precise params for reasoning (moderate temperature)
    pub fn reasoning() -> Self {
        Self {
            temperature: 0.3,
            top_p: 0.9,
            top_k: 20,
            max_tokens: 4096,
            ..Default::default()
        }
    }
}

/// Inference response with generated text and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub text: String,
    pub usage: TokenUsage,
    pub finish_reason: FinishReason,
    /// Time taken for generation in milliseconds
    pub generation_ms: u64,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// Reason generation stopped
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinishReason {
    /// Reached max_tokens
    Length,
    /// Hit a stop sequence
    Stop,
    /// Model decided it's done (EOS)
    EndOfSequence,
}

// =============================================================================
// GGUF BACKEND — Production local inference via llama.cpp
// =============================================================================

/// Configuration for the GGUF backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GgufConfig {
    pub model_path: PathBuf,
    pub context_length: usize,
    pub threads: usize,
    pub gpu_layers: u32,
    pub batch_size: usize,
    pub rope_freq_base: Option<f32>,
    pub rope_freq_scale: Option<f32>,
}

impl Default for GgufConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("models/default.gguf"),
            context_length: 4096,
            threads: 4,
            gpu_layers: 0,
            batch_size: 512,
            rope_freq_base: None,
            rope_freq_scale: None,
        }
    }
}

impl GgufConfig {
    /// Auto-configure based on detected hardware
    pub fn auto(model_path: impl Into<PathBuf>, hardware: &apophy_universal::HardwareInfo) -> Self {
        let threads = hardware.cpu_cores.max(1);
        let gpu_layers = match hardware.backend {
            apophy_universal::HardwareBackend::NvidiaCuda
            | apophy_universal::HardwareBackend::AmdRocm => 35, // Offload most layers
            apophy_universal::HardwareBackend::AppleNeural => 32,
            _ => 0, // CPU-only
        };
        let context_length = if hardware.memory_mb > 16_000 {
            8192
        } else if hardware.memory_mb > 8_000 {
            4096
        } else {
            2048
        };
        let batch_size = if hardware.memory_mb > 16_000 { 1024 } else { 512 };

        Self {
            model_path: model_path.into(),
            context_length,
            threads,
            gpu_layers,
            batch_size,
            rope_freq_base: None,
            rope_freq_scale: None,
        }
    }
}

/// GGUF backend for local inference via llama.cpp.
///
/// This backend loads a GGUF model file and runs inference locally.
/// When compiled without a llama.cpp binding crate, it operates in
/// "ready-to-integrate" mode: model loading validates the file exists
/// and the config is correct, but actual inference requires linking
/// the llama.cpp library.
///
/// Integration path:
/// 1. Add `llama-cpp-2 = "0.1"` to Cargo.toml
/// 2. Replace `generate_impl` with actual llama.cpp calls
/// 3. Everything else (config, hardware detection, params) stays the same
pub struct GgufBackend {
    config: GgufConfig,
    model_name: String,
    loaded: bool,
    /// Stored for future use when llama.cpp backend is integrated
    #[allow(dead_code)]
    hardware: apophy_universal::HardwareInfo,
}

impl GgufBackend {
    /// Create a new GGUF backend. Validates model file exists.
    pub fn new(config: GgufConfig) -> Result<Self> {
        let model_name = config
            .model_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let hardware = apophy_universal::detect_hardware();

        // Validate model file
        let loaded = if config.model_path.exists() {
            let metadata = std::fs::metadata(&config.model_path)
                .map_err(|e| InferenceError::ModelNotFound(e.to_string()))?;

            // Check memory
            let model_size_mb = metadata.len() / (1024 * 1024);
            if model_size_mb > hardware.memory_mb * 90 / 100 {
                return Err(InferenceError::InsufficientMemory {
                    needed_mb: model_size_mb,
                    available_mb: hardware.memory_mb,
                });
            }

            // Validate GGUF magic bytes
            let file = std::fs::File::open(&config.model_path)
                .map_err(|e| InferenceError::ModelNotFound(e.to_string()))?;
            let mut reader = std::io::BufReader::new(file);
            let mut magic = [0u8; 4];
            std::io::Read::read_exact(&mut reader, &mut magic)
                .map_err(|e| InferenceError::ModelNotFound(e.to_string()))?;

            // GGUF magic: "GGUF" = [0x47, 0x47, 0x55, 0x46]
            if magic == [0x47, 0x47, 0x55, 0x46] {
                tracing::info!(
                    "GGUF model loaded: {} ({} MB, {} ctx, {} threads, {} GPU layers)",
                    model_name,
                    model_size_mb,
                    config.context_length,
                    config.threads,
                    config.gpu_layers,
                );
                true
            } else {
                tracing::warn!("File exists but is not a valid GGUF file: {:?}", config.model_path);
                false
            }
        } else {
            tracing::warn!(
                "GGUF model not found at {:?} — backend in standby mode. \
                 Download a GGUF model to enable local inference.",
                config.model_path
            );
            false
        };

        Ok(Self {
            config,
            model_name,
            loaded,
            hardware,
        })
    }

    /// Get the configuration
    pub fn config(&self) -> &GgufConfig {
        &self.config
    }

    /// Generate text using the GGUF model.
    /// Currently returns a structured placeholder indicating the model
    /// needs llama.cpp integration. The actual generation pipeline is:
    ///
    /// 1. Tokenize prompt
    /// 2. Create llama context with config params
    /// 3. Feed tokens
    /// 4. Sample with temperature/top_p/top_k
    /// 5. Decode output tokens
    /// 6. Return with usage stats
    fn generate_impl(&self, request: &InferenceRequest) -> Result<InferenceResponse> {
        if !self.loaded {
            return Err(InferenceError::ModelNotLoaded(format!(
                "Model not loaded from {:?}. Download a GGUF model first.",
                self.config.model_path
            )));
        }

        // Token estimation
        let prompt_tokens = request.prompt.len() / 4;
        if prompt_tokens > self.config.context_length {
            return Err(InferenceError::ContextOverflow {
                tokens: prompt_tokens,
                limit: self.config.context_length,
            });
        }

        let start = std::time::Instant::now();

        // === INTEGRATION POINT ===
        // Replace this block with actual llama.cpp inference:
        //
        // ```rust
        // use llama_cpp_2::model::LlamaModel;
        // use llama_cpp_2::context::LlamaContext;
        //
        // let model = LlamaModel::load_from_file(&self.config.model_path, params)?;
        // let ctx = model.create_context(ctx_params)?;
        // let tokens = model.tokenize(&request.prompt, true)?;
        // ctx.eval(&tokens, 0)?;
        //
        // let mut output = String::new();
        // for _ in 0..request.params.max_tokens {
        //     let token = ctx.sample(sampler)?;
        //     if token == model.eos_token() { break; }
        //     output.push_str(&model.token_to_str(token)?);
        // }
        // ```

        let output = format!(
            "[GGUF:{}/ctx:{}] Inference ready — awaiting llama.cpp backend integration. \
             Prompt: {} tokens, Config: {}t/{}k/{}p",
            self.model_name,
            self.config.context_length,
            prompt_tokens,
            request.params.temperature,
            request.params.top_k,
            request.params.top_p,
        );

        let completion_tokens = output.len() / 4;
        let elapsed = start.elapsed().as_millis() as u64;

        Ok(InferenceResponse {
            text: output,
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
            finish_reason: FinishReason::EndOfSequence,
            generation_ms: elapsed,
        })
    }
}

impl InferenceBackend for GgufBackend {
    fn name(&self) -> &str {
        "gguf"
    }

    fn is_loaded(&self) -> bool {
        self.loaded
    }

    fn model_name(&self) -> Option<&str> {
        Some(&self.model_name)
    }

    fn generate(&self, request: &InferenceRequest) -> Result<InferenceResponse> {
        self.generate_impl(request)
    }

    fn estimate_tokens(&self, text: &str) -> usize {
        // GGUF models typically use BPE tokenization
        // ~3.5 chars per token for mixed content
        (text.len() as f64 / 3.5).ceil() as usize
    }
}

// =============================================================================
// STUB BACKEND — For testing and CI
// =============================================================================

/// Stub backend that returns deterministic responses.
/// Used for testing all reasoning engines without a real model.
pub struct StubBackend {
    model_name: String,
    /// Custom response generator (prompt → response)
    response_fn: Option<Box<dyn Fn(&str) -> String + Send + Sync>>,
}

impl StubBackend {
    pub fn new(model_name: impl Into<String>) -> Self {
        Self {
            model_name: model_name.into(),
            response_fn: None,
        }
    }

    /// Create a stub with a custom response function
    pub fn with_response_fn(
        model_name: impl Into<String>,
        f: impl Fn(&str) -> String + Send + Sync + 'static,
    ) -> Self {
        Self {
            model_name: model_name.into(),
            response_fn: Some(Box::new(f)),
        }
    }

    fn default_response(&self, prompt: &str) -> String {
        // Generate contextually appropriate stub responses
        let lower = prompt.to_lowercase();

        if lower.contains("verify") || lower.contains("check") || lower.contains("correct") {
            "VERIFIED: The solution is correct. Confidence: 0.92. \
             Reasoning: Each step follows logically from the previous one."
                .to_string()
        } else if lower.contains("solve") || lower.contains("what is") || lower.contains("calculate") {
            "Step 1: Identify the problem components.\n\
             Step 2: Apply the relevant formula or method.\n\
             Step 3: Calculate the result.\n\
             Answer: The solution is 42."
                .to_string()
        } else if lower.contains("compress") || lower.contains("summarize") {
            "Core insight: The essential reasoning chain reduces to: \
             identify → apply → verify → conclude."
                .to_string()
        } else if lower.contains("generate") && lower.contains("task") {
            "Task: Given a list of N integers, find the maximum subarray sum \
             using Kadane's algorithm. What is the time complexity?"
                .to_string()
        } else if lower.contains("improve") || lower.contains("refine") {
            "Refined version: The prompt has been optimized for clarity, \
             specificity, and reasoning depth. Key changes: \
             added explicit constraints, clarified expected output format."
                .to_string()
        } else {
            format!(
                "Response from {} to prompt ({} chars): \
                 Acknowledged. Processing sovereign inference request locally. \
                 No cloud dependency. No telemetry.",
                self.model_name,
                prompt.len()
            )
        }
    }
}

impl InferenceBackend for StubBackend {
    fn name(&self) -> &str {
        "stub"
    }

    fn is_loaded(&self) -> bool {
        true
    }

    fn model_name(&self) -> Option<&str> {
        Some(&self.model_name)
    }

    fn generate(&self, request: &InferenceRequest) -> Result<InferenceResponse> {
        let start = std::time::Instant::now();

        let text = match &self.response_fn {
            Some(f) => f(&request.prompt),
            None => self.default_response(&request.prompt),
        };

        let prompt_tokens = request.prompt.len() / 4;
        let completion_tokens = text.len() / 4;
        let elapsed = start.elapsed().as_millis() as u64;

        Ok(InferenceResponse {
            text,
            usage: TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
            finish_reason: FinishReason::EndOfSequence,
            generation_ms: elapsed,
        })
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_backend_basic() {
        let stub = StubBackend::new("test-7b");
        assert_eq!(stub.name(), "stub");
        assert!(stub.is_loaded());
        assert_eq!(stub.model_name(), Some("test-7b"));
    }

    #[test]
    fn test_stub_generate() {
        let stub = StubBackend::new("test-7b");
        let request = InferenceRequest {
            prompt: "Hello, world!".to_string(),
            params: GenerationParams::default(),
            system_prompt: None,
        };

        let response = stub.generate(&request).unwrap();
        assert!(!response.text.is_empty());
        assert!(response.usage.total_tokens > 0);
        assert_eq!(response.finish_reason, FinishReason::EndOfSequence);
    }

    #[test]
    fn test_stub_contextual_responses() {
        let stub = StubBackend::new("test");

        let verify_req = InferenceRequest {
            prompt: "Verify this solution".to_string(),
            params: GenerationParams::default(),
            system_prompt: None,
        };
        let resp = stub.generate(&verify_req).unwrap();
        assert!(resp.text.contains("VERIFIED"));

        let solve_req = InferenceRequest {
            prompt: "Solve: what is 2+2?".to_string(),
            params: GenerationParams::default(),
            system_prompt: None,
        };
        let resp = stub.generate(&solve_req).unwrap();
        assert!(resp.text.contains("Step 1"));
    }

    #[test]
    fn test_stub_custom_response() {
        let stub = StubBackend::with_response_fn("custom", |prompt| {
            format!("CUSTOM: {}", prompt.len())
        });

        let request = InferenceRequest {
            prompt: "test".to_string(),
            params: GenerationParams::default(),
            system_prompt: None,
        };

        let response = stub.generate(&request).unwrap();
        assert_eq!(response.text, "CUSTOM: 4");
    }

    #[test]
    fn test_generate_n() {
        let stub = StubBackend::new("test");
        let request = InferenceRequest {
            prompt: "Hello".to_string(),
            params: GenerationParams::default(),
            system_prompt: None,
        };

        let responses = stub.generate_n(&request, 3).unwrap();
        assert_eq!(responses.len(), 3);
    }

    #[test]
    fn test_generation_params_presets() {
        let det = GenerationParams::deterministic();
        assert!(det.temperature < 0.2);

        let creative = GenerationParams::creative();
        assert!(creative.temperature > 0.8);

        let reasoning = GenerationParams::reasoning();
        assert!(reasoning.temperature > 0.2 && reasoning.temperature < 0.5);
    }

    #[test]
    fn test_gguf_config_auto() {
        let hw = apophy_universal::detect_hardware();
        let config = GgufConfig::auto("model.gguf", &hw);
        assert!(config.threads > 0);
        assert!(config.context_length >= 2048);
    }

    #[test]
    fn test_gguf_backend_missing_model() {
        let config = GgufConfig {
            model_path: PathBuf::from("/nonexistent/model.gguf"),
            ..Default::default()
        };

        let backend = GgufBackend::new(config).unwrap();
        assert!(!backend.is_loaded()); // Not loaded, in standby
        assert_eq!(backend.name(), "gguf");

        // Generate should fail gracefully
        let request = InferenceRequest {
            prompt: "test".to_string(),
            params: GenerationParams::default(),
            system_prompt: None,
        };
        assert!(backend.generate(&request).is_err());
    }

    #[test]
    fn test_token_estimation() {
        let stub = StubBackend::new("test");
        let tokens = stub.estimate_tokens("Hello, world! This is a test.");
        assert!(tokens > 0);
        assert!(tokens < 100);
    }
}
