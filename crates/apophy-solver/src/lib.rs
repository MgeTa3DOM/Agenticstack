//! # Apophy Solver — Sovereign Enterprise Problem Solver
//!
//! A complete enterprise-grade problem-solving engine that diagnoses,
//! analyzes, resolves, and tracks complex business/technical problems.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                  ENTERPRISE SOLVER                       │
//! │                                                         │
//! │  ┌──────────┐  ┌──────────┐  ┌───────────┐  ┌────────┐│
//! │  │DIAGNOSTIC│→ │RESOLUTION│→ │ IMPACT    │→ │COMPLI- ││
//! │  │  ENGINE  │  │ ENGINE   │  │ ANALYZER  │  │ANCE    ││
//! │  └──────────┘  └──────────┘  └───────────┘  └────────┘│
//! │       ↓              ↓             ↓             ↓      │
//! │  ┌──────────┐  ┌──────────┐  ┌───────────┐  ┌────────┐│
//! │  │  RISK    │  │KNOWLEDGE │  │  SLA      │  │ESCALA- ││
//! │  │ MATRIX   │  │  BASE    │  │ TRACKER   │  │TION    ││
//! │  └──────────┘  └──────────┘  └───────────┘  └────────┘│
//! └─────────────────────────────────────────────────────────┘
//! ```

pub mod diagnostic;
pub mod resolution;
pub mod impact;
pub mod compliance;
pub mod risk;
pub mod knowledge;
pub mod sla;
pub mod escalation;

pub use diagnostic::*;
pub use resolution::*;
pub use impact::*;
pub use compliance::*;
pub use risk::*;
pub use knowledge::*;
pub use sla::*;
pub use escalation::*;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

// ─── Core Types ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Domain {
    Infrastructure,
    Security,
    Performance,
    DataIntegrity,
    Compliance,
    Financial,
    Operations,
    HumanResources,
    CustomerExperience,
    SupplyChain,
    Legal,
    Strategy,
}

impl std::fmt::Display for Domain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Domain::Infrastructure => write!(f, "Infrastructure"),
            Domain::Security => write!(f, "Security"),
            Domain::Performance => write!(f, "Performance"),
            Domain::DataIntegrity => write!(f, "Data Integrity"),
            Domain::Compliance => write!(f, "Compliance"),
            Domain::Financial => write!(f, "Financial"),
            Domain::Operations => write!(f, "Operations"),
            Domain::HumanResources => write!(f, "Human Resources"),
            Domain::CustomerExperience => write!(f, "Customer Experience"),
            Domain::SupplyChain => write!(f, "Supply Chain"),
            Domain::Legal => write!(f, "Legal"),
            Domain::Strategy => write!(f, "Strategy"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    pub fn weight(&self) -> f64 {
        match self {
            Severity::Critical => 1.0,
            Severity::High => 0.75,
            Severity::Medium => 0.5,
            Severity::Low => 0.25,
            Severity::Info => 0.1,
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Critical => write!(f, "CRITICAL"),
            Severity::High => write!(f, "HIGH"),
            Severity::Medium => write!(f, "MEDIUM"),
            Severity::Low => write!(f, "LOW"),
            Severity::Info => write!(f, "INFO"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProblemStatus {
    Open,
    Diagnosing,
    SolutionProposed,
    Implementing,
    Validating,
    Resolved,
    Escalated,
    Wontfix,
}

impl std::fmt::Display for ProblemStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProblemStatus::Open => write!(f, "OPEN"),
            ProblemStatus::Diagnosing => write!(f, "DIAGNOSING"),
            ProblemStatus::SolutionProposed => write!(f, "SOLUTION_PROPOSED"),
            ProblemStatus::Implementing => write!(f, "IMPLEMENTING"),
            ProblemStatus::Validating => write!(f, "VALIDATING"),
            ProblemStatus::Resolved => write!(f, "RESOLVED"),
            ProblemStatus::Escalated => write!(f, "ESCALATED"),
            ProblemStatus::Wontfix => write!(f, "WONTFIX"),
        }
    }
}

// ─── Problem ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problem {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub domain: Domain,
    pub severity: Severity,
    pub status: ProblemStatus,
    pub reported_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub affected_systems: Vec<String>,
    pub hash: String,
}

impl Problem {
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        domain: Domain,
        severity: Severity,
        reported_by: impl Into<String>,
    ) -> Self {
        let title = title.into();
        let description = description.into();
        let reported_by = reported_by.into();
        let id = Uuid::new_v4();
        let now = Utc::now();

        let mut hasher = Sha256::new();
        hasher.update(id.to_string().as_bytes());
        hasher.update(title.as_bytes());
        hasher.update(now.to_rfc3339().as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        Self {
            id,
            title,
            description,
            domain,
            severity,
            status: ProblemStatus::Open,
            reported_by,
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
            affected_systems: Vec::new(),
            hash,
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_affected_systems(mut self, systems: Vec<String>) -> Self {
        self.affected_systems = systems;
        self
    }
}

// ─── Enterprise Solver (Main Engine) ────────────────────────

pub struct EnterpriseSolver {
    pub problems: Vec<Problem>,
    pub diagnostic_engine: DiagnosticEngine,
    pub resolution_engine: ResolutionEngine,
    pub impact_analyzer: ImpactAnalyzer,
    pub compliance_checker: ComplianceChecker,
    pub risk_matrix: RiskMatrix,
    pub knowledge_base: KnowledgeBase,
    pub sla_tracker: SlaTracker,
    pub escalation_manager: EscalationManager,
}

impl EnterpriseSolver {
    pub fn new() -> Self {
        Self {
            problems: Vec::new(),
            diagnostic_engine: DiagnosticEngine::new(),
            resolution_engine: ResolutionEngine::new(),
            impact_analyzer: ImpactAnalyzer::new(),
            compliance_checker: ComplianceChecker::new(),
            risk_matrix: RiskMatrix::new(),
            knowledge_base: KnowledgeBase::new(),
            sla_tracker: SlaTracker::new(),
            escalation_manager: EscalationManager::new(),
        }
    }

    /// Submit a new problem to the solver pipeline
    pub fn submit(&mut self, mut problem: Problem) -> SolverReport {
        tracing::info!(
            "Problem submitted: [{}] {} ({})",
            problem.severity,
            problem.title,
            problem.domain
        );

        // Phase 1: Diagnostic
        problem.status = ProblemStatus::Diagnosing;
        let diagnosis = self.diagnostic_engine.diagnose(&problem);

        // Phase 2: Risk Assessment
        let risk_assessment = self.risk_matrix.assess(&problem, &diagnosis);

        // Phase 3: Impact Analysis
        let impact = self.impact_analyzer.analyze(&problem, &diagnosis);

        // Phase 4: Compliance Check
        let compliance_result = self.compliance_checker.check(&problem, &diagnosis);

        // Phase 5: Resolution
        problem.status = ProblemStatus::SolutionProposed;
        let resolution = self.resolution_engine.resolve(&problem, &diagnosis, &risk_assessment);

        // Phase 6: SLA
        let sla = self.sla_tracker.track(&problem);

        // Phase 7: Escalation check
        let escalation = self.escalation_manager.evaluate(&problem, &risk_assessment, &sla);
        if escalation.should_escalate {
            problem.status = ProblemStatus::Escalated;
        }

        // Phase 8: Knowledge capture
        self.knowledge_base.capture(&problem, &diagnosis, &resolution);

        problem.updated_at = Utc::now();
        self.problems.push(problem.clone());

        SolverReport {
            problem,
            diagnosis,
            risk_assessment,
            impact,
            compliance_result,
            resolution,
            sla,
            escalation,
            generated_at: Utc::now(),
        }
    }

    /// Solve a problem from raw text input (convenience method)
    pub fn solve(
        &mut self,
        title: &str,
        description: &str,
        domain: Domain,
        severity: Severity,
    ) -> SolverReport {
        let problem = Problem::new(title, description, domain, severity, "system");
        self.submit(problem)
    }

    /// Get solver status summary
    pub fn status(&self) -> SolverStatus {
        let total = self.problems.len();
        let open = self.problems.iter().filter(|p| p.status == ProblemStatus::Open).count();
        let diagnosing = self.problems.iter().filter(|p| p.status == ProblemStatus::Diagnosing).count();
        let proposed = self.problems.iter().filter(|p| p.status == ProblemStatus::SolutionProposed).count();
        let resolved = self.problems.iter().filter(|p| p.status == ProblemStatus::Resolved).count();
        let escalated = self.problems.iter().filter(|p| p.status == ProblemStatus::Escalated).count();

        let critical = self.problems.iter().filter(|p| p.severity == Severity::Critical).count();
        let high = self.problems.iter().filter(|p| p.severity == Severity::High).count();

        let resolution_rate = if total > 0 {
            (resolved as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        SolverStatus {
            total_problems: total,
            open,
            diagnosing,
            solution_proposed: proposed,
            resolved,
            escalated,
            critical_count: critical,
            high_count: high,
            resolution_rate_pct: resolution_rate,
            knowledge_entries: self.knowledge_base.entries.len(),
            compliance_frameworks: self.compliance_checker.frameworks.len(),
            risk_thresholds: self.risk_matrix.thresholds.len(),
            sla_policies: self.sla_tracker.policies.len(),
        }
    }
}

impl Default for EnterpriseSolver {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Reports ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverReport {
    pub problem: Problem,
    pub diagnosis: Diagnosis,
    pub risk_assessment: RiskAssessment,
    pub impact: ImpactReport,
    pub compliance_result: ComplianceResult,
    pub resolution: Resolution,
    pub sla: SlaRecord,
    pub escalation: EscalationDecision,
    pub generated_at: DateTime<Utc>,
}

impl SolverReport {
    pub fn summary(&self) -> String {
        format!(
            "Problem: {} | Severity: {} | Root Cause: {} | Risk: {:.0}% | \
             Solutions: {} | Compliant: {} | SLA: {} | Escalated: {}",
            self.problem.title,
            self.problem.severity,
            self.diagnosis.root_cause,
            self.risk_assessment.risk_score * 100.0,
            self.resolution.solutions.len(),
            self.compliance_result.compliant,
            self.sla.status,
            self.escalation.should_escalate,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverStatus {
    pub total_problems: usize,
    pub open: usize,
    pub diagnosing: usize,
    pub solution_proposed: usize,
    pub resolved: usize,
    pub escalated: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub resolution_rate_pct: f64,
    pub knowledge_entries: usize,
    pub compliance_frameworks: usize,
    pub risk_thresholds: usize,
    pub sla_policies: usize,
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_problem() {
        let p = Problem::new("DB down", "Production database unreachable", Domain::Infrastructure, Severity::Critical, "ops-team");
        assert_eq!(p.status, ProblemStatus::Open);
        assert_eq!(p.severity, Severity::Critical);
        assert!(!p.hash.is_empty());
    }

    #[test]
    fn test_solver_submit() {
        let mut solver = EnterpriseSolver::new();
        let report = solver.solve("Latency spike", "API response times > 5s", Domain::Performance, Severity::High);
        assert!(!report.diagnosis.root_cause.is_empty());
        assert!(!report.resolution.solutions.is_empty());
        assert_eq!(solver.problems.len(), 1);
    }

    #[test]
    fn test_solver_status() {
        let mut solver = EnterpriseSolver::new();
        solver.solve("Issue A", "Desc A", Domain::Security, Severity::Critical);
        solver.solve("Issue B", "Desc B", Domain::Financial, Severity::Low);
        let status = solver.status();
        assert_eq!(status.total_problems, 2);
        assert_eq!(status.critical_count, 1);
    }

    #[test]
    fn test_severity_weight() {
        assert_eq!(Severity::Critical.weight(), 1.0);
        assert_eq!(Severity::Low.weight(), 0.25);
    }

    #[test]
    fn test_problem_with_tags() {
        let p = Problem::new("Test", "Test desc", Domain::Operations, Severity::Info, "user")
            .with_tags(vec!["urgent".into(), "backend".into()])
            .with_affected_systems(vec!["api-gateway".into()]);
        assert_eq!(p.tags.len(), 2);
        assert_eq!(p.affected_systems.len(), 1);
    }
}
