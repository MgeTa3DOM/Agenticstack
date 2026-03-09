//! # Failure Taxonomy — Systematic Classification of Failure Modes
//!
//! "Si tu ne nommes pas le probleme, tu ne peux pas le resoudre.
//! Si tu ne le classifies pas, tu ne peux pas le prevenir."
//!
//! 8 canonical failure modes for LLM systems:
//! 1. Hallucination — confident fabrication
//! 2. Drift — gradual deviation from intent
//! 3. Refusal — model refuses valid request
//! 4. Verbosity — token waste, padding, repetition
//! 5. Format violation — output doesn't match spec
//! 6. Regression — previously working behavior breaks
//! 7. Cascade — one failure triggers chain reaction
//! 8. Authority loss — agent exceeds its permissions
//!
//! Each failure gets a Diagnosis: what happened, why, and how to prevent recurrence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// The 8 canonical failure modes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailureMode {
    /// Model generates confident but incorrect information
    Hallucination,
    /// Output gradually deviates from original intent over iterations
    Drift,
    /// Model refuses a valid, permitted request
    Refusal,
    /// Excessive tokens: padding, repetition, unnecessary hedging
    Verbosity,
    /// Output doesn't match required format (JSON, markdown, etc.)
    FormatViolation,
    /// Previously working behavior now fails
    Regression,
    /// One failure triggers downstream failures
    Cascade,
    /// Agent acts outside its permission envelope
    AuthorityLoss,
}

impl FailureMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Hallucination => "Hallucination",
            Self::Drift => "Drift",
            Self::Refusal => "Refusal",
            Self::Verbosity => "Verbosity",
            Self::FormatViolation => "Format Violation",
            Self::Regression => "Regression",
            Self::Cascade => "Cascade",
            Self::AuthorityLoss => "Authority Loss",
        }
    }

    pub fn severity_default(&self) -> FailureSeverity {
        match self {
            Self::Hallucination => FailureSeverity::Critical,
            Self::AuthorityLoss => FailureSeverity::Critical,
            Self::Cascade => FailureSeverity::Critical,
            Self::Regression => FailureSeverity::High,
            Self::FormatViolation => FailureSeverity::High,
            Self::Drift => FailureSeverity::Medium,
            Self::Refusal => FailureSeverity::Medium,
            Self::Verbosity => FailureSeverity::Low,
        }
    }

    pub fn all() -> Vec<FailureMode> {
        vec![
            Self::Hallucination,
            Self::Drift,
            Self::Refusal,
            Self::Verbosity,
            Self::FormatViolation,
            Self::Regression,
            Self::Cascade,
            Self::AuthorityLoss,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureSeverity {
    /// System-breaking, data corruption, security breach
    Critical,
    /// Significant impact, needs immediate fix
    High,
    /// Noticeable impact, fix in next iteration
    Medium,
    /// Minor impact, optimize when possible
    Low,
}

/// A diagnosis of a specific failure instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnosis {
    /// Unique ID
    pub id: Uuid,
    /// When was this failure detected
    pub detected_at: DateTime<Utc>,
    /// Which failure mode
    pub mode: FailureMode,
    /// Severity override (or default)
    pub severity: FailureSeverity,
    /// Which trace/call triggered this
    pub trace_id: Option<Uuid>,
    /// What happened (description)
    pub what: String,
    /// Why it happened (root cause analysis)
    pub why: String,
    /// How to prevent recurrence
    pub prevention: String,
    /// Was this failure caught by verification?
    pub caught_by_verification: bool,
    /// Tokens wasted by this failure
    pub tokens_wasted: u64,
    /// Cost wasted in microdollars
    pub cost_wasted_microdollars: u64,
    /// Tags for categorization
    pub tags: Vec<String>,
}

impl Diagnosis {
    pub fn new(mode: FailureMode, what: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            detected_at: Utc::now(),
            severity: mode.severity_default(),
            mode,
            trace_id: None,
            what: what.into(),
            why: String::new(),
            prevention: String::new(),
            caught_by_verification: false,
            tokens_wasted: 0,
            cost_wasted_microdollars: 0,
            tags: Vec::new(),
        }
    }

    pub fn with_cause(mut self, why: impl Into<String>) -> Self {
        self.why = why.into();
        self
    }

    pub fn with_prevention(mut self, prevention: impl Into<String>) -> Self {
        self.prevention = prevention.into();
        self
    }

    pub fn with_trace(mut self, trace_id: Uuid) -> Self {
        self.trace_id = Some(trace_id);
        self
    }

    pub fn with_waste(mut self, tokens: u64, cost_microdollars: u64) -> Self {
        self.tokens_wasted = tokens;
        self.cost_wasted_microdollars = cost_microdollars;
        self
    }

    pub fn caught(mut self) -> Self {
        self.caught_by_verification = true;
        self
    }
}

/// The Failure Taxonomy — tracks all diagnosed failures.
pub struct FailureTaxonomy {
    diagnoses: Vec<Diagnosis>,
}

impl FailureTaxonomy {
    pub fn new() -> Self {
        Self {
            diagnoses: Vec::new(),
        }
    }

    /// Record a new diagnosis.
    pub fn record(&mut self, diagnosis: Diagnosis) {
        self.diagnoses.push(diagnosis);
    }

    /// Total failures recorded.
    pub fn total(&self) -> usize {
        self.diagnoses.len()
    }

    /// Count by failure mode.
    pub fn count_by_mode(&self) -> HashMap<FailureMode, usize> {
        let mut counts = HashMap::new();
        for d in &self.diagnoses {
            *counts.entry(d.mode.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Most common failure mode.
    pub fn most_common(&self) -> Option<(FailureMode, usize)> {
        self.count_by_mode()
            .into_iter()
            .max_by_key(|(_, count)| *count)
    }

    /// Total tokens wasted across all failures.
    pub fn total_tokens_wasted(&self) -> u64 {
        self.diagnoses.iter().map(|d| d.tokens_wasted).sum()
    }

    /// Total cost wasted in dollars.
    pub fn total_cost_wasted_dollars(&self) -> f64 {
        let microdollars: u64 = self.diagnoses.iter().map(|d| d.cost_wasted_microdollars).sum();
        microdollars as f64 / 1_000_000.0
    }

    /// Catch rate: what percentage of failures were caught by verification?
    pub fn catch_rate(&self) -> f64 {
        if self.diagnoses.is_empty() {
            return 0.0;
        }
        let caught = self.diagnoses.iter().filter(|d| d.caught_by_verification).count();
        caught as f64 / self.diagnoses.len() as f64
    }

    /// Critical failures (the ones that matter most).
    pub fn critical_failures(&self) -> Vec<&Diagnosis> {
        self.diagnoses
            .iter()
            .filter(|d| d.severity == FailureSeverity::Critical)
            .collect()
    }

    /// Failures by mode.
    pub fn by_mode(&self, mode: &FailureMode) -> Vec<&Diagnosis> {
        self.diagnoses
            .iter()
            .filter(|d| &d.mode == mode)
            .collect()
    }

    /// Recent failures (last N).
    pub fn recent(&self, n: usize) -> &[Diagnosis] {
        let start = self.diagnoses.len().saturating_sub(n);
        &self.diagnoses[start..]
    }

    /// Generate a taxonomy report.
    pub fn report(&self) -> TaxonomyReport {
        let counts = self.count_by_mode();
        let mut mode_reports: Vec<ModeReport> = FailureMode::all()
            .into_iter()
            .map(|mode| {
                let count = counts.get(&mode).copied().unwrap_or(0);
                let failures = self.by_mode(&mode);
                let tokens_wasted: u64 = failures.iter().map(|d| d.tokens_wasted).sum();
                let caught = failures.iter().filter(|d| d.caught_by_verification).count();
                ModeReport {
                    mode,
                    count,
                    tokens_wasted,
                    catch_rate: if count > 0 { caught as f64 / count as f64 } else { 0.0 },
                }
            })
            .collect();

        mode_reports.sort_by(|a, b| b.count.cmp(&a.count));

        TaxonomyReport {
            total_failures: self.total(),
            total_tokens_wasted: self.total_tokens_wasted(),
            total_cost_wasted_dollars: self.total_cost_wasted_dollars(),
            overall_catch_rate: self.catch_rate(),
            critical_count: self.critical_failures().len(),
            modes: mode_reports,
        }
    }
}

impl Default for FailureTaxonomy {
    fn default() -> Self {
        Self::new()
    }
}

/// Report for a single failure mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeReport {
    pub mode: FailureMode,
    pub count: usize,
    pub tokens_wasted: u64,
    pub catch_rate: f64,
}

/// Full taxonomy report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyReport {
    pub total_failures: usize,
    pub total_tokens_wasted: u64,
    pub total_cost_wasted_dollars: f64,
    pub overall_catch_rate: f64,
    pub critical_count: usize,
    pub modes: Vec<ModeReport>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_hallucination() -> Diagnosis {
        Diagnosis::new(FailureMode::Hallucination, "Model fabricated a non-existent API")
            .with_cause("No grounding in documentation")
            .with_prevention("Add retrieval step before generation")
            .with_waste(500, 7500)
            .caught()
    }

    fn sample_drift() -> Diagnosis {
        Diagnosis::new(FailureMode::Drift, "Output style changed over 10 iterations")
            .with_cause("No style constraints in system prompt")
            .with_prevention("Add golden example in context")
            .with_waste(2000, 30000)
    }

    fn sample_verbosity() -> Diagnosis {
        Diagnosis::new(FailureMode::Verbosity, "300 tokens of hedging before answer")
            .with_cause("Default model behavior without constraint")
            .with_prevention("Add 'be concise' constraint")
            .with_waste(300, 4500)
    }

    #[test]
    fn test_failure_mode_labels() {
        assert_eq!(FailureMode::Hallucination.label(), "Hallucination");
        assert_eq!(FailureMode::AuthorityLoss.label(), "Authority Loss");
        assert_eq!(FailureMode::all().len(), 8);
    }

    #[test]
    fn test_severity_defaults() {
        assert_eq!(FailureMode::Hallucination.severity_default(), FailureSeverity::Critical);
        assert_eq!(FailureMode::Verbosity.severity_default(), FailureSeverity::Low);
        assert_eq!(FailureMode::Drift.severity_default(), FailureSeverity::Medium);
    }

    #[test]
    fn test_diagnosis_creation() {
        let d = sample_hallucination();
        assert_eq!(d.mode, FailureMode::Hallucination);
        assert!(d.caught_by_verification);
        assert_eq!(d.tokens_wasted, 500);
    }

    #[test]
    fn test_taxonomy_recording() {
        let mut tax = FailureTaxonomy::new();
        tax.record(sample_hallucination());
        tax.record(sample_drift());
        tax.record(sample_verbosity());

        assert_eq!(tax.total(), 3);
    }

    #[test]
    fn test_count_by_mode() {
        let mut tax = FailureTaxonomy::new();
        tax.record(sample_hallucination());
        tax.record(sample_hallucination());
        tax.record(sample_drift());

        let counts = tax.count_by_mode();
        assert_eq!(counts[&FailureMode::Hallucination], 2);
        assert_eq!(counts[&FailureMode::Drift], 1);
    }

    #[test]
    fn test_most_common() {
        let mut tax = FailureTaxonomy::new();
        tax.record(sample_hallucination());
        tax.record(sample_hallucination());
        tax.record(sample_drift());

        let (mode, count) = tax.most_common().unwrap();
        assert_eq!(mode, FailureMode::Hallucination);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_waste_tracking() {
        let mut tax = FailureTaxonomy::new();
        tax.record(sample_hallucination()); // 500 tokens, 7500 microdollars
        tax.record(sample_verbosity());     // 300 tokens, 4500 microdollars

        assert_eq!(tax.total_tokens_wasted(), 800);
        assert!((tax.total_cost_wasted_dollars() - 0.012).abs() < 0.001);
    }

    #[test]
    fn test_catch_rate() {
        let mut tax = FailureTaxonomy::new();
        tax.record(sample_hallucination()); // caught
        tax.record(sample_drift());          // not caught
        tax.record(sample_verbosity());      // not caught

        assert!((tax.catch_rate() - 0.3333).abs() < 0.01);
    }

    #[test]
    fn test_critical_failures() {
        let mut tax = FailureTaxonomy::new();
        tax.record(sample_hallucination()); // critical
        tax.record(sample_verbosity());      // low

        assert_eq!(tax.critical_failures().len(), 1);
    }

    #[test]
    fn test_report_generation() {
        let mut tax = FailureTaxonomy::new();
        tax.record(sample_hallucination());
        tax.record(sample_hallucination());
        tax.record(sample_drift());
        tax.record(sample_verbosity());

        let report = tax.report();
        assert_eq!(report.total_failures, 4);
        assert_eq!(report.modes.len(), 8); // All 8 modes reported
        assert_eq!(report.modes[0].count, 2); // Most common first
    }

    #[test]
    fn test_report_serialization() {
        let mut tax = FailureTaxonomy::new();
        tax.record(sample_hallucination());
        let report = tax.report();
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("Hallucination"));
        assert!(json.contains("total_tokens_wasted"));
    }

    #[test]
    fn test_empty_taxonomy() {
        let tax = FailureTaxonomy::new();
        assert_eq!(tax.total(), 0);
        assert_eq!(tax.catch_rate(), 0.0);
        assert!(tax.most_common().is_none());
    }
}
