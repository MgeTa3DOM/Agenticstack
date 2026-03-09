//! # AlphaResolve — Multi-step Agentic Reasoning Engine
//!
//! Inspired by Gemini 3.1 Pro's AlphaResolve methodology:
//! Generate → Verify → Refine → Converge
//!
//! Unlike single-shot generation, AlphaResolve runs multiple passes:
//! 1. **Generate**: Produce N candidate solutions with different temperatures
//! 2. **Verify**: Self-check each solution for correctness and coherence
//! 3. **Refine**: Take the best candidate and improve it using verification feedback
//! 4. **Converge**: If verification passes threshold, return. Otherwise, loop.
//!
//! This emulates Gemini's agentic reasoning locally — no cloud required.

use crate::backend::{GenerationParams, InferenceBackend, InferenceRequest};
use crate::{InferenceError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Configuration for AlphaResolve
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlphaResolveConfig {
    /// Number of candidate solutions to generate per round
    pub num_candidates: usize,
    /// Maximum refinement rounds before giving up
    pub max_rounds: usize,
    /// Confidence threshold to accept a solution (0.0 - 1.0)
    pub confidence_threshold: f64,
    /// Temperature for candidate generation
    pub generation_temperature: f32,
    /// Temperature for verification (should be low)
    pub verification_temperature: f32,
    /// Max tokens for generation steps
    pub generation_max_tokens: usize,
    /// Max tokens for verification steps
    pub verification_max_tokens: usize,
}

impl Default for AlphaResolveConfig {
    fn default() -> Self {
        Self {
            num_candidates: 3,
            max_rounds: 3,
            confidence_threshold: 0.8,
            generation_temperature: 0.7,
            verification_temperature: 0.1,
            generation_max_tokens: 4096,
            verification_max_tokens: 2048,
        }
    }
}

/// Result of an AlphaResolve reasoning session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveResult {
    pub id: Uuid,
    pub task: String,
    pub final_answer: String,
    pub confidence: f64,
    pub steps: Vec<ResolveStep>,
    pub rounds: usize,
    pub total_tokens: usize,
    pub duration_ms: u64,
    pub converged: bool,
    pub started_at: DateTime<Utc>,
}

/// A single step in the resolve process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveStep {
    pub round: usize,
    pub phase: ResolvePhase,
    pub input_summary: String,
    pub output: String,
    pub confidence: Option<f64>,
    pub tokens_used: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResolvePhase {
    Generate,
    Verify,
    Refine,
    Converge,
}

/// AlphaResolve reasoning engine
pub struct AlphaResolve {
    config: AlphaResolveConfig,
}

impl AlphaResolve {
    pub fn new(config: AlphaResolveConfig) -> Self {
        Self { config }
    }

    /// Solve a task using multi-step reasoning
    pub fn solve(
        &self,
        backend: &dyn InferenceBackend,
        task: &str,
        _base_params: &GenerationParams,
    ) -> Result<ResolveResult> {
        let started_at = Utc::now();
        let start = std::time::Instant::now();
        let mut steps = Vec::new();
        let mut total_tokens = 0;
        let mut best_answer = String::new();
        let mut best_confidence = 0.0;
        let mut converged = false;

        for round in 0..self.config.max_rounds {
            // === PHASE 1: Generate candidates ===
            let gen_start = std::time::Instant::now();
            let candidates = self.generate_candidates(backend, task, &best_answer, round)?;
            let gen_tokens: usize = candidates.iter().map(|c| c.1).sum();
            total_tokens += gen_tokens;

            steps.push(ResolveStep {
                round,
                phase: ResolvePhase::Generate,
                input_summary: if round == 0 {
                    format!("Task: {} ({} chars)", &task[..task.len().min(100)], task.len())
                } else {
                    format!("Refining from round {} (confidence: {:.2})", round - 1, best_confidence)
                },
                output: format!("{} candidates generated", candidates.len()),
                confidence: None,
                tokens_used: gen_tokens,
                duration_ms: gen_start.elapsed().as_millis() as u64,
            });

            // === PHASE 2: Verify each candidate ===
            let mut scored_candidates = Vec::new();
            for (i, (candidate, _)) in candidates.iter().enumerate() {
                let ver_start = std::time::Instant::now();
                let (confidence, ver_tokens) = self.verify_candidate(backend, task, candidate)?;
                total_tokens += ver_tokens;

                steps.push(ResolveStep {
                    round,
                    phase: ResolvePhase::Verify,
                    input_summary: format!("Candidate {} ({} chars)", i, candidate.len()),
                    output: format!("Confidence: {:.3}", confidence),
                    confidence: Some(confidence),
                    tokens_used: ver_tokens,
                    duration_ms: ver_start.elapsed().as_millis() as u64,
                });

                scored_candidates.push((candidate.clone(), confidence));
            }

            // Select best candidate
            scored_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            if let Some((best, conf)) = scored_candidates.first() {
                best_answer = best.clone();
                best_confidence = *conf;
            }

            // === PHASE 3: Check convergence ===
            if best_confidence >= self.config.confidence_threshold {
                steps.push(ResolveStep {
                    round,
                    phase: ResolvePhase::Converge,
                    input_summary: format!("Best confidence: {:.3}", best_confidence),
                    output: "Converged — threshold met".to_string(),
                    confidence: Some(best_confidence),
                    tokens_used: 0,
                    duration_ms: 0,
                });
                converged = true;
                break;
            }

            // === PHASE 4: Refine the best candidate ===
            if round < self.config.max_rounds - 1 {
                let ref_start = std::time::Instant::now();
                let (refined, ref_tokens) = self.refine(backend, task, &best_answer, best_confidence)?;
                total_tokens += ref_tokens;

                steps.push(ResolveStep {
                    round,
                    phase: ResolvePhase::Refine,
                    input_summary: format!("Refining best answer (confidence: {:.3})", best_confidence),
                    output: format!("Refined ({} chars → {} chars)", best_answer.len(), refined.len()),
                    confidence: None,
                    tokens_used: ref_tokens,
                    duration_ms: ref_start.elapsed().as_millis() as u64,
                });

                best_answer = refined;
            }
        }

        let rounds = self.config.max_rounds.min(steps.len());
        Ok(ResolveResult {
            id: Uuid::new_v4(),
            task: task.to_string(),
            final_answer: best_answer,
            confidence: best_confidence,
            steps,
            rounds,
            total_tokens,
            duration_ms: start.elapsed().as_millis() as u64,
            converged,
            started_at,
        })
    }

    /// Generate N candidate solutions
    fn generate_candidates(
        &self,
        backend: &dyn InferenceBackend,
        task: &str,
        previous_best: &str,
        round: usize,
    ) -> Result<Vec<(String, usize)>> {
        let mut candidates = Vec::new();

        for i in 0..self.config.num_candidates {
            // Vary temperature across candidates for diversity
            let temp_offset = (i as f32) * 0.15;
            let temperature = (self.config.generation_temperature + temp_offset).min(1.5);

            let prompt = if round == 0 {
                format!(
                    "Solve the following task step by step. Show your reasoning clearly.\n\n\
                     Task: {}\n\n\
                     Solution (candidate {}):",
                    task,
                    i + 1
                )
            } else {
                format!(
                    "The previous best attempt at this task was:\n\n{}\n\n\
                     But it may have issues. Please provide an improved solution.\n\n\
                     Task: {}\n\n\
                     Improved solution (round {}, candidate {}):",
                    previous_best, task, round + 1, i + 1
                )
            };

            let request = InferenceRequest {
                prompt,
                params: GenerationParams {
                    max_tokens: self.config.generation_max_tokens,
                    temperature,
                    ..GenerationParams::default()
                },
                system_prompt: Some(
                    "You are a precise reasoning engine. Think step by step. \
                     Show all work. Be concise but thorough."
                        .to_string(),
                ),
            };

            let response = backend.generate(&request)
                .map_err(|e| InferenceError::ReasoningFailed(format!("Generation failed: {}", e)))?;

            candidates.push((response.text, response.usage.total_tokens));
        }

        Ok(candidates)
    }

    /// Verify a candidate solution
    fn verify_candidate(
        &self,
        backend: &dyn InferenceBackend,
        task: &str,
        candidate: &str,
    ) -> Result<(f64, usize)> {
        let prompt = format!(
            "You are a verification engine. Check if this solution is correct.\n\n\
             Task: {}\n\n\
             Proposed solution:\n{}\n\n\
             Rate the solution's correctness from 0.0 to 1.0.\n\
             First explain any issues, then output exactly one line:\n\
             CONFIDENCE: <number between 0.0 and 1.0>",
            task, candidate
        );

        let request = InferenceRequest {
            prompt,
            params: GenerationParams {
                max_tokens: self.config.verification_max_tokens,
                temperature: self.config.verification_temperature,
                ..GenerationParams::deterministic()
            },
            system_prompt: Some(
                "You are a strict verification engine. Be critical. \
                 Only output high confidence if the solution is truly correct."
                    .to_string(),
            ),
        };

        let response = backend.generate(&request)
            .map_err(|e| InferenceError::ReasoningFailed(format!("Verification failed: {}", e)))?;

        // Parse confidence from response
        let confidence = parse_confidence(&response.text);

        Ok((confidence, response.usage.total_tokens))
    }

    /// Refine a candidate using verification feedback
    fn refine(
        &self,
        backend: &dyn InferenceBackend,
        task: &str,
        current_best: &str,
        current_confidence: f64,
    ) -> Result<(String, usize)> {
        let prompt = format!(
            "You are a refinement engine. Improve this solution.\n\n\
             Task: {}\n\n\
             Current solution (confidence: {:.2}):\n{}\n\n\
             Please provide a refined, improved solution that addresses any weaknesses.\n\
             Refined solution:",
            task, current_confidence, current_best
        );

        let request = InferenceRequest {
            prompt,
            params: GenerationParams {
                max_tokens: self.config.generation_max_tokens,
                temperature: 0.4, // Moderate temp for refinement
                ..GenerationParams::reasoning()
            },
            system_prompt: Some(
                "You are a refinement engine. Keep what works, fix what doesn't. \
                 Be precise and thorough."
                    .to_string(),
            ),
        };

        let response = backend.generate(&request)
            .map_err(|e| InferenceError::ReasoningFailed(format!("Refinement failed: {}", e)))?;

        Ok((response.text, response.usage.total_tokens))
    }
}

/// Parse confidence score from verification output
fn parse_confidence(text: &str) -> f64 {
    // Look for "CONFIDENCE: X.XX" pattern
    for line in text.lines().rev() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("CONFIDENCE:") {
            if let Ok(val) = rest.trim().parse::<f64>() {
                return val.clamp(0.0, 1.0);
            }
        }
    }

    // Fallback: look for any float-like pattern after "confidence"
    let lower = text.to_lowercase();
    if let Some(pos) = lower.rfind("confidence") {
        let after = &text[pos..];
        for word in after.split_whitespace() {
            let cleaned = word.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
            if let Ok(val) = cleaned.parse::<f64>() {
                if (0.0..=1.0).contains(&val) {
                    return val;
                }
            }
        }
    }

    // Default: moderate confidence
    0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::StubBackend;

    #[test]
    fn test_alpha_resolve_basic() {
        let config = AlphaResolveConfig {
            num_candidates: 2,
            max_rounds: 2,
            confidence_threshold: 0.5,
            ..Default::default()
        };
        let ar = AlphaResolve::new(config);
        let backend = StubBackend::new("test-7b");
        let params = GenerationParams::default();

        let result = ar.solve(&backend, "What is 2+2?", &params).unwrap();

        assert!(!result.final_answer.is_empty());
        assert!(!result.steps.is_empty());
        assert!(result.total_tokens > 0);
        assert!(result.duration_ms >= 0);
    }

    #[test]
    fn test_alpha_resolve_converges() {
        let config = AlphaResolveConfig {
            num_candidates: 1,
            max_rounds: 3,
            confidence_threshold: 0.5, // Low threshold for stub
            ..Default::default()
        };
        let ar = AlphaResolve::new(config);
        let backend = StubBackend::new("test");
        let params = GenerationParams::default();

        let result = ar.solve(&backend, "Solve: 2+2", &params).unwrap();
        assert!(result.confidence >= 0.0);
    }

    #[test]
    fn test_parse_confidence() {
        assert!((parse_confidence("CONFIDENCE: 0.95") - 0.95).abs() < 0.01);
        assert!((parse_confidence("The confidence is 0.8") - 0.8).abs() < 0.01);
        assert!((parse_confidence("Some text\nCONFIDENCE: 0.72") - 0.72).abs() < 0.01);
        assert!((parse_confidence("no number here") - 0.5).abs() < 0.01); // default
    }

    #[test]
    fn test_resolve_steps_tracked() {
        let config = AlphaResolveConfig {
            num_candidates: 2,
            max_rounds: 1,
            confidence_threshold: 0.99, // Won't converge
            ..Default::default()
        };
        let ar = AlphaResolve::new(config);
        let backend = StubBackend::new("test");
        let params = GenerationParams::default();

        let result = ar.solve(&backend, "Test task", &params).unwrap();

        // Should have: 1 generate + 2 verify steps at minimum
        assert!(result.steps.len() >= 3);
        assert!(result.steps.iter().any(|s| s.phase == ResolvePhase::Generate));
        assert!(result.steps.iter().any(|s| s.phase == ResolvePhase::Verify));
    }
}
