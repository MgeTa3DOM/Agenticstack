//! # Evaluation Harness — The Compound Loop
//!
//! "L'evaluation est le seul mecanisme qui transforme
//! l'experience en amelioration systematique."
//!
//! Components:
//! - **Golden Examples**: known-good input/output pairs (the ground truth)
//! - **Scorecards**: multi-dimension scoring with pass/fail thresholds
//! - **Regression Tests**: ensure what worked before still works
//! - **EvalHarness**: orchestrates everything, tracks improvement over time

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A golden example — known-good input/output pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenExample {
    /// Example ID
    pub id: Uuid,
    /// Human-readable name
    pub name: String,
    /// The input prompt
    pub input: String,
    /// The expected output (or output pattern)
    pub expected_output: String,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Which dimensions to score on
    pub dimensions: Vec<String>,
    /// When this example was created
    pub created_at: DateTime<Utc>,
    /// Is this a regression test? (must not fail)
    pub is_regression_test: bool,
}

impl GoldenExample {
    pub fn new(name: impl Into<String>, input: impl Into<String>, expected: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            input: input.into(),
            expected_output: expected.into(),
            tags: Vec::new(),
            dimensions: vec!["accuracy".into(), "format".into(), "conciseness".into()],
            created_at: Utc::now(),
            is_regression_test: false,
        }
    }

    pub fn as_regression(mut self) -> Self {
        self.is_regression_test = true;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_dimensions(mut self, dims: Vec<String>) -> Self {
        self.dimensions = dims;
        self
    }
}

/// A scorecard — multi-dimensional scoring for an eval run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scorecard {
    /// Which golden example was tested
    pub example_id: Uuid,
    /// Example name
    pub example_name: String,
    /// When this eval ran
    pub evaluated_at: DateTime<Utc>,
    /// Dimension scores (0.0 to 1.0)
    pub scores: HashMap<String, f64>,
    /// Overall score (average of dimensions)
    pub overall_score: f64,
    /// Did this pass the threshold?
    pub passed: bool,
    /// The actual output from the model
    pub actual_output: String,
    /// Time to generate in ms
    pub latency_ms: u64,
    /// Tokens used
    pub tokens_used: u32,
    /// Is this a regression failure?
    pub is_regression_failure: bool,
    /// Notes from scoring
    pub notes: String,
}

impl Scorecard {
    pub fn new(example: &GoldenExample, actual_output: impl Into<String>) -> Self {
        Self {
            example_id: example.id,
            example_name: example.name.clone(),
            evaluated_at: Utc::now(),
            scores: HashMap::new(),
            overall_score: 0.0,
            passed: false,
            actual_output: actual_output.into(),
            latency_ms: 0,
            tokens_used: 0,
            is_regression_failure: false,
            notes: String::new(),
        }
    }

    /// Score a dimension.
    pub fn score(&mut self, dimension: impl Into<String>, value: f64) {
        self.scores.insert(dimension.into(), value.clamp(0.0, 1.0));
        self.recalculate();
    }

    /// Set multiple scores at once.
    pub fn score_all(&mut self, scores: HashMap<String, f64>) {
        for (dim, val) in scores {
            self.scores.insert(dim, val.clamp(0.0, 1.0));
        }
        self.recalculate();
    }

    fn recalculate(&mut self) {
        if self.scores.is_empty() {
            self.overall_score = 0.0;
            return;
        }
        let sum: f64 = self.scores.values().sum();
        self.overall_score = sum / self.scores.len() as f64;
    }

    /// Check against a threshold.
    pub fn evaluate(&mut self, threshold: f64, is_regression: bool) {
        self.passed = self.overall_score >= threshold;
        if is_regression && !self.passed {
            self.is_regression_failure = true;
        }
    }
}

/// Result of a full evaluation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    /// Run ID
    pub id: Uuid,
    /// When the eval ran
    pub ran_at: DateTime<Utc>,
    /// Model tested
    pub model: String,
    /// All scorecards
    pub scorecards: Vec<Scorecard>,
    /// Pass rate (0.0 to 1.0)
    pub pass_rate: f64,
    /// Average overall score
    pub avg_score: f64,
    /// Regression failures (must be 0 for deployment)
    pub regression_failures: usize,
    /// Total tokens consumed by eval
    pub total_tokens: u64,
    /// Total time in ms
    pub total_latency_ms: u64,
    /// Can this model be deployed? (pass_rate >= threshold AND 0 regressions)
    pub deployable: bool,
}

/// The Evaluation Harness — orchestrates golden examples and scoring.
pub struct EvalHarness {
    /// Golden examples (the ground truth)
    examples: Vec<GoldenExample>,
    /// Historical eval results (for tracking improvement)
    history: Vec<EvalResult>,
    /// Pass/fail threshold (default 0.8)
    threshold: f64,
    /// Deployment threshold (default 0.9)
    deploy_threshold: f64,
}

impl EvalHarness {
    pub fn new() -> Self {
        Self {
            examples: Vec::new(),
            history: Vec::new(),
            threshold: 0.8,
            deploy_threshold: 0.9,
        }
    }

    pub fn with_thresholds(threshold: f64, deploy_threshold: f64) -> Self {
        Self {
            examples: Vec::new(),
            history: Vec::new(),
            threshold,
            deploy_threshold,
        }
    }

    /// Add a golden example.
    pub fn add_example(&mut self, example: GoldenExample) {
        self.examples.push(example);
    }

    /// Number of golden examples.
    pub fn example_count(&self) -> usize {
        self.examples.len()
    }

    /// Number of regression tests.
    pub fn regression_count(&self) -> usize {
        self.examples.iter().filter(|e| e.is_regression_test).count()
    }

    /// Get examples by tag.
    pub fn examples_by_tag(&self, tag: &str) -> Vec<&GoldenExample> {
        self.examples.iter().filter(|e| e.tags.contains(&tag.to_string())).collect()
    }

    /// Run an evaluation: takes a scorer function that produces a Scorecard for each example.
    pub fn run_eval<F>(&mut self, model: impl Into<String>, scorer: F) -> EvalResult
    where
        F: Fn(&GoldenExample) -> Scorecard,
    {
        let model_name = model.into();
        let mut scorecards = Vec::new();

        for example in &self.examples {
            let mut card = scorer(example);
            card.evaluate(self.threshold, example.is_regression_test);
            scorecards.push(card);
        }

        let passed = scorecards.iter().filter(|c| c.passed).count();
        let pass_rate = if scorecards.is_empty() {
            0.0
        } else {
            passed as f64 / scorecards.len() as f64
        };

        let avg_score = if scorecards.is_empty() {
            0.0
        } else {
            scorecards.iter().map(|c| c.overall_score).sum::<f64>() / scorecards.len() as f64
        };

        let regression_failures = scorecards.iter().filter(|c| c.is_regression_failure).count();
        let total_tokens: u64 = scorecards.iter().map(|c| c.tokens_used as u64).sum();
        let total_latency: u64 = scorecards.iter().map(|c| c.latency_ms).sum();

        let deployable = pass_rate >= self.deploy_threshold && regression_failures == 0;

        let result = EvalResult {
            id: Uuid::new_v4(),
            ran_at: Utc::now(),
            model: model_name,
            scorecards,
            pass_rate,
            avg_score,
            regression_failures,
            total_tokens,
            total_latency_ms: total_latency,
            deployable,
        };

        self.history.push(result.clone());
        result
    }

    /// Get improvement trend (avg_score over time).
    pub fn improvement_trend(&self) -> Vec<(DateTime<Utc>, f64)> {
        self.history.iter().map(|r| (r.ran_at, r.avg_score)).collect()
    }

    /// Is the system improving?
    pub fn is_improving(&self) -> Option<bool> {
        if self.history.len() < 2 {
            return None;
        }
        let last = self.history.last().unwrap().avg_score;
        let prev = self.history[self.history.len() - 2].avg_score;
        Some(last >= prev)
    }

    /// Latest eval result.
    pub fn latest(&self) -> Option<&EvalResult> {
        self.history.last()
    }

    /// Is the system deployable based on latest eval?
    pub fn is_deployable(&self) -> bool {
        self.history.last().map(|r| r.deployable).unwrap_or(false)
    }

    /// History count.
    pub fn history_count(&self) -> usize {
        self.history.len()
    }
}

impl Default for EvalHarness {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_examples() -> Vec<GoldenExample> {
        vec![
            GoldenExample::new(
                "Summarize",
                "Summarize this document in 3 sentences",
                "Three sentence summary of the document.",
            ).with_tags(vec!["summary".into()]),
            GoldenExample::new(
                "Extract JSON",
                "Extract name and age from: John is 30",
                r#"{"name": "John", "age": 30}"#,
            ).with_tags(vec!["extraction".into()]).as_regression(),
            GoldenExample::new(
                "Classify",
                "Is this email spam? 'Buy now!'",
                "spam",
            ).with_tags(vec!["classification".into()]),
        ]
    }

    fn good_scorer(example: &GoldenExample) -> Scorecard {
        let mut card = Scorecard::new(example, "good output");
        card.score("accuracy", 0.95);
        card.score("format", 0.90);
        card.score("conciseness", 0.85);
        card.tokens_used = 100;
        card.latency_ms = 50;
        card
    }

    fn mixed_scorer(example: &GoldenExample) -> Scorecard {
        let mut card = Scorecard::new(example, "mixed output");
        if example.name == "Summarize" {
            card.score("accuracy", 0.9);
            card.score("format", 0.9);
        } else {
            card.score("accuracy", 0.5);
            card.score("format", 0.6);
        }
        card.tokens_used = 150;
        card
    }

    #[test]
    fn test_golden_example_creation() {
        let ex = GoldenExample::new("Test", "input", "output");
        assert_eq!(ex.name, "Test");
        assert!(!ex.is_regression_test);

        let ex = ex.as_regression();
        assert!(ex.is_regression_test);
    }

    #[test]
    fn test_scorecard_scoring() {
        let ex = GoldenExample::new("Test", "in", "out");
        let mut card = Scorecard::new(&ex, "actual");
        card.score("accuracy", 0.9);
        card.score("format", 0.8);

        assert!((card.overall_score - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_scorecard_clamp() {
        let ex = GoldenExample::new("Test", "in", "out");
        let mut card = Scorecard::new(&ex, "actual");
        card.score("accuracy", 1.5); // should clamp to 1.0
        card.score("format", -0.5);  // should clamp to 0.0

        assert_eq!(card.scores["accuracy"], 1.0);
        assert_eq!(card.scores["format"], 0.0);
    }

    #[test]
    fn test_eval_harness_basic() {
        let mut harness = EvalHarness::new();
        for ex in sample_examples() {
            harness.add_example(ex);
        }

        assert_eq!(harness.example_count(), 3);
        assert_eq!(harness.regression_count(), 1);
    }

    #[test]
    fn test_eval_run_all_pass() {
        let mut harness = EvalHarness::new();
        for ex in sample_examples() {
            harness.add_example(ex);
        }

        let result = harness.run_eval("test-model", good_scorer);
        assert_eq!(result.pass_rate, 1.0);
        assert_eq!(result.regression_failures, 0);
        assert!(result.deployable);
        assert!(result.avg_score > 0.85);
    }

    #[test]
    fn test_eval_run_mixed() {
        let mut harness = EvalHarness::new();
        for ex in sample_examples() {
            harness.add_example(ex);
        }

        let result = harness.run_eval("test-model", mixed_scorer);
        assert!(result.pass_rate < 1.0);
        // "Extract JSON" is regression + low score → regression failure
        assert!(result.regression_failures > 0);
        assert!(!result.deployable);
    }

    #[test]
    fn test_improvement_tracking() {
        let mut harness = EvalHarness::new();
        for ex in sample_examples() {
            harness.add_example(ex);
        }

        // First run: mixed
        harness.run_eval("model-v1", mixed_scorer);
        // Second run: good
        harness.run_eval("model-v2", good_scorer);

        assert_eq!(harness.history_count(), 2);
        assert_eq!(harness.is_improving(), Some(true));
        assert!(harness.is_deployable());
    }

    #[test]
    fn test_improvement_trend() {
        let mut harness = EvalHarness::new();
        harness.add_example(GoldenExample::new("Test", "in", "out"));

        harness.run_eval("v1", |ex| {
            let mut c = Scorecard::new(ex, "out");
            c.score("accuracy", 0.7);
            c
        });
        harness.run_eval("v2", |ex| {
            let mut c = Scorecard::new(ex, "out");
            c.score("accuracy", 0.9);
            c
        });

        let trend = harness.improvement_trend();
        assert_eq!(trend.len(), 2);
        assert!(trend[1].1 > trend[0].1);
    }

    #[test]
    fn test_examples_by_tag() {
        let mut harness = EvalHarness::new();
        for ex in sample_examples() {
            harness.add_example(ex);
        }

        assert_eq!(harness.examples_by_tag("summary").len(), 1);
        assert_eq!(harness.examples_by_tag("nonexistent").len(), 0);
    }

    #[test]
    fn test_deploy_threshold() {
        let mut harness = EvalHarness::with_thresholds(0.5, 0.95);
        harness.add_example(GoldenExample::new("Test", "in", "out"));

        let result = harness.run_eval("model", |ex| {
            let mut c = Scorecard::new(ex, "out");
            c.score("accuracy", 0.4); // fails threshold (0.5) → pass_rate=0.0 < deploy(0.95)
            c
        });

        assert_eq!(result.pass_rate, 0.0);
        assert!(!result.deployable);
    }

    #[test]
    fn test_empty_harness() {
        let mut harness = EvalHarness::new();
        let result = harness.run_eval("model", |_| unreachable!());
        assert_eq!(result.pass_rate, 0.0);
        assert!(!result.deployable);
        assert_eq!(harness.is_improving(), None);
    }

    #[test]
    fn test_result_serialization() {
        let mut harness = EvalHarness::new();
        harness.add_example(GoldenExample::new("Test", "in", "out"));
        let result = harness.run_eval("model", good_scorer);
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("pass_rate"));
        assert!(json.contains("deployable"));
        assert!(json.contains("scorecards"));
    }
}
