//! # Observability — Trace Every LLM Call
//!
//! "Vous ne pouvez pas inspecter le raisonnement interne du modele.
//! Rendez l'infrastructure environnante lisible."
//!
//! Every LLM call is traced: inputs, outputs, model version, constraints,
//! tools called, validations, timing, and cost. This transforms the system
//! from a black box into an auditable process.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

/// A single LLM call trace — the atomic unit of observability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmTrace {
    /// Unique trace ID
    pub id: Uuid,
    /// When the call started
    pub started_at: DateTime<Utc>,
    /// When the call completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Which model was called
    pub model: String,
    /// Model version/ID
    pub model_version: String,
    /// Input tokens count
    pub input_tokens: u32,
    /// Output tokens count
    pub output_tokens: u32,
    /// Cost in microdollars (1 dollar = 1_000_000)
    pub cost_microdollars: u64,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// The prompt/system message (may be truncated for storage)
    pub prompt_hash: String,
    /// Constraints that were applied
    pub constraints: Vec<String>,
    /// Tools that were called
    pub tools_called: Vec<String>,
    /// Whether the output was verified
    pub verified: bool,
    /// Verification method if verified
    pub verification_method: Option<String>,
    /// The workflow/pipeline this call belongs to
    pub workflow_id: Option<Uuid>,
    /// Step number in the workflow
    pub workflow_step: Option<u32>,
    /// Outcome classification
    pub outcome: TraceOutcome,
    /// Arbitrary metadata
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TraceOutcome {
    /// Call succeeded, output verified
    Success,
    /// Call succeeded but output failed verification
    VerificationFailed,
    /// Call returned but was classified as a failure mode
    FailureMode(String),
    /// Call timed out
    Timeout,
    /// Call errored (model error, not application error)
    Error(String),
    /// Call was rate-limited or throttled
    RateLimited,
}

impl LlmTrace {
    /// Start a new trace (completed_at filled when done)
    pub fn start(model: impl Into<String>, model_version: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            started_at: Utc::now(),
            completed_at: None,
            model: model.into(),
            model_version: model_version.into(),
            input_tokens: 0,
            output_tokens: 0,
            cost_microdollars: 0,
            latency_ms: 0,
            prompt_hash: String::new(),
            constraints: Vec::new(),
            tools_called: Vec::new(),
            verified: false,
            verification_method: None,
            workflow_id: None,
            workflow_step: None,
            outcome: TraceOutcome::Success,
            metadata: HashMap::new(),
        }
    }

    /// Complete the trace with results
    pub fn complete(
        &mut self,
        input_tokens: u32,
        output_tokens: u32,
        outcome: TraceOutcome,
    ) {
        self.completed_at = Some(Utc::now());
        self.input_tokens = input_tokens;
        self.output_tokens = output_tokens;
        self.outcome = outcome;
        if let Some(start) = self.completed_at {
            self.latency_ms = (start - self.started_at).num_milliseconds().unsigned_abs();
        }
    }

    /// Set the prompt hash (hash the prompt for privacy, don't store it)
    pub fn set_prompt(&mut self, prompt: &str) {
        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        self.prompt_hash = format!("{:x}", hasher.finalize())[..16].to_string();
    }

    /// Compute cost based on model pricing
    pub fn compute_cost(&mut self, input_price_per_mtok: f64, output_price_per_mtok: f64) {
        let input_cost = (self.input_tokens as f64 / 1_000_000.0) * input_price_per_mtok * 1_000_000.0;
        let output_cost = (self.output_tokens as f64 / 1_000_000.0) * output_price_per_mtok * 1_000_000.0;
        self.cost_microdollars = (input_cost + output_cost) as u64;
    }

    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }

    pub fn is_success(&self) -> bool {
        matches!(self.outcome, TraceOutcome::Success)
    }
}

/// The Trace Log — append-only observability ledger
pub struct TraceLog {
    traces: Vec<LlmTrace>,
    /// Cost budget in microdollars (0 = unlimited)
    budget_microdollars: u64,
}

impl TraceLog {
    pub fn new() -> Self {
        Self {
            traces: Vec::new(),
            budget_microdollars: 0,
        }
    }

    pub fn with_budget(budget_dollars: f64) -> Self {
        Self {
            traces: Vec::new(),
            budget_microdollars: (budget_dollars * 1_000_000.0) as u64,
        }
    }

    /// Record a completed trace
    pub fn record(&mut self, trace: LlmTrace) {
        self.traces.push(trace);
    }

    /// Total cost so far
    pub fn total_cost_microdollars(&self) -> u64 {
        self.traces.iter().map(|t| t.cost_microdollars).sum()
    }

    pub fn total_cost_dollars(&self) -> f64 {
        self.total_cost_microdollars() as f64 / 1_000_000.0
    }

    /// Check if budget is exceeded
    pub fn budget_exceeded(&self) -> bool {
        self.budget_microdollars > 0 && self.total_cost_microdollars() > self.budget_microdollars
    }

    /// Remaining budget
    pub fn budget_remaining_dollars(&self) -> Option<f64> {
        if self.budget_microdollars == 0 {
            return None;
        }
        let remaining = self.budget_microdollars.saturating_sub(self.total_cost_microdollars());
        Some(remaining as f64 / 1_000_000.0)
    }

    /// Total tokens consumed
    pub fn total_tokens(&self) -> u64 {
        self.traces.iter().map(|t| t.total_tokens() as u64).sum()
    }

    /// Success rate
    pub fn success_rate(&self) -> f64 {
        if self.traces.is_empty() {
            return 0.0;
        }
        let successes = self.traces.iter().filter(|t| t.is_success()).count();
        successes as f64 / self.traces.len() as f64
    }

    /// Average latency in ms
    pub fn avg_latency_ms(&self) -> f64 {
        if self.traces.is_empty() {
            return 0.0;
        }
        let total: u64 = self.traces.iter().map(|t| t.latency_ms).sum();
        total as f64 / self.traces.len() as f64
    }

    /// Get traces for a specific workflow
    pub fn workflow_traces(&self, workflow_id: Uuid) -> Vec<&LlmTrace> {
        self.traces
            .iter()
            .filter(|t| t.workflow_id == Some(workflow_id))
            .collect()
    }

    /// Token waste: tokens spent on failed calls
    pub fn wasted_tokens(&self) -> u64 {
        self.traces
            .iter()
            .filter(|t| !t.is_success())
            .map(|t| t.total_tokens() as u64)
            .sum()
    }

    /// Waste rate (the 30-50% that Tool Mode bleeds)
    pub fn waste_rate(&self) -> f64 {
        let total = self.total_tokens();
        if total == 0 {
            return 0.0;
        }
        self.wasted_tokens() as f64 / total as f64
    }

    /// Get last N traces
    pub fn last_n(&self, n: usize) -> &[LlmTrace] {
        let start = self.traces.len().saturating_sub(n);
        &self.traces[start..]
    }

    pub fn trace_count(&self) -> usize {
        self.traces.len()
    }

    /// Comprehensive summary
    pub fn summary(&self) -> TraceSummary {
        let models: Vec<String> = self.traces.iter().map(|t| t.model.clone()).collect::<std::collections::HashSet<_>>().into_iter().collect();

        TraceSummary {
            total_traces: self.traces.len(),
            total_tokens: self.total_tokens(),
            wasted_tokens: self.wasted_tokens(),
            waste_rate: self.waste_rate(),
            total_cost_dollars: self.total_cost_dollars(),
            budget_remaining: self.budget_remaining_dollars(),
            success_rate: self.success_rate(),
            avg_latency_ms: self.avg_latency_ms(),
            models_used: models,
            verified_count: self.traces.iter().filter(|t| t.verified).count(),
        }
    }
}

impl Default for TraceLog {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSummary {
    pub total_traces: usize,
    pub total_tokens: u64,
    pub wasted_tokens: u64,
    pub waste_rate: f64,
    pub total_cost_dollars: f64,
    pub budget_remaining: Option<f64>,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub models_used: Vec<String>,
    pub verified_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_trace(success: bool) -> LlmTrace {
        let mut trace = LlmTrace::start("apophy-local", "nemotron-7b");
        trace.set_prompt("Summarize this document in 200 words");
        trace.constraints = vec!["max_tokens: 200".into(), "format: markdown".into()];
        trace.complete(
            500,
            200,
            if success { TraceOutcome::Success } else { TraceOutcome::FailureMode("hallucination".into()) },
        );
        trace.compute_cost(3.0, 15.0); // $3/Mtok input, $15/Mtok output
        trace
    }

    #[test]
    fn test_trace_lifecycle() {
        let mut trace = LlmTrace::start("gpt-4", "gpt-4-turbo-2024");
        assert!(trace.completed_at.is_none());

        trace.complete(1000, 500, TraceOutcome::Success);
        assert!(trace.completed_at.is_some());
        assert_eq!(trace.total_tokens(), 1500);
        assert!(trace.is_success());
    }

    #[test]
    fn test_prompt_hashing() {
        let mut trace = LlmTrace::start("model", "v1");
        trace.set_prompt("my secret prompt");
        assert_eq!(trace.prompt_hash.len(), 16);
        assert!(!trace.prompt_hash.contains("secret")); // Hash, not plaintext
    }

    #[test]
    fn test_cost_computation() {
        let mut trace = LlmTrace::start("model", "v1");
        trace.input_tokens = 1_000_000;
        trace.output_tokens = 1_000_000;
        trace.compute_cost(3.0, 15.0); // $3 input + $15 output = $18
        assert_eq!(trace.cost_microdollars, 18_000_000);
    }

    #[test]
    fn test_trace_log_recording() {
        let mut log = TraceLog::new();
        log.record(sample_trace(true));
        log.record(sample_trace(true));
        log.record(sample_trace(false));

        assert_eq!(log.trace_count(), 3);
        assert!((log.success_rate() - 0.6667).abs() < 0.01);
    }

    #[test]
    fn test_budget_tracking() {
        let mut log = TraceLog::with_budget(10.0); // $10 budget

        let mut trace = LlmTrace::start("model", "v1");
        trace.cost_microdollars = 5_000_000; // $5
        log.record(trace);

        assert!(!log.budget_exceeded());
        assert!((log.budget_remaining_dollars().unwrap() - 5.0).abs() < 0.01);

        let mut trace2 = LlmTrace::start("model", "v1");
        trace2.cost_microdollars = 6_000_000; // $6
        log.record(trace2);

        assert!(log.budget_exceeded());
    }

    #[test]
    fn test_waste_tracking() {
        let mut log = TraceLog::new();
        log.record(sample_trace(true));  // 700 tokens success
        log.record(sample_trace(false)); // 700 tokens wasted
        log.record(sample_trace(true));  // 700 tokens success

        assert_eq!(log.wasted_tokens(), 700);
        assert!((log.waste_rate() - 0.3333).abs() < 0.01);
    }

    #[test]
    fn test_workflow_filtering() {
        let mut log = TraceLog::new();
        let wf_id = Uuid::new_v4();

        let mut t1 = sample_trace(true);
        t1.workflow_id = Some(wf_id);
        t1.workflow_step = Some(1);
        log.record(t1);

        let mut t2 = sample_trace(true);
        t2.workflow_id = Some(wf_id);
        t2.workflow_step = Some(2);
        log.record(t2);

        log.record(sample_trace(true)); // No workflow

        assert_eq!(log.workflow_traces(wf_id).len(), 2);
    }

    #[test]
    fn test_summary() {
        let mut log = TraceLog::with_budget(100.0);
        log.record(sample_trace(true));
        log.record(sample_trace(false));

        let summary = log.summary();
        assert_eq!(summary.total_traces, 2);
        assert!(summary.total_tokens > 0);
        assert!(summary.budget_remaining.is_some());
        assert_eq!(summary.models_used.len(), 1);
    }

    #[test]
    fn test_summary_serialization() {
        let log = TraceLog::new();
        let summary = log.summary();
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("waste_rate"));
    }

    #[test]
    fn test_empty_log() {
        let log = TraceLog::new();
        assert_eq!(log.success_rate(), 0.0);
        assert_eq!(log.waste_rate(), 0.0);
        assert_eq!(log.avg_latency_ms(), 0.0);
    }
}
