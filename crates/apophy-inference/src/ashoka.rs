//! # Ashoka — Autonomous Prompt Evolution & Autolearn
//!
//! Self-improving prompt system inspired by evolutionary learning.
//! Named after Emperor Ashoka who transformed through self-reflection.
//!
//! ## Mechanism
//!
//! ```text
//! ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
//! │ OBSERVE  │────▶│ EVALUATE │────▶│  EVOLVE  │────▶│  SELECT  │
//! │ (collect │     │ (score   │     │ (mutate  │     │ (keep    │
//! │ feedback)│     │ prompts) │     │ prompts) │     │ best)    │
//! └──────────┘     └──────────┘     └──────────┘     └──────────┘
//! ```
//!
//! 1. **Observe**: Collect feedback on prompt-response pairs
//! 2. **Evaluate**: Score prompts by aggregated feedback signals
//! 3. **Evolve**: Mutate underperforming prompts using successful patterns
//! 4. **Select**: Keep the best-performing prompt variants
//!
//! This creates a closed-loop system where prompts improve automatically
//! based on real usage patterns — no human prompt engineering needed.

use crate::{InferenceError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Ashoka Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AshokaConfig {
    /// Minimum feedback entries before triggering evolution
    pub evolution_threshold: usize,
    /// Maximum prompt variants to maintain per ID
    pub max_variants: usize,
    /// Selection pressure (0.0 = keep all, 1.0 = keep only best)
    pub selection_pressure: f64,
    /// Mutation rate (probability of modifying a prompt element)
    pub mutation_rate: f64,
    /// Maximum feedback entries to store
    pub max_feedback: usize,
}

impl Default for AshokaConfig {
    fn default() -> Self {
        Self {
            evolution_threshold: 10,
            max_variants: 5,
            selection_pressure: 0.7,
            mutation_rate: 0.3,
            max_feedback: 10_000,
        }
    }
}

/// A feedback entry for a prompt-response pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEntry {
    pub prompt_id: String,
    pub original_prompt: String,
    pub response: String,
    pub signal: LearningSignal,
    pub context: Option<String>,
}

/// Learning signal — what happened after the response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LearningSignal {
    /// User accepted / positive outcome
    Positive { score: f64 },
    /// User rejected / negative outcome
    Negative { score: f64, reason: Option<String> },
    /// Verification passed/failed
    Verification { passed: bool, confidence: f64 },
    /// Task completed successfully
    TaskComplete { steps: usize, efficiency: f64 },
    /// Task failed
    TaskFailed { error: String, step: Option<usize> },
}

impl LearningSignal {
    /// Convert signal to a normalized score (-1.0 to 1.0)
    pub fn to_score(&self) -> f64 {
        match self {
            Self::Positive { score } => *score,
            Self::Negative { score, .. } => -score,
            Self::Verification { passed, confidence } => {
                if *passed { *confidence } else { -confidence }
            }
            Self::TaskComplete { efficiency, .. } => *efficiency,
            Self::TaskFailed { .. } => -0.5,
        }
    }
}

/// A prompt evolution event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptEvolution {
    pub id: Uuid,
    pub prompt_id: String,
    pub generation: usize,
    pub original: String,
    pub evolved: String,
    pub parent_score: f64,
    pub mutations: Vec<Mutation>,
    pub created_at: DateTime<Utc>,
}

/// A specific mutation applied during evolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mutation {
    pub mutation_type: MutationType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MutationType {
    /// Add more specificity
    Specify,
    /// Add constraints
    Constrain,
    /// Restructure the prompt
    Restructure,
    /// Add examples
    Exemplify,
    /// Simplify language
    Simplify,
    /// Add chain-of-thought instruction
    ChainOfThought,
}

/// Internal tracking for a prompt's performance
#[derive(Debug, Clone)]
struct PromptTracker {
    prompt_id: String,
    current_prompt: String,
    generation: usize,
    scores: Vec<f64>,
    feedback_count: usize,
}

impl PromptTracker {
    fn average_score(&self) -> f64 {
        if self.scores.is_empty() {
            return 0.0;
        }
        self.scores.iter().sum::<f64>() / self.scores.len() as f64
    }

    fn recent_score(&self, window: usize) -> f64 {
        let recent: Vec<_> = self.scores.iter().rev().take(window).collect();
        if recent.is_empty() {
            return 0.0;
        }
        recent.iter().copied().sum::<f64>() / recent.len() as f64
    }
}

/// Ashoka Learner — autonomous prompt evolution engine
pub struct AshokaLearner {
    config: AshokaConfig,
    feedback: Vec<FeedbackEntry>,
    trackers: HashMap<String, PromptTracker>,
    evolutions: Vec<PromptEvolution>,
}

impl AshokaLearner {
    pub fn new(config: AshokaConfig) -> Self {
        Self {
            config,
            feedback: Vec::new(),
            trackers: HashMap::new(),
            evolutions: Vec::new(),
        }
    }

    /// Total feedback entries recorded
    pub fn feedback_count(&self) -> usize {
        self.feedback.len()
    }

    /// Total evolution events
    pub fn evolution_count(&self) -> usize {
        self.evolutions.len()
    }

    /// Record feedback and potentially trigger evolution
    pub fn record(&mut self, entry: FeedbackEntry) -> Result<Option<PromptEvolution>> {
        let score = entry.signal.to_score();
        let prompt_id = entry.prompt_id.clone();
        let original_prompt = entry.original_prompt.clone();

        // Update tracker
        let tracker = self
            .trackers
            .entry(prompt_id.clone())
            .or_insert_with(|| PromptTracker {
                prompt_id: prompt_id.clone(),
                current_prompt: original_prompt.clone(),
                generation: 0,
                scores: Vec::new(),
                feedback_count: 0,
            });

        tracker.scores.push(score);
        tracker.feedback_count += 1;

        // Store feedback (with capacity limit)
        if self.feedback.len() >= self.config.max_feedback {
            self.feedback.remove(0);
        }
        self.feedback.push(entry);

        // Check if evolution should trigger
        if tracker.feedback_count >= self.config.evolution_threshold
            && tracker.recent_score(self.config.evolution_threshold) < self.config.selection_pressure
        {
            // Trigger evolution
            let evolution = self.evolve_prompt(&prompt_id)?;
            return Ok(Some(evolution));
        }

        Ok(None)
    }

    /// Evolve a prompt based on accumulated feedback
    fn evolve_prompt(&mut self, prompt_id: &str) -> Result<PromptEvolution> {
        let tracker = self
            .trackers
            .get(prompt_id)
            .ok_or_else(|| InferenceError::LearningError("Prompt not tracked".into()))?;

        let parent_score = tracker.average_score();
        let current_prompt = tracker.current_prompt.clone();
        let generation = tracker.generation;

        // Determine mutations based on feedback patterns
        let mutations = self.determine_mutations(prompt_id);

        // Apply mutations to create evolved prompt
        let evolved = self.apply_mutations(&current_prompt, &mutations);

        let evolution = PromptEvolution {
            id: Uuid::new_v4(),
            prompt_id: prompt_id.to_string(),
            generation: generation + 1,
            original: current_prompt,
            evolved: evolved.clone(),
            parent_score,
            mutations,
            created_at: Utc::now(),
        };

        // Update tracker
        if let Some(tracker) = self.trackers.get_mut(prompt_id) {
            tracker.current_prompt = evolved;
            tracker.generation += 1;
            tracker.scores.clear(); // Reset scores for new generation
            tracker.feedback_count = 0;
        }

        self.evolutions.push(evolution.clone());

        tracing::info!(
            "Ashoka evolution: prompt={}, gen={} → {}, parent_score={:.3}, mutations={}",
            prompt_id,
            generation,
            generation + 1,
            parent_score,
            evolution.mutations.len(),
        );

        Ok(evolution)
    }

    /// Determine which mutations to apply based on feedback patterns
    fn determine_mutations(&self, prompt_id: &str) -> Vec<Mutation> {
        let mut mutations = Vec::new();

        let negative_feedback: Vec<_> = self
            .feedback
            .iter()
            .filter(|f| f.prompt_id == prompt_id)
            .filter(|f| f.signal.to_score() < 0.0)
            .collect();

        let positive_feedback: Vec<_> = self
            .feedback
            .iter()
            .filter(|f| f.prompt_id == prompt_id)
            .filter(|f| f.signal.to_score() > 0.0)
            .collect();

        // Analyze failure patterns
        let has_task_failures = negative_feedback.iter().any(|f| {
            matches!(f.signal, LearningSignal::TaskFailed { .. })
        });
        let has_verification_failures = negative_feedback.iter().any(|f| {
            matches!(f.signal, LearningSignal::Verification { passed: false, .. })
        });
        let has_rejections = negative_feedback.iter().any(|f| {
            matches!(f.signal, LearningSignal::Negative { .. })
        });

        // Apply targeted mutations
        if has_task_failures {
            mutations.push(Mutation {
                mutation_type: MutationType::Constrain,
                description: "Add explicit constraints to prevent task failures".into(),
            });
            mutations.push(Mutation {
                mutation_type: MutationType::ChainOfThought,
                description: "Add step-by-step reasoning instruction".into(),
            });
        }

        if has_verification_failures {
            mutations.push(Mutation {
                mutation_type: MutationType::Specify,
                description: "Add more specificity to expected output format".into(),
            });
        }

        if has_rejections {
            mutations.push(Mutation {
                mutation_type: MutationType::Restructure,
                description: "Restructure prompt for clarity".into(),
            });
        }

        // If mostly positive but could be better
        if positive_feedback.len() > negative_feedback.len() && mutations.is_empty() {
            mutations.push(Mutation {
                mutation_type: MutationType::Simplify,
                description: "Simplify prompt for efficiency".into(),
            });
        }

        // Always add at least one mutation
        if mutations.is_empty() {
            mutations.push(Mutation {
                mutation_type: MutationType::Exemplify,
                description: "Add examples for better grounding".into(),
            });
        }

        mutations
    }

    /// Apply mutations to create an evolved prompt
    fn apply_mutations(&self, prompt: &str, mutations: &[Mutation]) -> String {
        let mut evolved = prompt.to_string();

        for mutation in mutations {
            evolved = match mutation.mutation_type {
                MutationType::Specify => {
                    format!(
                        "{}\n\nBe specific and precise in your response. \
                         Include concrete details and explicit reasoning.",
                        evolved
                    )
                }
                MutationType::Constrain => {
                    format!(
                        "{}\n\nConstraints:\n\
                         - Stay within the scope of the task\n\
                         - Validate each step before proceeding\n\
                         - If uncertain, state your uncertainty explicitly",
                        evolved
                    )
                }
                MutationType::Restructure => {
                    // Move key instruction to the end (recency bias)
                    let parts: Vec<&str> = evolved.splitn(2, '\n').collect();
                    if parts.len() == 2 {
                        format!("{}\n\nIMPORTANT: {}", parts[1], parts[0])
                    } else {
                        format!("TASK: {}\n\nProvide a clear, structured response.", evolved)
                    }
                }
                MutationType::Exemplify => {
                    format!(
                        "{}\n\nExample format:\n\
                         Input: [example input]\n\
                         Reasoning: [step by step]\n\
                         Output: [concrete answer]",
                        evolved
                    )
                }
                MutationType::Simplify => {
                    // Keep only the essential instruction
                    let lines: Vec<&str> = evolved.lines().collect();
                    if lines.len() > 5 {
                        lines[..5].join("\n")
                    } else {
                        evolved
                    }
                }
                MutationType::ChainOfThought => {
                    format!(
                        "{}\n\nThink step by step:\n\
                         1. Understand the requirements\n\
                         2. Plan your approach\n\
                         3. Execute each step\n\
                         4. Verify your result",
                        evolved
                    )
                }
            };
        }

        evolved
    }

    /// Get the current best prompt for a given ID
    pub fn get_prompt(&self, prompt_id: &str) -> Option<&str> {
        self.trackers.get(prompt_id).map(|t| t.current_prompt.as_str())
    }

    /// Get statistics for all tracked prompts
    pub fn stats(&self) -> AshokaStats {
        let prompt_stats: Vec<_> = self
            .trackers
            .values()
            .map(|t| PromptStats {
                prompt_id: t.prompt_id.clone(),
                generation: t.generation,
                average_score: t.average_score(),
                recent_score: t.recent_score(10),
                feedback_count: t.scores.len(),
            })
            .collect();

        AshokaStats {
            total_feedback: self.feedback.len(),
            total_evolutions: self.evolutions.len(),
            tracked_prompts: self.trackers.len(),
            prompt_stats,
        }
    }
}

/// Ashoka learning statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AshokaStats {
    pub total_feedback: usize,
    pub total_evolutions: usize,
    pub tracked_prompts: usize,
    pub prompt_stats: Vec<PromptStats>,
}

/// Per-prompt statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptStats {
    pub prompt_id: String,
    pub generation: usize,
    pub average_score: f64,
    pub recent_score: f64,
    pub feedback_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_feedback(prompt_id: &str, signal: LearningSignal) -> FeedbackEntry {
        FeedbackEntry {
            prompt_id: prompt_id.to_string(),
            original_prompt: "Solve this problem".to_string(),
            response: "Solution here".to_string(),
            signal,
            context: None,
        }
    }

    #[test]
    fn test_ashoka_creation() {
        let learner = AshokaLearner::new(AshokaConfig::default());
        assert_eq!(learner.feedback_count(), 0);
        assert_eq!(learner.evolution_count(), 0);
    }

    #[test]
    fn test_ashoka_record_feedback() {
        let mut learner = AshokaLearner::new(AshokaConfig::default());

        let feedback = make_feedback("test-1", LearningSignal::Positive { score: 0.9 });
        let result = learner.record(feedback).unwrap();

        assert!(result.is_none()); // Not enough feedback to evolve yet
        assert_eq!(learner.feedback_count(), 1);
    }

    #[test]
    fn test_ashoka_triggers_evolution() {
        let mut learner = AshokaLearner::new(AshokaConfig {
            evolution_threshold: 3,
            selection_pressure: 0.8,
            ..Default::default()
        });

        // Record enough negative feedback to trigger evolution
        for i in 0..3 {
            let feedback = make_feedback(
                "test-1",
                LearningSignal::Negative {
                    score: 0.6,
                    reason: Some(format!("Issue {}", i)),
                },
            );
            let result = learner.record(feedback).unwrap();

            if i == 2 {
                // Should trigger evolution on 3rd entry
                assert!(result.is_some());
                let evolution = result.unwrap();
                assert_eq!(evolution.prompt_id, "test-1");
                assert_eq!(evolution.generation, 1);
                assert!(!evolution.mutations.is_empty());
                break;
            }
        }
    }

    #[test]
    fn test_ashoka_no_evolution_on_positive() {
        let mut learner = AshokaLearner::new(AshokaConfig {
            evolution_threshold: 3,
            selection_pressure: 0.5,
            ..Default::default()
        });

        // Record positive feedback — should not trigger evolution
        for _ in 0..5 {
            let feedback = make_feedback("test-1", LearningSignal::Positive { score: 0.95 });
            let result = learner.record(feedback).unwrap();
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_ashoka_prompt_evolution_content() {
        let mut learner = AshokaLearner::new(AshokaConfig {
            evolution_threshold: 2,
            selection_pressure: 0.9,
            ..Default::default()
        });

        let feedback1 = make_feedback(
            "test-1",
            LearningSignal::TaskFailed {
                error: "timeout".into(),
                step: Some(3),
            },
        );
        let _ = learner.record(feedback1);

        let feedback2 = make_feedback(
            "test-1",
            LearningSignal::Verification {
                passed: false,
                confidence: 0.3,
            },
        );
        let result = learner.record(feedback2).unwrap();

        if let Some(evolution) = result {
            // Should have constraint + chain-of-thought mutations (from task failure)
            assert!(evolution.mutations.iter().any(|m| m.mutation_type == MutationType::Constrain));
            assert!(evolution.evolved.len() > evolution.original.len());
        }
    }

    #[test]
    fn test_ashoka_get_prompt() {
        let mut learner = AshokaLearner::new(AshokaConfig::default());

        let feedback = make_feedback("test-1", LearningSignal::Positive { score: 0.5 });
        let _ = learner.record(feedback);

        assert!(learner.get_prompt("test-1").is_some());
        assert!(learner.get_prompt("nonexistent").is_none());
    }

    #[test]
    fn test_ashoka_stats() {
        let mut learner = AshokaLearner::new(AshokaConfig::default());

        for _ in 0..5 {
            let feedback = make_feedback("p1", LearningSignal::Positive { score: 0.8 });
            let _ = learner.record(feedback);
        }

        let stats = learner.stats();
        assert_eq!(stats.total_feedback, 5);
        assert_eq!(stats.tracked_prompts, 1);
        assert!(!stats.prompt_stats.is_empty());
        assert!((stats.prompt_stats[0].average_score - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_learning_signal_scores() {
        assert!((LearningSignal::Positive { score: 0.9 }.to_score() - 0.9).abs() < 0.01);
        assert!((LearningSignal::Negative { score: 0.5, reason: None }.to_score() + 0.5).abs() < 0.01);
        assert!((LearningSignal::Verification { passed: true, confidence: 0.8 }.to_score() - 0.8).abs() < 0.01);
        assert!((LearningSignal::TaskFailed { error: "err".into(), step: None }.to_score() + 0.5).abs() < 0.01);
    }

    #[test]
    fn test_mutation_types() {
        let learner = AshokaLearner::new(AshokaConfig::default());

        let base = "Do the task";
        let evolved = learner.apply_mutations(
            base,
            &[Mutation {
                mutation_type: MutationType::ChainOfThought,
                description: "test".into(),
            }],
        );
        assert!(evolved.contains("step by step"));

        let evolved2 = learner.apply_mutations(
            base,
            &[Mutation {
                mutation_type: MutationType::Constrain,
                description: "test".into(),
            }],
        );
        assert!(evolved2.contains("Constraints"));
    }

    #[test]
    fn test_feedback_capacity() {
        let mut learner = AshokaLearner::new(AshokaConfig {
            max_feedback: 5,
            ..Default::default()
        });

        for _ in 0..10 {
            let feedback = make_feedback("p1", LearningSignal::Positive { score: 0.5 });
            let _ = learner.record(feedback);
        }

        assert_eq!(learner.feedback_count(), 5);
    }
}
