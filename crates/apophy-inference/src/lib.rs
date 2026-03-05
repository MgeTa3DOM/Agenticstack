//! # Apophy Inference — Sovereign Local LLM Brain
//!
//! Local-first inference engine with advanced reasoning capabilities.
//! No cloud dependency. No telemetry. Your data stays on your machine.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │                 Reasoning Layer                   │
//! │  ┌──────────┐ ┌─────┐ ┌──────┐ ┌───────────┐   │
//! │  │AlphaResolve│ │ AZR │ │CTM-C │ │  Ashoka   │   │
//! │  │multi-step  │ │self-│ │edge  │ │ autolearn │   │
//! │  │verify+refine│ │play │ │compress│ │ feedback │   │
//! │  └──────┬─────┘ └──┬──┘ └──┬───┘ └────┬──────┘   │
//! │         └──────────┼───────┼───────────┘          │
//! │                    ▼                              │
//! │           ┌────────────────┐                     │
//! │           │ InferenceBackend│                     │
//! │           └───────┬────────┘                     │
//! │         ┌─────────┼──────────┐                   │
//! │         ▼         ▼          ▼                   │
//! │    ┌────────┐ ┌───────┐ ┌────────┐              │
//! │    │  GGUF  │ │ ONNX  │ │ Stub   │              │
//! │    │llama.cpp│ │(future)│ │(testing)│              │
//! │    └────────┘ └───────┘ └────────┘              │
//! └─────────────────────────────────────────────────┘
//! ```

pub mod backend;
pub mod alpha_resolve;
pub mod azr;
pub mod ctmc;
pub mod ashoka;
pub mod toon;

pub use backend::{
    InferenceBackend, InferenceRequest, InferenceResponse,
    GgufBackend, GgufConfig, StubBackend,
    GenerationParams, TokenUsage,
};
pub use alpha_resolve::{AlphaResolve, AlphaResolveConfig, ResolveResult, ResolveStep};
pub use azr::{AbsoluteZeroReasoner, AzrConfig, AzrEpisode, AzrTask, TaskDifficulty};
pub use ctmc::{CtmcCompressor, CtmcConfig, CompressedThought, CompressionStats};
pub use ashoka::{AshokaLearner, AshokaConfig, FeedbackEntry, LearningSignal, PromptEvolution};
pub use toon::{ToonCodec, ToonConfig, ToonEncoded, ToonStats};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum InferenceError {
    #[error("Model not loaded: {0}")]
    ModelNotLoaded(String),

    #[error("Generation failed: {0}")]
    GenerationFailed(String),

    #[error("Model file not found: {0}")]
    ModelNotFound(String),

    #[error("Insufficient memory: need {needed_mb}MB, have {available_mb}MB")]
    InsufficientMemory { needed_mb: u64, available_mb: u64 },

    #[error("Context overflow: {tokens} tokens exceeds limit {limit}")]
    ContextOverflow { tokens: usize, limit: usize },

    #[error("Reasoning failed: {0}")]
    ReasoningFailed(String),

    #[error("Self-play error: {0}")]
    SelfPlayError(String),

    #[error("Compression error: {0}")]
    CompressionError(String),

    #[error("Learning error: {0}")]
    LearningError(String),
}

pub type Result<T> = std::result::Result<T, InferenceError>;

/// Sovereign inference engine combining all reasoning capabilities.
/// This is the main entry point for the "brain" of Apophy.
pub struct SovereignBrain {
    pub backend: Box<dyn InferenceBackend + Send + Sync>,
    pub alpha_resolve: AlphaResolve,
    pub azr: AbsoluteZeroReasoner,
    pub ctmc: CtmcCompressor,
    pub ashoka: AshokaLearner,
    pub hardware: apophy_universal::HardwareInfo,
}

impl SovereignBrain {
    /// Create a new SovereignBrain with the given backend and hardware info.
    pub fn new(
        backend: Box<dyn InferenceBackend + Send + Sync>,
        hardware: apophy_universal::HardwareInfo,
    ) -> Self {
        Self {
            backend,
            alpha_resolve: AlphaResolve::new(AlphaResolveConfig::default()),
            azr: AbsoluteZeroReasoner::new(AzrConfig::default()),
            ctmc: CtmcCompressor::new(CtmcConfig::default()),
            ashoka: AshokaLearner::new(AshokaConfig::default()),
            hardware,
        }
    }

    /// Simple generation — direct pass-through to backend
    pub fn generate(&self, prompt: &str, params: &GenerationParams) -> Result<InferenceResponse> {
        self.backend.generate(&InferenceRequest {
            prompt: prompt.to_string(),
            params: params.clone(),
            system_prompt: None,
        })
    }

    /// AlphaResolve reasoning — multi-step generate → verify → refine
    pub fn reason(&self, task: &str, params: &GenerationParams) -> Result<ResolveResult> {
        self.alpha_resolve.solve(self.backend.as_ref(), task, params)
    }

    /// AZR self-play — generate task → solve → verify → learn
    pub fn self_play(&mut self, domain: &str, difficulty: TaskDifficulty) -> Result<AzrEpisode> {
        self.azr.run_episode(self.backend.as_ref(), domain, difficulty)
    }

    /// CTM-C compress a chain-of-thought for edge deployment
    pub fn compress_thought(&self, thought_chain: &str) -> Result<CompressedThought> {
        self.ctmc.compress(self.backend.as_ref(), thought_chain)
    }

    /// Ashoka autolearn — record feedback and evolve prompts
    pub fn record_feedback(&mut self, feedback: FeedbackEntry) -> Result<Option<PromptEvolution>> {
        self.ashoka.record(feedback)
    }

    /// Get brain status summary
    pub fn status(&self) -> BrainStatus {
        BrainStatus {
            backend_name: self.backend.name().to_string(),
            model_loaded: self.backend.is_loaded(),
            model_name: self.backend.model_name().map(|s| s.to_string()),
            hardware_backend: format!("{}", self.hardware.backend),
            memory_mb: self.hardware.memory_mb,
            cpu_cores: self.hardware.cpu_cores,
            azr_episodes: self.azr.episode_count(),
            ashoka_feedback_count: self.ashoka.feedback_count(),
            ashoka_evolutions: self.ashoka.evolution_count(),
        }
    }
}

/// Brain status for API/CLI display
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrainStatus {
    pub backend_name: String,
    pub model_loaded: bool,
    pub model_name: Option<String>,
    pub hardware_backend: String,
    pub memory_mb: u64,
    pub cpu_cores: usize,
    pub azr_episodes: usize,
    pub ashoka_feedback_count: usize,
    pub ashoka_evolutions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sovereign_brain_creation() {
        let stub = StubBackend::new("test-model");
        let hw = apophy_universal::detect_hardware();
        let brain = SovereignBrain::new(Box::new(stub), hw);

        let status = brain.status();
        assert_eq!(status.backend_name, "stub");
        assert!(status.model_loaded);
        assert_eq!(status.model_name, Some("test-model".to_string()));
    }

    #[test]
    fn test_sovereign_brain_generate() {
        let stub = StubBackend::new("test-model");
        let hw = apophy_universal::detect_hardware();
        let brain = SovereignBrain::new(Box::new(stub), hw);

        let params = GenerationParams::default();
        let response = brain.generate("Hello, world!", &params).unwrap();
        assert!(!response.text.is_empty());
        assert!(response.usage.total_tokens > 0);
    }

    #[test]
    fn test_sovereign_brain_reason() {
        let stub = StubBackend::new("test-model");
        let hw = apophy_universal::detect_hardware();
        let brain = SovereignBrain::new(Box::new(stub), hw);

        let params = GenerationParams::default();
        let result = brain.reason("What is 2+2?", &params).unwrap();
        assert!(!result.final_answer.is_empty());
        assert!(!result.steps.is_empty());
    }

    #[test]
    fn test_sovereign_brain_self_play() {
        let stub = StubBackend::new("test-model");
        let hw = apophy_universal::detect_hardware();
        let mut brain = SovereignBrain::new(Box::new(stub), hw);

        let episode = brain.self_play("math", TaskDifficulty::Easy).unwrap();
        assert!(!episode.task.question.is_empty());
        assert!(!episode.solution.is_empty());
    }

    #[test]
    fn test_sovereign_brain_compress() {
        let stub = StubBackend::new("test-model");
        let hw = apophy_universal::detect_hardware();
        let brain = SovereignBrain::new(Box::new(stub), hw);

        let thought = "Step 1: Read the problem. Step 2: Identify variables. Step 3: Solve. Answer: 42.";
        let compressed = brain.compress_thought(thought).unwrap();
        // With stub backend, compressed may not be smaller, but should exist
        assert!(!compressed.compressed.is_empty());
        assert!(compressed.stats.original_tokens > 0);
    }

    #[test]
    fn test_sovereign_brain_feedback() {
        let stub = StubBackend::new("test-model");
        let hw = apophy_universal::detect_hardware();
        let mut brain = SovereignBrain::new(Box::new(stub), hw);

        let feedback = FeedbackEntry {
            prompt_id: "test-1".to_string(),
            original_prompt: "Solve: 2+2".to_string(),
            response: "4".to_string(),
            signal: LearningSignal::Positive { score: 0.95 },
            context: None,
        };

        let _ = brain.record_feedback(feedback).unwrap();
        assert_eq!(brain.ashoka.feedback_count(), 1);
    }
}
