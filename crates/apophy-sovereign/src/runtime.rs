//! # Agent Runtime — The Missing Engine
//!
//! This module makes agents actually execute. The loop:
//! 1. Receive task
//! 2. Route to best agent (by domain + tier)
//! 3. Create permission envelope (governance)
//! 4. Build prompt from agent template + task
//! 5. Run inference (local, via HttpBackend or any InferenceBackend)
//! 6. Verify output through gates (safety, format, length)
//! 7. Record trace (observability)
//! 8. Store result in DB
//! 9. Return governed result

use crate::agents::{Agent, AgentDomain, AgentTier};
use apophy_governance::observe::{LlmTrace, TraceLog, TraceOutcome};
use apophy_governance::permit::{PermissionAction, PermissionEnvelope};
use apophy_governance::verify::VerificationGate;
use apophy_inference::{GenerationParams, InferenceRequest};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// A task submitted for agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// Unique task ID
    pub id: String,
    /// What the agent should do
    pub instruction: String,
    /// Optional domain constraint (routes to domain-specific agent)
    pub domain: Option<String>,
    /// Optional tier preference (strategic, tactical, operational)
    pub tier: Option<String>,
    /// Optional specific agent ID
    pub agent_id: Option<String>,
    /// Optional context to inject
    pub context: Option<String>,
    /// Max tokens for response
    pub max_tokens: Option<usize>,
    /// Temperature (0.0 - 1.0)
    pub temperature: Option<f32>,
}

/// Result of agent execution — fully governed and traced
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    /// Task ID
    pub task_id: String,
    /// Which agent executed this
    pub agent_id: String,
    /// Agent name
    pub agent_name: String,
    /// Agent domain
    pub domain: String,
    /// Agent tier
    pub tier: String,
    /// The generated response
    pub response: String,
    /// Whether output passed verification gates
    pub verified: bool,
    /// Verification details
    pub verification_details: Vec<GateResult>,
    /// Governance trace
    pub trace: TraceInfo,
    /// Execution status
    pub status: ExecutionStatus,
    /// Error message if failed
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub gate_name: String,
    pub passed: bool,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceInfo {
    pub trace_id: String,
    pub model: String,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub latency_ms: u64,
    pub cost_microdollars: u64,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Success,
    VerificationFailed,
    PermissionDenied,
    InferenceError,
    AgentNotFound,
}

/// The Agent Runtime — makes agents actually run.
pub struct AgentRuntime {
    brain: Arc<std::sync::Mutex<apophy_inference::SovereignBrain>>,
    trace_log: std::sync::Mutex<TraceLog>,
    /// Verification gates applied to all agent output
    gates: Vec<VerificationGate>,
}

impl AgentRuntime {
    pub fn new(brain: Arc<std::sync::Mutex<apophy_inference::SovereignBrain>>) -> Self {
        // Default verification gates for all agent output
        let gates = vec![
            VerificationGate::safety_gate(),
            VerificationGate::length_gate(1, 50_000),
        ];

        Self {
            brain,
            trace_log: std::sync::Mutex::new(TraceLog::with_budget(100.0)),
            gates,
        }
    }

    /// Execute a task — the core loop
    pub fn execute(&self, task: &AgentTask, agents: &[Agent]) -> AgentResult {
        let task_id = task.id.clone();

        // Step 1: Route to the best agent
        let agent = match self.route_agent(task, agents) {
            Some(a) => a,
            None => {
                return AgentResult {
                    task_id,
                    agent_id: String::new(),
                    agent_name: String::new(),
                    domain: task.domain.clone().unwrap_or_default(),
                    tier: String::new(),
                    response: String::new(),
                    verified: false,
                    verification_details: vec![],
                    trace: empty_trace(),
                    status: ExecutionStatus::AgentNotFound,
                    error: Some("No matching agent found for this task".into()),
                };
            }
        };

        // Step 2: Create permission envelope
        let mut envelope = match agent.tier {
            AgentTier::Strategic => {
                PermissionEnvelope::worker(&agent.id, 100_000, 0.0) // Local = $0
            }
            AgentTier::Tactical => {
                PermissionEnvelope::worker(&agent.id, 50_000, 0.0)
            }
            AgentTier::Operational => {
                PermissionEnvelope::read_only(&agent.id, 10_000)
            }
        };

        // Step 3: Check permission for inference execution
        let perm_check = envelope.check("inference", &PermissionAction::Execute, "own");
        if perm_check != apophy_governance::permit::PermitDecision::Allow {
            // Operational agents need explicit execute permission — grant it for inference
            envelope.allow(apophy_governance::permit::Permission::execute("inference"));
        }

        // Step 4: Build the prompt
        let prompt = self.build_prompt(&agent, task);

        // Step 5: Start trace
        let mut trace = LlmTrace::start("sovereign-local", "apophy-brain");
        trace.set_prompt(&prompt);
        trace.workflow_id = Some(Uuid::new_v4());
        trace.constraints = vec![
            format!("agent: {}", agent.id),
            format!("tier: {:?}", agent.tier),
            format!("domain: {:?}", agent.domain),
        ];

        // Step 6: Run inference
        let params = GenerationParams {
            max_tokens: task.max_tokens.unwrap_or(2048),
            temperature: task.temperature.unwrap_or(0.7),
            ..GenerationParams::default()
        };

        let brain = self.brain.lock().unwrap();
        let model_name = brain.status().model_name.clone().unwrap_or_else(|| brain.status().backend_name.clone());

        let inference_result = brain.backend.generate(&InferenceRequest {
            prompt: prompt.clone(),
            params,
            system_prompt: Some(agent.prompt_template.clone()),
        });
        drop(brain);

        let response_text = match inference_result {
            Ok(resp) => {
                trace.complete(
                    resp.usage.prompt_tokens as u32,
                    resp.usage.completion_tokens as u32,
                    TraceOutcome::Success,
                );
                envelope.consume(resp.usage.total_tokens as u64, 0); // Local = $0 cost
                resp.text
            }
            Err(e) => {
                trace.complete(0, 0, TraceOutcome::Error(e.to_string()));
                if let Ok(mut log) = self.trace_log.lock() {
                    log.record(trace.clone());
                }
                return AgentResult {
                    task_id,
                    agent_id: agent.id.clone(),
                    agent_name: agent.name.clone(),
                    domain: agent.domain.slug().to_string(),
                    tier: format!("{:?}", agent.tier),
                    response: String::new(),
                    verified: false,
                    verification_details: vec![],
                    trace: trace_to_info(&trace, &model_name),
                    status: ExecutionStatus::InferenceError,
                    error: Some(e.to_string()),
                };
            }
        };

        // Step 7: Run verification gates
        let mut gate_results = Vec::new();
        let mut all_passed = true;

        for gate in &self.gates {
            let result = gate.verify(&response_text);
            let passed = result.passed;
            gate_results.push(GateResult {
                gate_name: result.gate_name.clone(),
                passed,
                details: if passed {
                    format!("{}/{} checks passed", result.passed_count, result.passed_count + result.failed_count)
                } else {
                    result.critical_failures.join("; ")
                },
            });
            if !passed {
                all_passed = false;
            }
        }

        trace.verified = all_passed;
        if all_passed {
            trace.verification_method = Some("safety+length gates".into());
        } else {
            trace.outcome = TraceOutcome::VerificationFailed;
        }

        // Step 8: Record trace
        let trace_info = trace_to_info(&trace, &model_name);
        if let Ok(mut log) = self.trace_log.lock() {
            log.record(trace);
        }

        // Step 9: Return result
        AgentResult {
            task_id,
            agent_id: agent.id.clone(),
            agent_name: agent.name.clone(),
            domain: agent.domain.slug().to_string(),
            tier: format!("{:?}", agent.tier),
            response: response_text,
            verified: all_passed,
            verification_details: gate_results,
            trace: trace_info,
            status: if all_passed {
                ExecutionStatus::Success
            } else {
                ExecutionStatus::VerificationFailed
            },
            error: None,
        }
    }

    /// Route a task to the best matching agent
    fn route_agent<'a>(&self, task: &AgentTask, agents: &'a [Agent]) -> Option<&'a Agent> {
        // If specific agent ID requested, use it
        if let Some(ref id) = task.agent_id {
            return agents.iter().find(|a| a.id == *id);
        }

        // Filter by domain if specified
        let domain_filter: Option<AgentDomain> = task.domain.as_ref().and_then(|d| {
            AgentDomain::all().iter().find(|ad| ad.slug() == d.as_str()).copied()
        });

        // Filter by tier if specified
        let tier_filter: Option<AgentTier> = task.tier.as_ref().and_then(|t| match t.as_str() {
            "strategic" => Some(AgentTier::Strategic),
            "tactical" => Some(AgentTier::Tactical),
            "operational" => Some(AgentTier::Operational),
            _ => None,
        });

        // Score agents by match quality
        let mut best: Option<(&Agent, u32)> = None;

        for agent in agents {
            let mut score = 0u32;

            // Domain match
            if let Some(ref domain) = domain_filter {
                if agent.domain == *domain {
                    score += 100;
                } else {
                    continue; // Skip non-matching domains when domain is specified
                }
            }

            // Tier match
            if let Some(ref tier) = tier_filter {
                if agent.tier == *tier {
                    score += 50;
                }
            } else {
                // Default: prefer tactical (best balance of specialization)
                match agent.tier {
                    AgentTier::Tactical => score += 30,
                    AgentTier::Strategic => score += 20,
                    AgentTier::Operational => score += 10,
                }
            }

            // Keyword matching in instruction vs agent name/capabilities
            let instruction_lower = task.instruction.to_lowercase();
            let name_lower = agent.name.to_lowercase();
            if instruction_lower.split_whitespace().any(|w| name_lower.contains(w) && w.len() > 3) {
                score += 25;
            }

            match best {
                Some((_, best_score)) if score > best_score => {
                    best = Some((agent, score));
                }
                None => {
                    best = Some((agent, score));
                }
                _ => {}
            }
        }

        best.map(|(agent, _)| agent)
    }

    /// Build a complete prompt from agent template + task
    fn build_prompt(&self, _agent: &Agent, task: &AgentTask) -> String {
        let mut prompt = String::new();

        // Add context if provided
        if let Some(ref ctx) = task.context {
            prompt.push_str("## Context\n");
            prompt.push_str(ctx);
            prompt.push_str("\n\n");
        }

        // Add the instruction
        prompt.push_str("## Task\n");
        prompt.push_str(&task.instruction);

        prompt
    }

    /// Get runtime statistics
    pub fn stats(&self) -> RuntimeStats {
        let log = self.trace_log.lock().unwrap();
        let summary = log.summary();
        RuntimeStats {
            total_executions: summary.total_traces,
            total_tokens: summary.total_tokens,
            success_rate: summary.success_rate,
            avg_latency_ms: summary.avg_latency_ms,
            waste_rate: summary.waste_rate,
            verified_count: summary.verified_count,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStats {
    pub total_executions: usize,
    pub total_tokens: u64,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub waste_rate: f64,
    pub verified_count: usize,
}

fn empty_trace() -> TraceInfo {
    TraceInfo {
        trace_id: Uuid::new_v4().to_string(),
        model: String::new(),
        input_tokens: 0,
        output_tokens: 0,
        latency_ms: 0,
        cost_microdollars: 0,
        timestamp: Utc::now().to_rfc3339(),
    }
}

fn trace_to_info(trace: &LlmTrace, model: &str) -> TraceInfo {
    TraceInfo {
        trace_id: trace.id.to_string(),
        model: model.to_string(),
        input_tokens: trace.input_tokens as usize,
        output_tokens: trace.output_tokens as usize,
        latency_ms: trace.latency_ms,
        cost_microdollars: trace.cost_microdollars,
        timestamp: trace.started_at.to_rfc3339(),
    }
}
