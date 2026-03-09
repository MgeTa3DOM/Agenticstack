//! Resolution Engine — Solution generation with prioritization
//!
//! Generates actionable solutions based on diagnosis and risk assessment.
//! Each solution includes effort estimation, prerequisites, and validation criteria.

use crate::{Diagnosis, DiagnosticCategory, Domain, Problem, RiskAssessment};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub id: Uuid,
    pub problem_id: Uuid,
    pub solutions: Vec<Solution>,
    pub recommended_idx: usize,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub solution_type: SolutionType,
    pub steps: Vec<ActionStep>,
    pub effort: Effort,
    pub effectiveness_score: f64,
    pub prerequisites: Vec<String>,
    pub validation_criteria: Vec<String>,
    pub rollback_plan: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SolutionType {
    QuickFix,
    ShortTerm,
    LongTerm,
    Strategic,
    Workaround,
}

impl std::fmt::Display for SolutionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolutionType::QuickFix => write!(f, "Quick Fix"),
            SolutionType::ShortTerm => write!(f, "Short-Term"),
            SolutionType::LongTerm => write!(f, "Long-Term"),
            SolutionType::Strategic => write!(f, "Strategic"),
            SolutionType::Workaround => write!(f, "Workaround"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStep {
    pub order: usize,
    pub action: String,
    pub owner: String,
    pub estimated_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effort {
    pub hours: f64,
    pub team_size: usize,
    pub complexity: Complexity,
    pub cost_estimate_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Complexity {
    Trivial,
    Simple,
    Moderate,
    Complex,
    Extreme,
}

impl std::fmt::Display for Complexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct ResolutionEngine {
    pub templates: Vec<ResolutionTemplate>,
}

#[derive(Debug, Clone)]
pub struct ResolutionTemplate {
    pub cause_category: DiagnosticCategory,
    pub solutions: Vec<SolutionTemplate>,
}

#[derive(Debug, Clone)]
pub struct SolutionTemplate {
    pub title: String,
    pub description: String,
    pub solution_type: SolutionType,
    pub steps: Vec<String>,
    pub hours: f64,
    pub team_size: usize,
    pub effectiveness: f64,
}

impl ResolutionEngine {
    pub fn new() -> Self {
        Self {
            templates: Self::build_templates(),
        }
    }

    pub fn resolve(
        &self,
        problem: &Problem,
        diagnosis: &Diagnosis,
        risk: &RiskAssessment,
    ) -> Resolution {
        let solutions = self.generate_solutions(problem, diagnosis, risk);
        let recommended_idx = self.recommend(&solutions, risk);

        Resolution {
            id: Uuid::new_v4(),
            problem_id: problem.id,
            solutions,
            recommended_idx,
            created_at: Utc::now(),
        }
    }

    fn generate_solutions(
        &self,
        problem: &Problem,
        diagnosis: &Diagnosis,
        _risk: &RiskAssessment,
    ) -> Vec<Solution> {
        let matching_templates: Vec<&ResolutionTemplate> = self.templates.iter()
            .filter(|t| t.cause_category == diagnosis.category)
            .collect();

        let mut solutions = Vec::new();

        for template in &matching_templates {
            for st in &template.solutions {
                let steps: Vec<ActionStep> = st.steps.iter().enumerate().map(|(i, s)| {
                    ActionStep {
                        order: i + 1,
                        action: s.clone(),
                        owner: Self::infer_owner(&problem.domain),
                        estimated_hours: st.hours / st.steps.len() as f64,
                    }
                }).collect();

                let complexity = match st.hours {
                    h if h <= 2.0 => Complexity::Trivial,
                    h if h <= 8.0 => Complexity::Simple,
                    h if h <= 40.0 => Complexity::Moderate,
                    h if h <= 160.0 => Complexity::Complex,
                    _ => Complexity::Extreme,
                };

                let cost = st.hours * st.team_size as f64 * 150.0; // $150/hr blended rate

                solutions.push(Solution {
                    id: Uuid::new_v4(),
                    title: st.title.clone(),
                    description: st.description.clone(),
                    solution_type: st.solution_type.clone(),
                    steps,
                    effort: Effort {
                        hours: st.hours,
                        team_size: st.team_size,
                        complexity,
                        cost_estimate_usd: cost,
                    },
                    effectiveness_score: st.effectiveness,
                    prerequisites: vec![],
                    validation_criteria: vec![
                        format!("Problem '{}' no longer reproducible", problem.title),
                        "All affected systems operational".into(),
                        "Monitoring confirms stability for 24h".into(),
                    ],
                    rollback_plan: format!("Revert changes and restore previous state for: {}", problem.title),
                });
            }
        }

        // Always add a domain-specific fallback
        if solutions.is_empty() {
            solutions.push(Self::fallback_solution(problem));
        }

        solutions
    }

    fn recommend(&self, solutions: &[Solution], risk: &RiskAssessment) -> usize {
        if solutions.is_empty() {
            return 0;
        }

        // High risk → prefer quick fixes; low risk → prefer long-term solutions
        let prefer_quick = risk.risk_score > 0.7;

        solutions.iter().enumerate()
            .max_by(|(_, a), (_, b)| {
                let a_score = a.effectiveness_score
                    + if prefer_quick && a.solution_type == SolutionType::QuickFix { 0.3 } else { 0.0 }
                    + if !prefer_quick && a.solution_type == SolutionType::LongTerm { 0.2 } else { 0.0 };
                let b_score = b.effectiveness_score
                    + if prefer_quick && b.solution_type == SolutionType::QuickFix { 0.3 } else { 0.0 }
                    + if !prefer_quick && b.solution_type == SolutionType::LongTerm { 0.2 } else { 0.0 };
                a_score.partial_cmp(&b_score).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn infer_owner(domain: &Domain) -> String {
        match domain {
            Domain::Infrastructure => "SRE Team".into(),
            Domain::Security => "Security Team".into(),
            Domain::Performance => "Platform Team".into(),
            Domain::DataIntegrity => "Data Engineering".into(),
            Domain::Compliance => "Compliance Officer".into(),
            Domain::Financial => "Finance Team".into(),
            Domain::Operations => "Ops Team".into(),
            Domain::HumanResources => "HR Department".into(),
            Domain::CustomerExperience => "Product Team".into(),
            Domain::SupplyChain => "Supply Chain Manager".into(),
            Domain::Legal => "Legal Department".into(),
            Domain::Strategy => "Executive Team".into(),
        }
    }

    fn fallback_solution(problem: &Problem) -> Solution {
        Solution {
            id: Uuid::new_v4(),
            title: format!("Investigate and remediate: {}", problem.title),
            description: "Manual investigation required — no automated resolution template matched".into(),
            solution_type: SolutionType::ShortTerm,
            steps: vec![
                ActionStep { order: 1, action: "Gather detailed logs and metrics".into(), owner: "On-call".into(), estimated_hours: 2.0 },
                ActionStep { order: 2, action: "Reproduce the issue in staging".into(), owner: "Engineering".into(), estimated_hours: 4.0 },
                ActionStep { order: 3, action: "Implement fix and validate".into(), owner: "Engineering".into(), estimated_hours: 8.0 },
                ActionStep { order: 4, action: "Deploy and monitor".into(), owner: "SRE".into(), estimated_hours: 2.0 },
            ],
            effort: Effort { hours: 16.0, team_size: 2, complexity: Complexity::Moderate, cost_estimate_usd: 4800.0 },
            effectiveness_score: 0.6,
            prerequisites: vec!["Access to production logs".into()],
            validation_criteria: vec!["Issue no longer reproducible".into()],
            rollback_plan: "Revert deployment to last known good state".into(),
        }
    }

    fn build_templates() -> Vec<ResolutionTemplate> {
        vec![
            ResolutionTemplate {
                cause_category: DiagnosticCategory::ResourceExhaustion,
                solutions: vec![
                    SolutionTemplate {
                        title: "Emergency resource scaling".into(),
                        description: "Immediately scale up affected resources to restore service".into(),
                        solution_type: SolutionType::QuickFix,
                        steps: vec!["Identify exhausted resource".into(), "Scale up (vertical/horizontal)".into(), "Verify service restored".into()],
                        hours: 2.0, team_size: 1, effectiveness: 0.85,
                    },
                    SolutionTemplate {
                        title: "Implement autoscaling with resource limits".into(),
                        description: "Configure autoscaling policies and resource quotas to prevent future exhaustion".into(),
                        solution_type: SolutionType::LongTerm,
                        steps: vec!["Audit current resource usage".into(), "Define scaling policies".into(), "Set resource limits and quotas".into(), "Configure alerts at 70% threshold".into(), "Test scaling behavior under load".into()],
                        hours: 24.0, team_size: 2, effectiveness: 0.95,
                    },
                ],
            },
            ResolutionTemplate {
                cause_category: DiagnosticCategory::ConfigurationDrift,
                solutions: vec![
                    SolutionTemplate {
                        title: "Fix misconfiguration".into(),
                        description: "Correct the configuration error and validate".into(),
                        solution_type: SolutionType::QuickFix,
                        steps: vec!["Identify incorrect setting".into(), "Apply correct value".into(), "Restart affected service".into(), "Verify fix".into()],
                        hours: 1.0, team_size: 1, effectiveness: 0.9,
                    },
                    SolutionTemplate {
                        title: "Config validation pipeline".into(),
                        description: "Add automated config validation to CI/CD pipeline".into(),
                        solution_type: SolutionType::LongTerm,
                        steps: vec!["Define config schema (JSON Schema / TOML)".into(), "Add validation step in CI".into(), "Add pre-deploy config checks".into(), "Document all config parameters".into()],
                        hours: 16.0, team_size: 1, effectiveness: 0.92,
                    },
                ],
            },
            ResolutionTemplate {
                cause_category: DiagnosticCategory::SecurityBreach,
                solutions: vec![
                    SolutionTemplate {
                        title: "Incident containment".into(),
                        description: "Isolate affected systems, rotate credentials, patch vulnerability".into(),
                        solution_type: SolutionType::QuickFix,
                        steps: vec!["Isolate affected systems".into(), "Rotate all exposed credentials".into(), "Patch vulnerability".into(), "Audit access logs".into()],
                        hours: 4.0, team_size: 2, effectiveness: 0.8,
                    },
                    SolutionTemplate {
                        title: "Security hardening program".into(),
                        description: "Comprehensive security audit, dependency updates, and defense-in-depth".into(),
                        solution_type: SolutionType::Strategic,
                        steps: vec!["Full dependency audit".into(), "SAST/DAST integration".into(), "WAF deployment".into(), "Penetration testing".into(), "Security training for team".into(), "Incident response runbook".into()],
                        hours: 80.0, team_size: 3, effectiveness: 0.97,
                    },
                ],
            },
            ResolutionTemplate {
                cause_category: DiagnosticCategory::CapacityLimit,
                solutions: vec![
                    SolutionTemplate {
                        title: "Caching and query optimization".into(),
                        description: "Add caching layer and optimize hot-path queries".into(),
                        solution_type: SolutionType::ShortTerm,
                        steps: vec!["Profile slow endpoints".into(), "Add Redis/in-memory cache".into(), "Optimize database queries".into(), "Add database indices".into()],
                        hours: 16.0, team_size: 2, effectiveness: 0.88,
                    },
                    SolutionTemplate {
                        title: "Architecture redesign for scale".into(),
                        description: "Re-architect bottleneck components for horizontal scalability".into(),
                        solution_type: SolutionType::Strategic,
                        steps: vec!["Load test to identify limits".into(), "Design scalable architecture".into(), "Implement CQRS/event sourcing".into(), "Add read replicas".into(), "Deploy CDN".into()],
                        hours: 120.0, team_size: 4, effectiveness: 0.96,
                    },
                ],
            },
            ResolutionTemplate {
                cause_category: DiagnosticCategory::DataCorruption,
                solutions: vec![
                    SolutionTemplate {
                        title: "Data repair and reconciliation".into(),
                        description: "Identify corrupted records, repair from backups or recalculate".into(),
                        solution_type: SolutionType::QuickFix,
                        steps: vec!["Identify affected records".into(), "Restore from backup if available".into(), "Reconcile data".into(), "Verify integrity".into()],
                        hours: 8.0, team_size: 2, effectiveness: 0.75,
                    },
                    SolutionTemplate {
                        title: "Data integrity framework".into(),
                        description: "Implement checksums, constraints, and automated integrity checks".into(),
                        solution_type: SolutionType::LongTerm,
                        steps: vec!["Add database constraints".into(), "Implement checksums on critical data".into(), "Create reconciliation jobs".into(), "Add data quality monitoring".into()],
                        hours: 40.0, team_size: 2, effectiveness: 0.93,
                    },
                ],
            },
            ResolutionTemplate {
                cause_category: DiagnosticCategory::PolicyViolation,
                solutions: vec![
                    SolutionTemplate {
                        title: "Immediate compliance remediation".into(),
                        description: "Address policy violation with corrective action".into(),
                        solution_type: SolutionType::QuickFix,
                        steps: vec!["Document the violation".into(), "Apply corrective controls".into(), "Notify stakeholders".into(), "Update audit trail".into()],
                        hours: 4.0, team_size: 1, effectiveness: 0.7,
                    },
                    SolutionTemplate {
                        title: "Compliance automation program".into(),
                        description: "Automate compliance checks, policy enforcement, and audit reporting".into(),
                        solution_type: SolutionType::Strategic,
                        steps: vec!["Map all regulatory requirements".into(), "Implement automated policy checks".into(), "Build compliance dashboard".into(), "Schedule regular audits".into(), "Train all teams".into()],
                        hours: 60.0, team_size: 3, effectiveness: 0.94,
                    },
                ],
            },
            ResolutionTemplate {
                cause_category: DiagnosticCategory::DependencyFailure,
                solutions: vec![
                    SolutionTemplate {
                        title: "Network path restoration".into(),
                        description: "Restore connectivity via failover or configuration fix".into(),
                        solution_type: SolutionType::QuickFix,
                        steps: vec!["Identify failed link".into(), "Activate failover path".into(), "Verify connectivity".into()],
                        hours: 1.0, team_size: 1, effectiveness: 0.85,
                    },
                    SolutionTemplate {
                        title: "Network redundancy and monitoring".into(),
                        description: "Implement redundant paths, health checks, and automated failover".into(),
                        solution_type: SolutionType::LongTerm,
                        steps: vec!["Design redundant topology".into(), "Deploy health check probes".into(), "Configure automated failover".into(), "Add network monitoring".into()],
                        hours: 32.0, team_size: 2, effectiveness: 0.94,
                    },
                ],
            },
            ResolutionTemplate {
                cause_category: DiagnosticCategory::ProcessBottleneck,
                solutions: vec![
                    SolutionTemplate {
                        title: "Process patch".into(),
                        description: "Document and implement the missing process step".into(),
                        solution_type: SolutionType::ShortTerm,
                        steps: vec!["Map current process".into(), "Identify gap".into(), "Document new step".into(), "Train team".into()],
                        hours: 8.0, team_size: 1, effectiveness: 0.7,
                    },
                    SolutionTemplate {
                        title: "Process automation and optimization".into(),
                        description: "Automate manual processes and eliminate handoff friction".into(),
                        solution_type: SolutionType::Strategic,
                        steps: vec!["Value stream mapping".into(), "Identify automation candidates".into(), "Implement workflow automation".into(), "Define SLAs for each step".into(), "Continuous improvement loop".into()],
                        hours: 48.0, team_size: 2, effectiveness: 0.91,
                    },
                ],
            },
            ResolutionTemplate {
                cause_category: DiagnosticCategory::ExternalDisruption,
                solutions: vec![
                    SolutionTemplate {
                        title: "Vendor failover".into(),
                        description: "Switch to backup vendor or alternative source".into(),
                        solution_type: SolutionType::QuickFix,
                        steps: vec!["Activate backup vendor".into(), "Redirect supply flow".into(), "Validate quality".into()],
                        hours: 4.0, team_size: 1, effectiveness: 0.75,
                    },
                    SolutionTemplate {
                        title: "Multi-vendor strategy".into(),
                        description: "Establish multiple suppliers and reduce single-point dependency".into(),
                        solution_type: SolutionType::Strategic,
                        steps: vec!["Audit all single-source deps".into(), "Qualify alternate vendors".into(), "Negotiate contracts".into(), "Build safety stock".into(), "Create contingency playbook".into()],
                        hours: 80.0, team_size: 2, effectiveness: 0.92,
                    },
                ],
            },
            ResolutionTemplate {
                cause_category: DiagnosticCategory::IntegrationFailure,
                solutions: vec![
                    SolutionTemplate {
                        title: "Hotfix deployment".into(),
                        description: "Deploy targeted fix for the code defect".into(),
                        solution_type: SolutionType::QuickFix,
                        steps: vec!["Isolate defective code".into(), "Write fix + tests".into(), "Code review".into(), "Deploy via hotfix pipeline".into()],
                        hours: 4.0, team_size: 1, effectiveness: 0.85,
                    },
                    SolutionTemplate {
                        title: "Quality engineering program".into(),
                        description: "Strengthen testing, code review, and deployment practices".into(),
                        solution_type: SolutionType::LongTerm,
                        steps: vec!["Increase test coverage to 80%+".into(), "Mandatory code review".into(), "Add integration tests".into(), "Implement canary deployments".into(), "Post-mortem process".into()],
                        hours: 60.0, team_size: 3, effectiveness: 0.93,
                    },
                ],
            },
            ResolutionTemplate {
                cause_category: DiagnosticCategory::HumanError,
                solutions: vec![
                    SolutionTemplate {
                        title: "Immediate correction".into(),
                        description: "Reverse the erroneous action and validate state".into(),
                        solution_type: SolutionType::QuickFix,
                        steps: vec!["Identify the error".into(), "Reverse action".into(), "Verify system state".into()],
                        hours: 2.0, team_size: 1, effectiveness: 0.8,
                    },
                    SolutionTemplate {
                        title: "Error-proofing (poka-yoke)".into(),
                        description: "Add guardrails, confirmation prompts, and automation to prevent human errors".into(),
                        solution_type: SolutionType::LongTerm,
                        steps: vec!["Identify error-prone operations".into(), "Add confirmation gates".into(), "Automate risky procedures".into(), "Add undo capability".into(), "Training and documentation".into()],
                        hours: 32.0, team_size: 2, effectiveness: 0.9,
                    },
                ],
            },
            ResolutionTemplate {
                cause_category: DiagnosticCategory::DesignFlaw,
                solutions: vec![
                    SolutionTemplate {
                        title: "Targeted UX/design fix".into(),
                        description: "Fix the specific design issue causing friction".into(),
                        solution_type: SolutionType::ShortTerm,
                        steps: vec!["User research on pain point".into(), "Design solution".into(), "A/B test".into(), "Deploy winner".into()],
                        hours: 16.0, team_size: 2, effectiveness: 0.8,
                    },
                    SolutionTemplate {
                        title: "Design system overhaul".into(),
                        description: "Redesign the affected user journey with data-driven approach".into(),
                        solution_type: SolutionType::Strategic,
                        steps: vec!["Full journey mapping".into(), "User interviews".into(), "Prototype new design".into(), "Usability testing".into(), "Phased rollout".into()],
                        hours: 80.0, team_size: 3, effectiveness: 0.94,
                    },
                ],
            },
            ResolutionTemplate {
                cause_category: DiagnosticCategory::IntegrationFailure,
                solutions: vec![
                    SolutionTemplate {
                        title: "Structured investigation".into(),
                        description: "Systematic investigation with 5-Whys and fishbone analysis".into(),
                        solution_type: SolutionType::ShortTerm,
                        steps: vec!["Gather all available data".into(), "5-Whys analysis".into(), "Fishbone diagram".into(), "Hypothesis testing".into(), "Root cause confirmation".into()],
                        hours: 16.0, team_size: 2, effectiveness: 0.65,
                    },
                ],
            },
        ]
    }
}

impl Default for ResolutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Severity, risk::RiskMatrix};

    #[test]
    fn test_resolve_generates_solutions() {
        let engine = ResolutionEngine::new();
        let p = Problem::new("Test", "Resource exhaustion on DB", Domain::Infrastructure, Severity::High, "test");
        let d = crate::DiagnosticEngine::new().diagnose(&p);
        let r = RiskMatrix::new().assess(&p, &d);
        let resolution = engine.resolve(&p, &d, &r);
        assert!(!resolution.solutions.is_empty());
    }

    #[test]
    fn test_fallback_solution() {
        let engine = ResolutionEngine::new();
        let p = Problem::new("Unknown", "Completely unknown issue", Domain::Strategy, crate::Severity::Low, "test");
        let d = crate::DiagnosticEngine::new().diagnose(&p);
        let r = RiskMatrix::new().assess(&p, &d);
        let resolution = engine.resolve(&p, &d, &r);
        assert!(!resolution.solutions.is_empty());
    }
}
