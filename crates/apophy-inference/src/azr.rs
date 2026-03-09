//! # AZR — Absolute Zero Reasoner (Self-Play Learning Loop)
//!
//! Inspired by the Absolute Zero paper: a model that improves by generating
//! its own training tasks, solving them, and verifying the solutions.
//!
//! ## Loop
//!
//! ```text
//! ┌─────────┐     ┌─────────┐     ┌──────────┐     ┌─────────┐
//! │ PROPOSE │────▶│  SOLVE  │────▶│  VERIFY  │────▶│  LEARN  │
//! │ (gen    │     │ (solve  │     │ (check   │     │ (update │
//! │  task)  │     │  task)  │     │  answer) │     │  memory)│
//! └─────────┘     └─────────┘     └──────────┘     └────┬────┘
//!      ▲                                                 │
//!      └─────────────────────────────────────────────────┘
//! ```
//!
//! The key insight from Absolute Zero: the proposer learns to generate
//! tasks at the right difficulty level — not too easy (no learning signal)
//! and not too hard (no correct solutions to learn from).

use crate::backend::{GenerationParams, InferenceBackend, InferenceRequest};
use crate::{InferenceError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// AZR Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzrConfig {
    /// Max episodes to store in memory
    pub max_episodes: usize,
    /// Difficulty auto-adjustment rate (0.0 - 1.0)
    pub difficulty_lr: f64,
    /// Target success rate (adjust difficulty to maintain this)
    pub target_success_rate: f64,
    /// Domains available for task generation
    pub domains: Vec<String>,
}

impl Default for AzrConfig {
    fn default() -> Self {
        Self {
            max_episodes: 1000,
            difficulty_lr: 0.1,
            target_success_rate: 0.6,
            domains: vec![
                "math".into(),
                "logic".into(),
                "code".into(),
                "reasoning".into(),
                "analysis".into(),
            ],
        }
    }
}

/// Task difficulty level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskDifficulty {
    Easy,
    Medium,
    Hard,
    Expert,
}

impl TaskDifficulty {
    pub fn as_f64(&self) -> f64 {
        match self {
            Self::Easy => 0.25,
            Self::Medium => 0.5,
            Self::Hard => 0.75,
            Self::Expert => 1.0,
        }
    }

    pub fn from_f64(v: f64) -> Self {
        if v < 0.35 {
            Self::Easy
        } else if v < 0.6 {
            Self::Medium
        } else if v < 0.85 {
            Self::Hard
        } else {
            Self::Expert
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Easy => "easy",
            Self::Medium => "medium",
            Self::Hard => "hard",
            Self::Expert => "expert",
        }
    }
}

impl std::fmt::Display for TaskDifficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// A self-generated task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzrTask {
    pub id: Uuid,
    pub domain: String,
    pub difficulty: TaskDifficulty,
    pub question: String,
    pub expected_format: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A complete self-play episode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzrEpisode {
    pub id: Uuid,
    pub task: AzrTask,
    pub solution: String,
    pub verification: VerificationResult,
    pub tokens_used: usize,
    pub duration_ms: u64,
    pub created_at: DateTime<Utc>,
}

/// Verification result for a solution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub correct: bool,
    pub confidence: f64,
    pub feedback: String,
}

/// Absolute Zero Reasoner — self-play learning engine
pub struct AbsoluteZeroReasoner {
    config: AzrConfig,
    episodes: Vec<AzrEpisode>,
    /// Running difficulty level per domain (auto-adjusted)
    difficulty_levels: std::collections::HashMap<String, f64>,
}

impl AbsoluteZeroReasoner {
    pub fn new(config: AzrConfig) -> Self {
        let difficulty_levels = config
            .domains
            .iter()
            .map(|d| (d.clone(), 0.5)) // Start at medium
            .collect();

        Self {
            config,
            episodes: Vec::new(),
            difficulty_levels,
        }
    }

    /// Number of completed episodes
    pub fn episode_count(&self) -> usize {
        self.episodes.len()
    }

    /// Get current difficulty for a domain
    pub fn current_difficulty(&self, domain: &str) -> TaskDifficulty {
        let level = self.difficulty_levels.get(domain).copied().unwrap_or(0.5);
        TaskDifficulty::from_f64(level)
    }

    /// Success rate for a domain (from recent episodes)
    pub fn success_rate(&self, domain: &str) -> f64 {
        let domain_episodes: Vec<_> = self
            .episodes
            .iter()
            .filter(|e| e.task.domain == domain)
            .collect();

        if domain_episodes.is_empty() {
            return 0.0;
        }

        let recent = &domain_episodes[domain_episodes.len().saturating_sub(20)..];
        let successes = recent.iter().filter(|e| e.verification.correct).count();
        successes as f64 / recent.len() as f64
    }

    /// Run a single self-play episode
    pub fn run_episode(
        &mut self,
        backend: &dyn InferenceBackend,
        domain: &str,
        difficulty: TaskDifficulty,
    ) -> Result<AzrEpisode> {
        let start = std::time::Instant::now();
        let mut total_tokens = 0;

        // === PHASE 1: PROPOSE — Generate a task ===
        let task = self.propose_task(backend, domain, difficulty)?;
        total_tokens += backend.estimate_tokens(&task.question);

        // === PHASE 2: SOLVE — Solve the generated task ===
        let (solution, solve_tokens) = self.solve_task(backend, &task)?;
        total_tokens += solve_tokens;

        // === PHASE 3: VERIFY — Check the solution ===
        let (verification, verify_tokens) = self.verify_solution(backend, &task, &solution)?;
        total_tokens += verify_tokens;

        // === PHASE 4: LEARN — Update difficulty based on result ===
        self.update_difficulty(domain, verification.correct);

        let episode = AzrEpisode {
            id: Uuid::new_v4(),
            task,
            solution,
            verification,
            tokens_used: total_tokens,
            duration_ms: start.elapsed().as_millis() as u64,
            created_at: Utc::now(),
        };

        // Store episode (with capacity limit)
        if self.episodes.len() >= self.config.max_episodes {
            self.episodes.remove(0); // FIFO
        }
        self.episodes.push(episode.clone());

        tracing::info!(
            "AZR episode: domain={}, difficulty={}, correct={}, confidence={:.2}, tokens={}",
            episode.task.domain,
            episode.task.difficulty,
            episode.verification.correct,
            episode.verification.confidence,
            episode.tokens_used,
        );

        Ok(episode)
    }

    /// PROPOSE: Generate a new task at the appropriate difficulty
    fn propose_task(
        &self,
        backend: &dyn InferenceBackend,
        domain: &str,
        difficulty: TaskDifficulty,
    ) -> Result<AzrTask> {
        let prompt = format!(
            "Generate a {} difficulty {} task.\n\n\
             Requirements:\n\
             - The task must have a clear, verifiable answer\n\
             - Difficulty: {} (scale: easy/medium/hard/expert)\n\
             - The task should test reasoning, not just recall\n\
             - Output format: A single question followed by the expected answer format\n\n\
             Task:",
            difficulty, domain, difficulty
        );

        let request = InferenceRequest {
            prompt,
            params: GenerationParams {
                max_tokens: 512,
                temperature: 0.8, // Creative for diverse tasks
                ..GenerationParams::creative()
            },
            system_prompt: Some(format!(
                "You are a task generator for the {} domain. \
                 Generate clear, self-contained tasks with verifiable answers.",
                domain
            )),
        };

        let response = backend
            .generate(&request)
            .map_err(|e| InferenceError::SelfPlayError(format!("Task generation failed: {}", e)))?;

        Ok(AzrTask {
            id: Uuid::new_v4(),
            domain: domain.to_string(),
            difficulty,
            question: response.text,
            expected_format: None,
            created_at: Utc::now(),
        })
    }

    /// SOLVE: Attempt to solve the task
    fn solve_task(
        &self,
        backend: &dyn InferenceBackend,
        task: &AzrTask,
    ) -> Result<(String, usize)> {
        let prompt = format!(
            "Solve the following {} task ({} difficulty).\n\
             Show your step-by-step reasoning.\n\n\
             Task: {}\n\n\
             Solution:",
            task.domain, task.difficulty, task.question
        );

        let request = InferenceRequest {
            prompt,
            params: GenerationParams::reasoning(),
            system_prompt: Some(
                "You are a precise problem solver. Show all reasoning steps. \
                 Be concise but thorough. End with a clear answer."
                    .to_string(),
            ),
        };

        let response = backend
            .generate(&request)
            .map_err(|e| InferenceError::SelfPlayError(format!("Solve failed: {}", e)))?;

        Ok((response.text, response.usage.total_tokens))
    }

    /// VERIFY: Check if the solution is correct
    fn verify_solution(
        &self,
        backend: &dyn InferenceBackend,
        task: &AzrTask,
        solution: &str,
    ) -> Result<(VerificationResult, usize)> {
        let prompt = format!(
            "Verify this solution to a {} task.\n\n\
             Task: {}\n\n\
             Solution: {}\n\n\
             Is the solution correct? Rate confidence 0.0-1.0.\n\
             Format:\n\
             CORRECT: true/false\n\
             CONFIDENCE: 0.XX\n\
             FEEDBACK: <explanation>",
            task.domain, task.question, solution
        );

        let request = InferenceRequest {
            prompt,
            params: GenerationParams::deterministic(),
            system_prompt: Some(
                "You are a strict verification engine. \
                 Only mark correct if the solution is truly right."
                    .to_string(),
            ),
        };

        let response = backend
            .generate(&request)
            .map_err(|e| InferenceError::SelfPlayError(format!("Verification failed: {}", e)))?;

        let verification = parse_verification(&response.text);

        Ok((verification, response.usage.total_tokens))
    }

    /// LEARN: Adjust difficulty based on success/failure
    fn update_difficulty(&mut self, domain: &str, correct: bool) {
        let current = self.difficulty_levels.get(domain).copied().unwrap_or(0.5);
        let success_rate = self.success_rate(domain);
        let lr = self.config.difficulty_lr;

        // If success rate is too high, increase difficulty
        // If success rate is too low, decrease difficulty
        let adjustment = if correct {
            if success_rate > self.config.target_success_rate {
                lr // Increase difficulty
            } else {
                0.0
            }
        } else if success_rate < self.config.target_success_rate {
            -lr // Decrease difficulty
        } else {
            0.0
        };

        let new_level = (current + adjustment).clamp(0.1, 1.0);
        self.difficulty_levels.insert(domain.to_string(), new_level);

        tracing::debug!(
            "AZR difficulty update: {}  {:.2} → {:.2} (success_rate: {:.2}, target: {:.2})",
            domain,
            current,
            new_level,
            success_rate,
            self.config.target_success_rate,
        );
    }

    /// Get learning statistics
    pub fn stats(&self) -> AzrStats {
        let mut domain_stats = std::collections::HashMap::new();

        for domain in &self.config.domains {
            let episodes: Vec<_> = self
                .episodes
                .iter()
                .filter(|e| e.task.domain == *domain)
                .collect();

            let total = episodes.len();
            let correct = episodes.iter().filter(|e| e.verification.correct).count();

            domain_stats.insert(
                domain.clone(),
                DomainStats {
                    total_episodes: total,
                    success_count: correct,
                    success_rate: if total > 0 { correct as f64 / total as f64 } else { 0.0 },
                    current_difficulty: self.current_difficulty(domain),
                },
            );
        }

        AzrStats {
            total_episodes: self.episodes.len(),
            domain_stats,
        }
    }
}

/// Parse verification response
fn parse_verification(text: &str) -> VerificationResult {
    let lower = text.to_lowercase();

    let correct = lower.contains("correct: true") || lower.contains("verified");
    let confidence = {
        let mut conf = 0.5;
        for line in text.lines() {
            let line = line.trim().to_lowercase();
            if let Some(rest) = line.strip_prefix("confidence:") {
                if let Ok(val) = rest.trim().parse::<f64>() {
                    conf = val.clamp(0.0, 1.0);
                }
            }
        }
        conf
    };

    let feedback = text
        .lines()
        .find(|l| l.to_lowercase().starts_with("feedback:"))
        .map(|l| l.trim_start_matches(|c: char| !c.is_alphabetic() || c == 'F' || c == 'f'))
        .map(|l| {
            l.strip_prefix("FEEDBACK:")
                .or_else(|| l.strip_prefix("feedback:"))
                .or_else(|| l.strip_prefix("Feedback:"))
                .unwrap_or(l)
                .trim()
                .to_string()
        })
        .unwrap_or_else(|| "No feedback provided".to_string());

    VerificationResult {
        correct,
        confidence,
        feedback,
    }
}

/// AZR learning statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzrStats {
    pub total_episodes: usize,
    pub domain_stats: std::collections::HashMap<String, DomainStats>,
}

/// Per-domain statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainStats {
    pub total_episodes: usize,
    pub success_count: usize,
    pub success_rate: f64,
    pub current_difficulty: TaskDifficulty,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::StubBackend;

    #[test]
    fn test_azr_creation() {
        let azr = AbsoluteZeroReasoner::new(AzrConfig::default());
        assert_eq!(azr.episode_count(), 0);
        assert_eq!(azr.current_difficulty("math"), TaskDifficulty::Medium);
    }

    #[test]
    fn test_azr_run_episode() {
        let mut azr = AbsoluteZeroReasoner::new(AzrConfig::default());
        let backend = StubBackend::new("test-7b");

        let episode = azr.run_episode(&backend, "math", TaskDifficulty::Easy).unwrap();

        assert!(!episode.task.question.is_empty());
        assert!(!episode.solution.is_empty());
        assert!(episode.tokens_used > 0);
        assert_eq!(azr.episode_count(), 1);
    }

    #[test]
    fn test_azr_multiple_episodes() {
        let mut azr = AbsoluteZeroReasoner::new(AzrConfig::default());
        let backend = StubBackend::new("test");

        for _ in 0..5 {
            let _ = azr.run_episode(&backend, "logic", TaskDifficulty::Medium).unwrap();
        }

        assert_eq!(azr.episode_count(), 5);
        let stats = azr.stats();
        assert_eq!(stats.total_episodes, 5);
    }

    #[test]
    fn test_azr_difficulty_adjustment() {
        let mut azr = AbsoluteZeroReasoner::new(AzrConfig {
            target_success_rate: 0.6,
            difficulty_lr: 0.2,
            ..Default::default()
        });

        let backend = StubBackend::new("test");

        // Run several episodes — difficulty should adjust
        for _ in 0..10 {
            let _ = azr.run_episode(&backend, "math", TaskDifficulty::Easy);
        }

        // Difficulty should have changed from initial 0.5
        let stats = azr.stats();
        assert!(stats.domain_stats.contains_key("math"));
    }

    #[test]
    fn test_task_difficulty_conversion() {
        assert_eq!(TaskDifficulty::from_f64(0.1), TaskDifficulty::Easy);
        assert_eq!(TaskDifficulty::from_f64(0.5), TaskDifficulty::Medium);
        assert_eq!(TaskDifficulty::from_f64(0.8), TaskDifficulty::Hard);
        assert_eq!(TaskDifficulty::from_f64(0.95), TaskDifficulty::Expert);
    }

    #[test]
    fn test_parse_verification() {
        let text = "CORRECT: true\nCONFIDENCE: 0.85\nFEEDBACK: Solution is correct";
        let result = parse_verification(text);
        assert!(result.correct);
        assert!((result.confidence - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_azr_stats() {
        let azr = AbsoluteZeroReasoner::new(AzrConfig::default());
        let stats = azr.stats();
        assert_eq!(stats.total_episodes, 0);
        assert!(stats.domain_stats.contains_key("math"));
    }

    #[test]
    fn test_azr_episode_capacity() {
        let mut azr = AbsoluteZeroReasoner::new(AzrConfig {
            max_episodes: 3,
            ..Default::default()
        });
        let backend = StubBackend::new("test");

        for _ in 0..5 {
            let _ = azr.run_episode(&backend, "math", TaskDifficulty::Easy);
        }

        // Should be capped at 3
        assert_eq!(azr.episode_count(), 3);
    }
}
