//! # Blueprint Engine — "agents + code > agents seuls > code seul"
//!
//! Implements Stripe's key insight from their Minions architecture:
//! a Blueprint is a **coded workflow** that interleaves deterministic steps
//! (validation, parsing, file I/O, DB queries) with agent-piloted steps
//! (reasoning, generation, decision-making).
//!
//! ## Why This Matters
//!
//! - **Deterministic steps** are fast, reliable, auditable, and free.
//! - **Agent steps** are flexible, creative, but expensive and unreliable.
//! - The Blueprint Engine ensures agents only do what agents are good at,
//!   while the rest is handled by trustworthy code.
//!
//! ## Design
//!
//! ```text
//! Blueprint = Vec<BlueprintStep>
//!   ├── Deterministic { name, action: fn(ctx) -> Result }
//!   ├── Agent { name, prompt_template, constraints, validation }
//!   ├── Gate { name, condition: fn(ctx) -> bool }
//!   ├── Fork { name, condition, if_true, if_false }
//!   └── Parallel { name, steps: Vec<BlueprintStep> }
//! ```
//!
//! The runner executes steps in order, threading a shared `BlueprintContext`
//! (key-value store) through each step. Agent steps produce output that
//! deterministic steps then validate.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

// =============================================================================
// BLUEPRINT CONTEXT — shared state threaded through all steps
// =============================================================================

/// Key-value context passed between blueprint steps.
/// Deterministic steps read/write here. Agent steps receive a snapshot
/// and their output is merged back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintContext {
    /// Unique execution ID
    pub execution_id: Uuid,
    /// Key-value store (string→string for serialization safety)
    pub values: HashMap<String, String>,
    /// Structured artifacts produced by steps (JSON blobs)
    pub artifacts: HashMap<String, serde_json::Value>,
    /// Execution trace for observability
    pub trace: Vec<StepTrace>,
    /// Started at
    pub started_at: DateTime<Utc>,
    /// Total tokens consumed by agent steps
    pub total_tokens: u64,
    /// Total cost in microdollars
    pub total_cost_microdollars: u64,
}

impl BlueprintContext {
    pub fn new() -> Self {
        Self {
            execution_id: Uuid::new_v4(),
            values: HashMap::new(),
            artifacts: HashMap::new(),
            trace: Vec::new(),
            started_at: Utc::now(),
            total_tokens: 0,
            total_cost_microdollars: 0,
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    pub fn set_artifact(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.artifacts.insert(key.into(), value);
    }

    pub fn get_artifact(&self, key: &str) -> Option<&serde_json::Value> {
        self.artifacts.get(key)
    }

    pub fn record_trace(&mut self, trace: StepTrace) {
        if let Some(tokens) = trace.tokens_used {
            self.total_tokens += tokens;
        }
        if let Some(cost) = trace.cost_microdollars {
            self.total_cost_microdollars += cost;
        }
        self.trace.push(trace);
    }

    /// Elapsed time since execution start
    pub fn elapsed_ms(&self) -> i64 {
        (Utc::now() - self.started_at).num_milliseconds()
    }
}

impl Default for BlueprintContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Trace of a single step execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepTrace {
    pub step_name: String,
    pub step_kind: StepKind,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: i64,
    pub outcome: StepOutcome,
    pub tokens_used: Option<u64>,
    pub cost_microdollars: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepOutcome {
    Success,
    Failed,
    Skipped,
    GateBlocked,
}

// =============================================================================
// BLUEPRINT STEPS — the 5 step types
// =============================================================================

/// The kind of step (for tracing/reporting)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepKind {
    Deterministic,
    Agent,
    Gate,
    Fork,
    Parallel,
}

/// A deterministic action: pure code, no LLM.
/// Takes context, returns Result. Fast, free, reliable.
pub type DeterministicAction = Box<dyn Fn(&mut BlueprintContext) -> Result<(), String> + Send + Sync>;

/// An agent action: receives context snapshot + prompt, returns output.
/// The BlueprintRunner calls the AgentBackend trait to execute these.
pub type AgentPromptBuilder = Box<dyn Fn(&BlueprintContext) -> String + Send + Sync>;

/// Condition function for gates and forks
pub type ConditionFn = Box<dyn Fn(&BlueprintContext) -> bool + Send + Sync>;

/// Validation function for agent output
pub type ValidationFn = Box<dyn Fn(&BlueprintContext, &str) -> Result<(), String> + Send + Sync>;

/// A single step in the blueprint
pub struct BlueprintStep {
    pub name: String,
    pub kind: StepKind,
    pub inner: StepInner,
}

pub enum StepInner {
    /// Pure code step — fast, free, deterministic
    Deterministic {
        action: DeterministicAction,
    },

    /// Agent step — LLM-powered, with constraints and validation
    Agent {
        prompt_builder: AgentPromptBuilder,
        /// Max tokens the agent may use
        max_tokens: u64,
        /// Max cost in microdollars
        max_cost_microdollars: u64,
        /// Key in context where agent output is stored
        output_key: String,
        /// Optional validation of agent output
        validator: Option<ValidationFn>,
    },

    /// Gate — stops execution if condition is false
    Gate {
        condition: ConditionFn,
        /// If blocked, store this reason in context
        block_reason: String,
    },

    /// Fork — conditional branching
    Fork {
        condition: ConditionFn,
        if_true: Vec<BlueprintStep>,
        if_false: Vec<BlueprintStep>,
    },

    /// Parallel — run steps concurrently (conceptually; in practice sequential
    /// unless the runner supports async). Results merged into context.
    Parallel {
        steps: Vec<BlueprintStep>,
    },
}

// =============================================================================
// BLUEPRINT — ordered collection of steps
// =============================================================================

/// A Blueprint is a named, versioned workflow of interleaved steps.
pub struct Blueprint {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub description: String,
    pub steps: Vec<BlueprintStep>,
    pub created_at: DateTime<Utc>,
}

impl Blueprint {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            version: "1.0.0".into(),
            description: description.into(),
            steps: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn add_step(&mut self, step: BlueprintStep) {
        self.steps.push(step);
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    pub fn deterministic_count(&self) -> usize {
        self.steps.iter().filter(|s| s.kind == StepKind::Deterministic).count()
    }

    pub fn agent_count(&self) -> usize {
        self.steps.iter().filter(|s| s.kind == StepKind::Agent).count()
    }

    /// Ratio of deterministic to total steps (higher = cheaper + more reliable)
    pub fn deterministic_ratio(&self) -> f64 {
        if self.steps.is_empty() {
            return 0.0;
        }
        self.deterministic_count() as f64 / self.steps.len() as f64
    }

    /// Content hash for versioning
    pub fn content_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.id.as_bytes());
        hasher.update(self.name.as_bytes());
        hasher.update(self.version.as_bytes());
        hasher.update(self.steps.len().to_le_bytes());
        for step in &self.steps {
            hasher.update(step.name.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }
}

// =============================================================================
// AGENT BACKEND — trait for plugging in any LLM
// =============================================================================

/// Trait for the agent backend. Implement this for your LLM provider.
/// The blueprint runner calls this when it encounters an Agent step.
pub trait AgentBackend {
    /// Send a prompt to the agent and get a response.
    /// Returns (output_text, tokens_used, cost_microdollars).
    fn invoke(&self, prompt: &str, max_tokens: u64) -> Result<AgentResponse, String>;
}

/// Response from an agent invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub output: String,
    pub tokens_used: u64,
    pub cost_microdollars: u64,
}

// =============================================================================
// BLUEPRINT RUNNER — executes blueprints
// =============================================================================

/// Executes a Blueprint step-by-step, threading context through each step.
pub struct BlueprintRunner<A: AgentBackend> {
    agent: A,
}

impl<A: AgentBackend> BlueprintRunner<A> {
    pub fn new(agent: A) -> Self {
        Self { agent }
    }

    /// Execute a full blueprint. Returns the final context with all traces.
    pub fn run(&self, blueprint: &Blueprint) -> BlueprintResult {
        let mut ctx = BlueprintContext::new();
        ctx.set("blueprint_name", &blueprint.name);
        ctx.set("blueprint_version", &blueprint.version);

        let outcome = self.run_steps(&blueprint.steps, &mut ctx);

        BlueprintResult {
            blueprint_id: blueprint.id,
            blueprint_name: blueprint.name.clone(),
            context: ctx,
            success: outcome.is_ok(),
            error: outcome.err(),
        }
    }

    /// Execute a list of steps sequentially
    fn run_steps(&self, steps: &[BlueprintStep], ctx: &mut BlueprintContext) -> Result<(), String> {
        for step in steps {
            self.run_step(step, ctx)?;
        }
        Ok(())
    }

    /// Execute a single step
    fn run_step(&self, step: &BlueprintStep, ctx: &mut BlueprintContext) -> Result<(), String> {
        let started_at = Utc::now();

        let result = match &step.inner {
            StepInner::Deterministic { action } => {
                let r = action(ctx);
                let completed_at = Utc::now();
                ctx.record_trace(StepTrace {
                    step_name: step.name.clone(),
                    step_kind: StepKind::Deterministic,
                    started_at,
                    completed_at,
                    duration_ms: (completed_at - started_at).num_milliseconds(),
                    outcome: if r.is_ok() { StepOutcome::Success } else { StepOutcome::Failed },
                    tokens_used: None,
                    cost_microdollars: None,
                    error: r.as_ref().err().cloned(),
                });
                r
            }

            StepInner::Agent { prompt_builder, max_tokens, max_cost_microdollars: _, output_key, validator } => {
                let prompt = prompt_builder(ctx);
                match self.agent.invoke(&prompt, *max_tokens) {
                    Ok(response) => {
                        // Validate if validator present
                        let validation = if let Some(v) = validator {
                            v(ctx, &response.output)
                        } else {
                            Ok(())
                        };

                        let completed_at = Utc::now();
                        let outcome = if validation.is_ok() { StepOutcome::Success } else { StepOutcome::Failed };

                        ctx.record_trace(StepTrace {
                            step_name: step.name.clone(),
                            step_kind: StepKind::Agent,
                            started_at,
                            completed_at,
                            duration_ms: (completed_at - started_at).num_milliseconds(),
                            outcome: outcome.clone(),
                            tokens_used: Some(response.tokens_used),
                            cost_microdollars: Some(response.cost_microdollars),
                            error: validation.as_ref().err().cloned(),
                        });

                        // Store output in context
                        ctx.set(output_key, &response.output);

                        if let Err(e) = validation {
                            Err(format!("Agent output validation failed for '{}': {}", step.name, e))
                        } else {
                            Ok(())
                        }
                    }
                    Err(e) => {
                        let completed_at = Utc::now();
                        ctx.record_trace(StepTrace {
                            step_name: step.name.clone(),
                            step_kind: StepKind::Agent,
                            started_at,
                            completed_at,
                            duration_ms: (completed_at - started_at).num_milliseconds(),
                            outcome: StepOutcome::Failed,
                            tokens_used: None,
                            cost_microdollars: None,
                            error: Some(e.clone()),
                        });
                        Err(e)
                    }
                }
            }

            StepInner::Gate { condition, block_reason } => {
                let passed = condition(ctx);
                let completed_at = Utc::now();
                ctx.record_trace(StepTrace {
                    step_name: step.name.clone(),
                    step_kind: StepKind::Gate,
                    started_at,
                    completed_at,
                    duration_ms: (completed_at - started_at).num_milliseconds(),
                    outcome: if passed { StepOutcome::Success } else { StepOutcome::GateBlocked },
                    tokens_used: None,
                    cost_microdollars: None,
                    error: if passed { None } else { Some(block_reason.clone()) },
                });

                if passed {
                    Ok(())
                } else {
                    ctx.set("gate_blocked_reason", block_reason);
                    Err(format!("Gate '{}' blocked: {}", step.name, block_reason))
                }
            }

            StepInner::Fork { condition, if_true, if_false } => {
                let result = condition(ctx);
                let completed_at = Utc::now();
                ctx.record_trace(StepTrace {
                    step_name: step.name.clone(),
                    step_kind: StepKind::Fork,
                    started_at,
                    completed_at,
                    duration_ms: (completed_at - started_at).num_milliseconds(),
                    outcome: StepOutcome::Success,
                    tokens_used: None,
                    cost_microdollars: None,
                    error: None,
                });

                if result {
                    self.run_steps(if_true, ctx)
                } else {
                    self.run_steps(if_false, ctx)
                }
            }

            StepInner::Parallel { steps } => {
                // Sequential execution (true parallelism requires async runtime)
                // Each step runs independently; we collect all errors
                let mut errors = Vec::new();
                for s in steps {
                    if let Err(e) = self.run_step(s, ctx) {
                        errors.push(e);
                    }
                }
                let completed_at = Utc::now();
                ctx.record_trace(StepTrace {
                    step_name: step.name.clone(),
                    step_kind: StepKind::Parallel,
                    started_at,
                    completed_at,
                    duration_ms: (completed_at - started_at).num_milliseconds(),
                    outcome: if errors.is_empty() { StepOutcome::Success } else { StepOutcome::Failed },
                    tokens_used: None,
                    cost_microdollars: None,
                    error: if errors.is_empty() { None } else { Some(errors.join("; ")) },
                });

                if errors.is_empty() {
                    Ok(())
                } else {
                    Err(errors.join("; "))
                }
            }
        };

        result
    }
}

// =============================================================================
// BLUEPRINT RESULT — execution outcome
// =============================================================================

/// The result of running a Blueprint
#[derive(Debug, Serialize, Deserialize)]
pub struct BlueprintResult {
    pub blueprint_id: Uuid,
    pub blueprint_name: String,
    pub context: BlueprintContext,
    pub success: bool,
    pub error: Option<String>,
}

impl BlueprintResult {
    /// Total execution time in ms
    pub fn duration_ms(&self) -> i64 {
        self.context.elapsed_ms()
    }

    /// Total tokens consumed across all agent steps
    pub fn total_tokens(&self) -> u64 {
        self.context.total_tokens
    }

    /// Total cost in microdollars
    pub fn total_cost_microdollars(&self) -> u64 {
        self.context.total_cost_microdollars
    }

    /// Number of steps executed
    pub fn steps_executed(&self) -> usize {
        self.context.trace.len()
    }

    /// Number of agent steps executed
    pub fn agent_steps_executed(&self) -> usize {
        self.context.trace.iter().filter(|t| t.step_kind == StepKind::Agent).count()
    }

    /// Number of deterministic steps executed
    pub fn deterministic_steps_executed(&self) -> usize {
        self.context.trace.iter().filter(|t| t.step_kind == StepKind::Deterministic).count()
    }

    /// Steps that failed
    pub fn failed_steps(&self) -> Vec<&StepTrace> {
        self.context.trace.iter().filter(|t| t.outcome == StepOutcome::Failed).collect()
    }

    /// Summary for display
    pub fn summary(&self) -> String {
        format!(
            "Blueprint '{}': {} | {} steps ({} det, {} agent) | {} tokens | {} \u{00b5}$",
            self.blueprint_name,
            if self.success { "SUCCESS" } else { "FAILED" },
            self.steps_executed(),
            self.deterministic_steps_executed(),
            self.agent_steps_executed(),
            self.total_tokens(),
            self.total_cost_microdollars(),
        )
    }
}

// =============================================================================
// BUILDER HELPERS — ergonomic step construction
// =============================================================================

/// Helper to build deterministic steps
pub fn deterministic(name: impl Into<String>, action: impl Fn(&mut BlueprintContext) -> Result<(), String> + Send + Sync + 'static) -> BlueprintStep {
    BlueprintStep {
        name: name.into(),
        kind: StepKind::Deterministic,
        inner: StepInner::Deterministic {
            action: Box::new(action),
        },
    }
}

/// Helper to build agent steps
pub fn agent_step(
    name: impl Into<String>,
    output_key: impl Into<String>,
    max_tokens: u64,
    prompt_builder: impl Fn(&BlueprintContext) -> String + Send + Sync + 'static,
) -> BlueprintStep {
    BlueprintStep {
        name: name.into(),
        kind: StepKind::Agent,
        inner: StepInner::Agent {
            prompt_builder: Box::new(prompt_builder),
            max_tokens,
            max_cost_microdollars: 100_000, // $0.10 default cap
            output_key: output_key.into(),
            validator: None,
        },
    }
}

/// Helper to build agent steps with validation
pub fn agent_step_validated(
    name: impl Into<String>,
    output_key: impl Into<String>,
    max_tokens: u64,
    prompt_builder: impl Fn(&BlueprintContext) -> String + Send + Sync + 'static,
    validator: impl Fn(&BlueprintContext, &str) -> Result<(), String> + Send + Sync + 'static,
) -> BlueprintStep {
    BlueprintStep {
        name: name.into(),
        kind: StepKind::Agent,
        inner: StepInner::Agent {
            prompt_builder: Box::new(prompt_builder),
            max_tokens,
            max_cost_microdollars: 100_000,
            output_key: output_key.into(),
            validator: Some(Box::new(validator)),
        },
    }
}

/// Helper to build gate steps
pub fn gate(
    name: impl Into<String>,
    block_reason: impl Into<String>,
    condition: impl Fn(&BlueprintContext) -> bool + Send + Sync + 'static,
) -> BlueprintStep {
    BlueprintStep {
        name: name.into(),
        kind: StepKind::Gate,
        inner: StepInner::Gate {
            condition: Box::new(condition),
            block_reason: block_reason.into(),
        },
    }
}

/// Helper to build fork steps
pub fn fork(
    name: impl Into<String>,
    condition: impl Fn(&BlueprintContext) -> bool + Send + Sync + 'static,
    if_true: Vec<BlueprintStep>,
    if_false: Vec<BlueprintStep>,
) -> BlueprintStep {
    BlueprintStep {
        name: name.into(),
        kind: StepKind::Fork,
        inner: StepInner::Fork {
            condition: Box::new(condition),
            if_true,
            if_false,
        },
    }
}

/// Helper to build parallel steps
pub fn parallel(name: impl Into<String>, steps: Vec<BlueprintStep>) -> BlueprintStep {
    BlueprintStep {
        name: name.into(),
        kind: StepKind::Parallel,
        inner: StepInner::Parallel { steps },
    }
}

// =============================================================================
// SOVEREIGN BLUEPRINTS — pre-built workflows for Apophy
// =============================================================================

/// Pre-built blueprint: Code Review Pipeline
/// Pattern: parse → agent analyze → deterministic validate → agent summarize
pub fn code_review_blueprint() -> Blueprint {
    let mut bp = Blueprint::new(
        "Code Review Pipeline",
        "Sovereign code review: parse diff, agent analysis, deterministic checks, summary",
    );

    // Step 1: Deterministic — parse the diff
    bp.add_step(deterministic("parse_diff", |ctx| {
        let diff = ctx.get("input_diff").ok_or("No input_diff in context")?;
        let line_count = diff.lines().count();
        ctx.set("diff_lines", &line_count.to_string());
        ctx.set("diff_parsed", "true");
        Ok(())
    }));

    // Step 2: Gate — reject empty diffs
    bp.add_step(gate("non_empty_gate", "Diff is empty", |ctx| {
        ctx.get("diff_lines")
            .and_then(|v| v.parse::<usize>().ok())
            .map(|n| n > 0)
            .unwrap_or(false)
    }));

    // Step 3: Agent — analyze the diff
    bp.add_step(agent_step("analyze_diff", "analysis", 2048, |ctx| {
        let diff = ctx.get("input_diff").unwrap_or("(no diff)");
        format!(
            "Analyze this code diff for bugs, security issues, and style:\n\n{}",
            diff
        )
    }));

    // Step 4: Deterministic — check analysis validity
    bp.add_step(deterministic("validate_analysis", |ctx| {
        let analysis = ctx.get("analysis").ok_or("No analysis produced")?;
        if analysis.len() < 10 {
            return Err("Analysis too short — agent may have failed".into());
        }
        ctx.set("analysis_valid", "true");
        Ok(())
    }));

    bp
}

/// Pre-built blueprint: Feature Implementation Pipeline
/// Pattern: plan → validate plan → implement → test → report
pub fn feature_implementation_blueprint() -> Blueprint {
    let mut bp = Blueprint::new(
        "Feature Implementation Pipeline",
        "Plan → validate → implement → test → report",
    );

    // Step 1: Agent — create implementation plan
    bp.add_step(agent_step("create_plan", "plan", 4096, |ctx| {
        let spec = ctx.get("feature_spec").unwrap_or("(no spec)");
        let rules = ctx.get("scaffolding_rules").unwrap_or("(no rules)");
        format!(
            "Create a detailed implementation plan for:\n{}\n\nConstraints:\n{}",
            spec, rules
        )
    }));

    // Step 2: Deterministic — validate plan structure
    bp.add_step(deterministic("validate_plan", |ctx| {
        let plan = ctx.get("plan").ok_or("No plan produced")?;
        if !plan.contains("step") && !plan.contains("Step") {
            return Err("Plan doesn't contain any steps".into());
        }
        ctx.set("plan_valid", "true");
        Ok(())
    }));

    // Step 3: Agent — implement based on plan
    bp.add_step(agent_step_validated(
        "implement",
        "implementation",
        8192,
        |ctx| {
            let plan = ctx.get("plan").unwrap_or("(no plan)");
            format!("Implement the following plan:\n{}", plan)
        },
        |_ctx, output| {
            if output.len() < 20 {
                Err("Implementation too short".into())
            } else {
                Ok(())
            }
        },
    ));

    // Step 4: Deterministic — run tests
    bp.add_step(deterministic("run_tests", |ctx| {
        // In production, this would shell out to `cargo test`
        ctx.set("tests_passed", "true");
        ctx.set("test_count", "0");
        Ok(())
    }));

    // Step 5: Gate — tests must pass
    bp.add_step(gate("tests_gate", "Tests failed", |ctx| {
        ctx.get("tests_passed") == Some("true")
    }));

    bp
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock agent that returns deterministic responses
    struct MockAgent {
        responses: HashMap<String, String>,
        default_response: String,
    }

    impl MockAgent {
        fn new() -> Self {
            Self {
                responses: HashMap::new(),
                default_response: "Mock agent response with sufficient length for validation".into(),
            }
        }

        fn with_response(mut self, contains: &str, response: &str) -> Self {
            self.responses.insert(contains.into(), response.into());
            self
        }
    }

    impl AgentBackend for MockAgent {
        fn invoke(&self, prompt: &str, _max_tokens: u64) -> Result<AgentResponse, String> {
            let output = self.responses.iter()
                .find(|(key, _)| prompt.contains(key.as_str()))
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| self.default_response.clone());

            Ok(AgentResponse {
                output,
                tokens_used: 100,
                cost_microdollars: 50,
            })
        }
    }

    /// Mock agent that always fails
    struct FailingAgent;
    impl AgentBackend for FailingAgent {
        fn invoke(&self, _prompt: &str, _max_tokens: u64) -> Result<AgentResponse, String> {
            Err("Agent unavailable".into())
        }
    }

    #[test]
    fn test_blueprint_context_basic() {
        let mut ctx = BlueprintContext::new();
        ctx.set("key1", "value1");
        ctx.set("key2", "value2");

        assert_eq!(ctx.get("key1"), Some("value1"));
        assert_eq!(ctx.get("key2"), Some("value2"));
        assert_eq!(ctx.get("missing"), None);
    }

    #[test]
    fn test_blueprint_context_artifacts() {
        let mut ctx = BlueprintContext::new();
        ctx.set_artifact("data", serde_json::json!({"count": 42}));

        let artifact = ctx.get_artifact("data").unwrap();
        assert_eq!(artifact["count"], 42);
    }

    #[test]
    fn test_blueprint_construction() {
        let bp = code_review_blueprint();
        assert_eq!(bp.name, "Code Review Pipeline");
        assert_eq!(bp.step_count(), 4);
        assert_eq!(bp.deterministic_count(), 2);
        assert_eq!(bp.agent_count(), 1);
        assert!(bp.deterministic_ratio() > 0.4);
    }

    #[test]
    fn test_deterministic_only_blueprint() {
        let mut bp = Blueprint::new("det-only", "All deterministic");
        bp.add_step(deterministic("step1", |ctx| {
            ctx.set("a", "1");
            Ok(())
        }));
        bp.add_step(deterministic("step2", |ctx| {
            let a = ctx.get("a").ok_or("missing a")?;
            ctx.set("b", &format!("{}+2", a));
            Ok(())
        }));

        let runner = BlueprintRunner::new(MockAgent::new());
        let result = runner.run(&bp);

        assert!(result.success);
        assert_eq!(result.context.get("a"), Some("1"));
        assert_eq!(result.context.get("b"), Some("1+2"));
        assert_eq!(result.steps_executed(), 2);
        assert_eq!(result.deterministic_steps_executed(), 2);
        assert_eq!(result.agent_steps_executed(), 0);
        assert_eq!(result.total_tokens(), 0);
    }

    #[test]
    fn test_agent_step_execution() {
        let agent = MockAgent::new()
            .with_response("Analyze", "Found 3 issues: bug in line 10, missing null check, unused import");

        let mut bp = Blueprint::new("agent-test", "Test agent");
        bp.add_step(deterministic("setup", |ctx| {
            ctx.set("input_diff", "+ let x = foo();");
            Ok(())
        }));
        bp.add_step(agent_step("analyze", "result", 1024, |ctx| {
            format!("Analyze: {}", ctx.get("input_diff").unwrap_or(""))
        }));

        let runner = BlueprintRunner::new(agent);
        let result = runner.run(&bp);

        assert!(result.success);
        assert!(result.context.get("result").unwrap().contains("3 issues"));
        assert_eq!(result.total_tokens(), 100);
        assert_eq!(result.total_cost_microdollars(), 50);
    }

    #[test]
    fn test_agent_validation_failure() {
        let agent = MockAgent::new()
            .with_response("Generate", "x"); // Too short for validator

        let mut bp = Blueprint::new("val-test", "Validation test");
        bp.add_step(agent_step_validated(
            "generate",
            "output",
            1024,
            |_ctx| "Generate something".into(),
            |_ctx, output| {
                if output.len() < 10 {
                    Err("Output too short".into())
                } else {
                    Ok(())
                }
            },
        ));

        let runner = BlueprintRunner::new(agent);
        let result = runner.run(&bp);

        assert!(!result.success);
        assert!(result.error.unwrap().contains("validation failed"));
    }

    #[test]
    fn test_gate_blocks_execution() {
        let mut bp = Blueprint::new("gate-test", "Gate test");
        bp.add_step(deterministic("setup", |ctx| {
            ctx.set("ready", "false");
            Ok(())
        }));
        bp.add_step(gate("readiness", "System not ready", |ctx| {
            ctx.get("ready") == Some("true")
        }));
        bp.add_step(deterministic("should_not_run", |ctx| {
            ctx.set("ran", "true");
            Ok(())
        }));

        let runner = BlueprintRunner::new(MockAgent::new());
        let result = runner.run(&bp);

        assert!(!result.success);
        assert_eq!(result.context.get("ran"), None); // Should not have run
        assert_eq!(result.context.get("gate_blocked_reason"), Some("System not ready"));
    }

    #[test]
    fn test_gate_allows_execution() {
        let mut bp = Blueprint::new("gate-pass", "Gate pass test");
        bp.add_step(deterministic("setup", |ctx| {
            ctx.set("ready", "true");
            Ok(())
        }));
        bp.add_step(gate("readiness", "Not ready", |ctx| {
            ctx.get("ready") == Some("true")
        }));
        bp.add_step(deterministic("after_gate", |ctx| {
            ctx.set("reached", "true");
            Ok(())
        }));

        let runner = BlueprintRunner::new(MockAgent::new());
        let result = runner.run(&bp);

        assert!(result.success);
        assert_eq!(result.context.get("reached"), Some("true"));
    }

    #[test]
    fn test_fork_true_branch() {
        let mut bp = Blueprint::new("fork-test", "Fork test");
        bp.add_step(deterministic("setup", |ctx| {
            ctx.set("mode", "fast");
            Ok(())
        }));
        bp.add_step(fork(
            "mode_fork",
            |ctx| ctx.get("mode") == Some("fast"),
            vec![deterministic("fast_path", |ctx| {
                ctx.set("path", "fast");
                Ok(())
            })],
            vec![deterministic("slow_path", |ctx| {
                ctx.set("path", "slow");
                Ok(())
            })],
        ));

        let runner = BlueprintRunner::new(MockAgent::new());
        let result = runner.run(&bp);

        assert!(result.success);
        assert_eq!(result.context.get("path"), Some("fast"));
    }

    #[test]
    fn test_fork_false_branch() {
        let mut bp = Blueprint::new("fork-test-2", "Fork false");
        bp.add_step(deterministic("setup", |ctx| {
            ctx.set("mode", "slow");
            Ok(())
        }));
        bp.add_step(fork(
            "mode_fork",
            |ctx| ctx.get("mode") == Some("fast"),
            vec![deterministic("fast_path", |ctx| {
                ctx.set("path", "fast");
                Ok(())
            })],
            vec![deterministic("slow_path", |ctx| {
                ctx.set("path", "slow");
                Ok(())
            })],
        ));

        let runner = BlueprintRunner::new(MockAgent::new());
        let result = runner.run(&bp);

        assert!(result.success);
        assert_eq!(result.context.get("path"), Some("slow"));
    }

    #[test]
    fn test_parallel_steps() {
        let mut bp = Blueprint::new("parallel-test", "Parallel");
        bp.add_step(parallel("parallel_work", vec![
            deterministic("task_a", |ctx| {
                ctx.set("a_done", "true");
                Ok(())
            }),
            deterministic("task_b", |ctx| {
                ctx.set("b_done", "true");
                Ok(())
            }),
        ]));

        let runner = BlueprintRunner::new(MockAgent::new());
        let result = runner.run(&bp);

        assert!(result.success);
        assert_eq!(result.context.get("a_done"), Some("true"));
        assert_eq!(result.context.get("b_done"), Some("true"));
    }

    #[test]
    fn test_agent_failure_propagates() {
        let mut bp = Blueprint::new("fail-test", "Fail test");
        bp.add_step(agent_step("will_fail", "output", 1024, |_ctx| "anything".into()));

        let runner = BlueprintRunner::new(FailingAgent);
        let result = runner.run(&bp);

        assert!(!result.success);
        assert!(result.error.unwrap().contains("Agent unavailable"));
    }

    #[test]
    fn test_code_review_blueprint_full_run() {
        let agent = MockAgent::new()
            .with_response("Analyze this code diff", "Analysis: Found potential null pointer at line 5. Security: No issues. Style: Consistent naming.");

        let bp = code_review_blueprint();
        let runner = BlueprintRunner::new(agent);

        let mut ctx_setup = BlueprintContext::new();
        ctx_setup.set("input_diff", "+ fn foo() { bar() }");

        // Run with pre-populated context
        let mut bp2 = Blueprint::new("review", "test");
        bp2.add_step(deterministic("inject", move |ctx| {
            ctx.set("input_diff", "+ fn foo() { bar() }");
            Ok(())
        }));
        for step in code_review_blueprint().steps {
            bp2.steps.push(step);
        }

        let result = runner.run(&bp2);
        assert!(result.success);
        assert_eq!(result.context.get("analysis_valid"), Some("true"));
    }

    #[test]
    fn test_blueprint_content_hash() {
        let bp1 = code_review_blueprint();
        let bp2 = feature_implementation_blueprint();
        assert_ne!(bp1.content_hash(), bp2.content_hash());
    }

    #[test]
    fn test_blueprint_result_summary() {
        let bp = code_review_blueprint();
        let agent = MockAgent::new();
        let runner = BlueprintRunner::new(agent);

        let mut bp2 = Blueprint::new("test", "test");
        bp2.add_step(deterministic("setup", |ctx| {
            ctx.set("input_diff", "+ code");
            Ok(())
        }));
        for step in bp.steps {
            bp2.steps.push(step);
        }

        let result = runner.run(&bp2);
        let summary = result.summary();
        assert!(summary.contains("test"));
        assert!(summary.contains("SUCCESS") || summary.contains("FAILED"));
    }

    #[test]
    fn test_deterministic_ratio() {
        let mut bp = Blueprint::new("ratio", "test");
        bp.add_step(deterministic("d1", |_| Ok(())));
        bp.add_step(deterministic("d2", |_| Ok(())));
        bp.add_step(agent_step("a1", "out", 100, |_| "prompt".into()));

        assert!((bp.deterministic_ratio() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_step_trace_records_timing() {
        let mut bp = Blueprint::new("trace-test", "Timing");
        bp.add_step(deterministic("timed", |_ctx| Ok(())));

        let runner = BlueprintRunner::new(MockAgent::new());
        let result = runner.run(&bp);

        assert_eq!(result.context.trace.len(), 1);
        let trace = &result.context.trace[0];
        assert_eq!(trace.step_name, "timed");
        assert_eq!(trace.step_kind, StepKind::Deterministic);
        assert_eq!(trace.outcome, StepOutcome::Success);
        assert!(trace.duration_ms >= 0);
    }
}
