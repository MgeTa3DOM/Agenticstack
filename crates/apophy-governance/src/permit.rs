//! # Permission Envelopes — Zero-Trust Agent Boundaries
//!
//! "Un agent sans enveloppe de permission est un employe
//! avec la carte de credit de l'entreprise et aucune limite."
//!
//! Every agent action is wrapped in a permission envelope that specifies:
//! - What the agent CAN do (allowed actions)
//! - What it CANNOT do (denied actions)
//! - Resource limits (tokens, cost, time)
//! - Escalation rules (when to ask a human)
//!
//! The envelope is checked BEFORE execution, not after.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// A permission — a single allowed or denied action.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission {
    /// What resource/action this permission covers
    pub resource: String,
    /// The action (read, write, execute, delete, etc.)
    pub action: PermissionAction,
    /// Scope constraint (e.g., "own_data", "team_data", "all")
    pub scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PermissionAction {
    Read,
    Write,
    Execute,
    Delete,
    Approve,
    Escalate,
}

/// The Permission Envelope — wraps every agent execution context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionEnvelope {
    /// Envelope ID
    pub id: Uuid,
    /// Who/what this envelope is for
    pub agent_id: String,
    /// When this envelope was created
    pub created_at: DateTime<Utc>,
    /// When this envelope expires
    pub expires_at: DateTime<Utc>,
    /// Explicitly allowed permissions
    pub allowed: HashSet<Permission>,
    /// Explicitly denied permissions (override allowed)
    pub denied: HashSet<Permission>,
    /// Max tokens this agent can consume
    pub max_tokens: u64,
    /// Max cost in microdollars
    pub max_cost_microdollars: u64,
    /// Max execution time in seconds
    pub max_duration_secs: u64,
    /// Tokens consumed so far
    pub tokens_used: u64,
    /// Cost consumed so far
    pub cost_used_microdollars: u64,
    /// Actions that require human escalation
    pub escalation_triggers: Vec<String>,
    /// Audit trail of decisions
    pub audit_log: Vec<PermitAuditEntry>,
}

/// A record of a permission decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermitAuditEntry {
    pub timestamp: DateTime<Utc>,
    pub resource: String,
    pub action: PermissionAction,
    pub decision: PermitDecision,
    pub reason: String,
}

/// The decision on a permission check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PermitDecision {
    /// Action allowed
    Allow,
    /// Action denied (explicit deny)
    Deny,
    /// Action denied (not in allowed set)
    NotPermitted,
    /// Envelope expired
    Expired,
    /// Token budget exceeded
    TokenBudgetExceeded,
    /// Cost budget exceeded
    CostBudgetExceeded,
    /// Requires human escalation
    Escalate,
}

impl Permission {
    pub fn new(resource: impl Into<String>, action: PermissionAction, scope: impl Into<String>) -> Self {
        Self {
            resource: resource.into(),
            action,
            scope: scope.into(),
        }
    }

    pub fn read(resource: impl Into<String>) -> Self {
        Self::new(resource, PermissionAction::Read, "own")
    }

    pub fn write(resource: impl Into<String>) -> Self {
        Self::new(resource, PermissionAction::Write, "own")
    }

    pub fn execute(resource: impl Into<String>) -> Self {
        Self::new(resource, PermissionAction::Execute, "own")
    }
}

impl PermissionEnvelope {
    /// Create a new envelope with a TTL in seconds.
    pub fn new(agent_id: impl Into<String>, ttl_secs: i64) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            agent_id: agent_id.into(),
            created_at: now,
            expires_at: now + Duration::seconds(ttl_secs),
            allowed: HashSet::new(),
            denied: HashSet::new(),
            max_tokens: 0,
            max_cost_microdollars: 0,
            max_duration_secs: 0,
            tokens_used: 0,
            cost_used_microdollars: 0,
            escalation_triggers: Vec::new(),
            audit_log: Vec::new(),
        }
    }

    /// Allow a permission.
    pub fn allow(&mut self, perm: Permission) {
        self.allowed.insert(perm);
    }

    /// Deny a permission (overrides allow).
    pub fn deny(&mut self, perm: Permission) {
        self.denied.insert(perm);
    }

    /// Set token budget.
    pub fn with_token_budget(mut self, max_tokens: u64) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set cost budget in dollars.
    pub fn with_cost_budget(mut self, max_dollars: f64) -> Self {
        self.max_cost_microdollars = (max_dollars * 1_000_000.0) as u64;
        self
    }

    /// Set max duration.
    pub fn with_max_duration(mut self, secs: u64) -> Self {
        self.max_duration_secs = secs;
        self
    }

    /// Add an escalation trigger.
    pub fn add_escalation_trigger(&mut self, trigger: impl Into<String>) {
        self.escalation_triggers.push(trigger.into());
    }

    /// Check if an action is permitted.
    pub fn check(&mut self, resource: &str, action: &PermissionAction, scope: &str) -> PermitDecision {
        let perm = Permission {
            resource: resource.to_string(),
            action: action.clone(),
            scope: scope.to_string(),
        };

        // 1. Check expiry
        if Utc::now() > self.expires_at {
            self.log_decision(&perm, PermitDecision::Expired, "Envelope expired");
            return PermitDecision::Expired;
        }

        // 2. Check token budget
        if self.max_tokens > 0 && self.tokens_used >= self.max_tokens {
            self.log_decision(&perm, PermitDecision::TokenBudgetExceeded, "Token budget exceeded");
            return PermitDecision::TokenBudgetExceeded;
        }

        // 3. Check cost budget
        if self.max_cost_microdollars > 0 && self.cost_used_microdollars >= self.max_cost_microdollars {
            self.log_decision(&perm, PermitDecision::CostBudgetExceeded, "Cost budget exceeded");
            return PermitDecision::CostBudgetExceeded;
        }

        // 4. Check escalation triggers
        for trigger in &self.escalation_triggers {
            if resource.contains(trigger.as_str()) {
                self.log_decision(&perm, PermitDecision::Escalate, "Escalation trigger matched");
                return PermitDecision::Escalate;
            }
        }

        // 5. Explicit deny overrides everything
        if self.denied.contains(&perm) {
            self.log_decision(&perm, PermitDecision::Deny, "Explicitly denied");
            return PermitDecision::Deny;
        }

        // 6. Check if explicitly allowed
        if self.allowed.contains(&perm) {
            self.log_decision(&perm, PermitDecision::Allow, "Explicitly allowed");
            return PermitDecision::Allow;
        }

        // 7. Default deny (zero-trust)
        self.log_decision(&perm, PermitDecision::NotPermitted, "Not in allowed set");
        PermitDecision::NotPermitted
    }

    /// Record token/cost consumption.
    pub fn consume(&mut self, tokens: u64, cost_microdollars: u64) {
        self.tokens_used += tokens;
        self.cost_used_microdollars += cost_microdollars;
    }

    /// Is the envelope still valid?
    pub fn is_valid(&self) -> bool {
        Utc::now() <= self.expires_at
    }

    /// Remaining token budget.
    pub fn tokens_remaining(&self) -> Option<u64> {
        if self.max_tokens == 0 {
            return None;
        }
        Some(self.max_tokens.saturating_sub(self.tokens_used))
    }

    /// Remaining cost budget in dollars.
    pub fn cost_remaining_dollars(&self) -> Option<f64> {
        if self.max_cost_microdollars == 0 {
            return None;
        }
        let remaining = self.max_cost_microdollars.saturating_sub(self.cost_used_microdollars);
        Some(remaining as f64 / 1_000_000.0)
    }

    fn log_decision(&mut self, perm: &Permission, decision: PermitDecision, reason: &str) {
        self.audit_log.push(PermitAuditEntry {
            timestamp: Utc::now(),
            resource: perm.resource.clone(),
            action: perm.action.clone(),
            decision,
            reason: reason.to_string(),
        });
    }

    /// Sovereign default: read-only agent with token budget.
    pub fn read_only(agent_id: impl Into<String>, max_tokens: u64) -> Self {
        let mut env = Self::new(agent_id, 3600).with_token_budget(max_tokens);
        env.allow(Permission::read("documents"));
        env.allow(Permission::read("code"));
        env.allow(Permission::read("config"));
        env.deny(Permission::write("documents"));
        env.deny(Permission::write("code"));
        env.deny(Permission::new("system", PermissionAction::Execute, "all"));
        env
    }

    /// Sovereign default: worker agent with write access.
    pub fn worker(agent_id: impl Into<String>, max_tokens: u64, max_cost: f64) -> Self {
        let mut env = Self::new(agent_id, 7200)
            .with_token_budget(max_tokens)
            .with_cost_budget(max_cost);
        env.allow(Permission::read("documents"));
        env.allow(Permission::read("code"));
        env.allow(Permission::write("code"));
        env.allow(Permission::execute("tests"));
        env.deny(Permission::new("production", PermissionAction::Write, "all"));
        env.deny(Permission::new("production", PermissionAction::Delete, "all"));
        env.add_escalation_trigger("deploy");
        env.add_escalation_trigger("production");
        env
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_creation() {
        let perm = Permission::read("documents");
        assert_eq!(perm.resource, "documents");
        assert_eq!(perm.action, PermissionAction::Read);
        assert_eq!(perm.scope, "own");
    }

    #[test]
    fn test_envelope_creation() {
        let env = PermissionEnvelope::new("agent-1", 3600);
        assert!(env.is_valid());
        assert_eq!(env.agent_id, "agent-1");
    }

    #[test]
    fn test_allow_check() {
        let mut env = PermissionEnvelope::new("agent-1", 3600);
        env.allow(Permission::read("documents"));

        let decision = env.check("documents", &PermissionAction::Read, "own");
        assert_eq!(decision, PermitDecision::Allow);
    }

    #[test]
    fn test_deny_overrides_allow() {
        let mut env = PermissionEnvelope::new("agent-1", 3600);
        env.allow(Permission::write("code"));
        env.deny(Permission::write("code"));

        let decision = env.check("code", &PermissionAction::Write, "own");
        assert_eq!(decision, PermitDecision::Deny);
    }

    #[test]
    fn test_default_deny() {
        let mut env = PermissionEnvelope::new("agent-1", 3600);
        let decision = env.check("secrets", &PermissionAction::Read, "own");
        assert_eq!(decision, PermitDecision::NotPermitted);
    }

    #[test]
    fn test_token_budget() {
        let mut env = PermissionEnvelope::new("agent-1", 3600)
            .with_token_budget(1000);
        env.allow(Permission::read("documents"));

        env.consume(1000, 0);
        let decision = env.check("documents", &PermissionAction::Read, "own");
        assert_eq!(decision, PermitDecision::TokenBudgetExceeded);
    }

    #[test]
    fn test_cost_budget() {
        let mut env = PermissionEnvelope::new("agent-1", 3600)
            .with_cost_budget(1.0); // $1
        env.allow(Permission::read("documents"));

        env.consume(0, 1_000_000); // $1
        let decision = env.check("documents", &PermissionAction::Read, "own");
        assert_eq!(decision, PermitDecision::CostBudgetExceeded);
    }

    #[test]
    fn test_escalation_trigger() {
        let mut env = PermissionEnvelope::new("agent-1", 3600);
        env.allow(Permission::execute("deploy"));
        env.add_escalation_trigger("deploy");

        let decision = env.check("deploy", &PermissionAction::Execute, "own");
        assert_eq!(decision, PermitDecision::Escalate);
    }

    #[test]
    fn test_remaining_budgets() {
        let mut env = PermissionEnvelope::new("agent-1", 3600)
            .with_token_budget(10000)
            .with_cost_budget(5.0);

        env.consume(3000, 2_000_000);
        assert_eq!(env.tokens_remaining(), Some(7000));
        assert!((env.cost_remaining_dollars().unwrap() - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_audit_trail() {
        let mut env = PermissionEnvelope::new("agent-1", 3600);
        env.allow(Permission::read("code"));

        env.check("code", &PermissionAction::Read, "own");
        env.check("secrets", &PermissionAction::Read, "own");

        assert_eq!(env.audit_log.len(), 2);
        assert_eq!(env.audit_log[0].decision, PermitDecision::Allow);
        assert_eq!(env.audit_log[1].decision, PermitDecision::NotPermitted);
    }

    #[test]
    fn test_read_only_default() {
        let mut env = PermissionEnvelope::read_only("reader", 5000);
        assert_eq!(env.check("documents", &PermissionAction::Read, "own"), PermitDecision::Allow);
        assert_eq!(env.check("documents", &PermissionAction::Write, "own"), PermitDecision::Deny);
    }

    #[test]
    fn test_worker_default() {
        let mut env = PermissionEnvelope::worker("worker-1", 50000, 10.0);
        assert_eq!(env.check("code", &PermissionAction::Read, "own"), PermitDecision::Allow);
        assert_eq!(env.check("code", &PermissionAction::Write, "own"), PermitDecision::Allow);
        // "production" matches escalation trigger, so escalation takes priority
        assert_eq!(env.check("production", &PermissionAction::Write, "all"), PermitDecision::Escalate);
    }

    #[test]
    fn test_worker_escalation_on_deploy() {
        let mut env = PermissionEnvelope::worker("worker-1", 50000, 10.0);
        let decision = env.check("deploy", &PermissionAction::Execute, "own");
        assert_eq!(decision, PermitDecision::Escalate);
    }

    #[test]
    fn test_envelope_serialization() {
        let env = PermissionEnvelope::read_only("test", 1000);
        let json = serde_json::to_string(&env).unwrap();
        assert!(json.contains("agent_id"));
        assert!(json.contains("allowed"));
        assert!(json.contains("denied"));
    }
}
