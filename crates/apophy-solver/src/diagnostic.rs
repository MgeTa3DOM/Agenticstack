//! Diagnostic Engine — Root cause analysis and symptom classification
//!
//! Uses pattern-based reasoning to identify root causes from problem descriptions.
//! Applies domain-specific heuristics for accurate classification.

use crate::{Domain, Problem, Severity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnosis {
    pub id: Uuid,
    pub problem_id: Uuid,
    pub root_cause: String,
    pub root_cause_category: RootCauseCategory,
    pub confidence: f64,
    pub symptoms: Vec<Symptom>,
    pub contributing_factors: Vec<String>,
    pub affected_components: Vec<String>,
    pub diagnostic_path: Vec<DiagnosticStep>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RootCauseCategory {
    Configuration,
    ResourceExhaustion,
    CodeDefect,
    ExternalDependency,
    HumanError,
    DesignFlaw,
    SecurityBreach,
    DataCorruption,
    NetworkFailure,
    CapacityLimit,
    PolicyViolation,
    ProcessGap,
    Unknown,
}

impl std::fmt::Display for RootCauseCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symptom {
    pub description: String,
    pub severity: Severity,
    pub first_observed: DateTime<Utc>,
    pub recurring: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticStep {
    pub step: usize,
    pub action: String,
    pub finding: String,
    pub confidence_delta: f64,
}

pub struct DiagnosticEngine {
    pub patterns: Vec<DiagnosticPattern>,
}

#[derive(Debug, Clone)]
pub struct DiagnosticPattern {
    pub domain: Domain,
    pub keywords: Vec<String>,
    pub root_cause_category: RootCauseCategory,
    pub root_cause_template: String,
    pub contributing_factors: Vec<String>,
}

impl DiagnosticEngine {
    pub fn new() -> Self {
        Self {
            patterns: Self::build_default_patterns(),
        }
    }

    pub fn diagnose(&self, problem: &Problem) -> Diagnosis {
        let desc_lower = problem.description.to_lowercase();
        let title_lower = problem.title.to_lowercase();
        let combined = format!("{} {}", title_lower, desc_lower);

        // Score each pattern
        let mut best_match: Option<(&DiagnosticPattern, usize)> = None;
        for pattern in &self.patterns {
            let hits = pattern.keywords.iter()
                .filter(|kw| combined.contains(kw.as_str()))
                .count();
            let domain_bonus = if pattern.domain == problem.domain { 3 } else { 0 };
            let score = hits + domain_bonus;

            if score > 0 {
                if let Some((_, best_score)) = &best_match {
                    if score > *best_score {
                        best_match = Some((pattern, score));
                    }
                } else {
                    best_match = Some((pattern, score));
                }
            }
        }

        let (root_cause, category, factors, confidence) = match best_match {
            Some((pattern, score)) => {
                let confidence = (score as f64 / (pattern.keywords.len() as f64 + 3.0)).min(0.95);
                (
                    pattern.root_cause_template.clone(),
                    pattern.root_cause_category.clone(),
                    pattern.contributing_factors.clone(),
                    confidence,
                )
            }
            None => {
                let (cause, cat) = Self::infer_from_domain(&problem.domain);
                (cause, cat, vec!["Insufficient data for precise diagnosis".into()], 0.3)
            }
        };

        // Build diagnostic path
        let steps = vec![
            DiagnosticStep {
                step: 1,
                action: "Symptom collection".into(),
                finding: format!("Problem reported in domain: {}", problem.domain),
                confidence_delta: 0.1,
            },
            DiagnosticStep {
                step: 2,
                action: "Pattern matching".into(),
                finding: format!("Root cause category identified: {}", category),
                confidence_delta: confidence - 0.1,
            },
            DiagnosticStep {
                step: 3,
                action: "Impact correlation".into(),
                finding: format!("Severity {} with {} affected systems", problem.severity, problem.affected_systems.len()),
                confidence_delta: 0.05,
            },
        ];

        // Extract symptoms from description
        let symptoms = vec![Symptom {
            description: problem.description.clone(),
            severity: problem.severity.clone(),
            first_observed: problem.created_at,
            recurring: false,
        }];

        Diagnosis {
            id: Uuid::new_v4(),
            problem_id: problem.id,
            root_cause,
            root_cause_category: category,
            confidence,
            symptoms,
            contributing_factors: factors,
            affected_components: problem.affected_systems.clone(),
            diagnostic_path: steps,
            created_at: Utc::now(),
        }
    }

    fn infer_from_domain(domain: &Domain) -> (String, RootCauseCategory) {
        match domain {
            Domain::Infrastructure => ("Infrastructure component degradation".into(), RootCauseCategory::ResourceExhaustion),
            Domain::Security => ("Potential security posture weakness".into(), RootCauseCategory::SecurityBreach),
            Domain::Performance => ("Performance bottleneck detected".into(), RootCauseCategory::CapacityLimit),
            Domain::DataIntegrity => ("Data consistency issue".into(), RootCauseCategory::DataCorruption),
            Domain::Compliance => ("Compliance gap identified".into(), RootCauseCategory::PolicyViolation),
            Domain::Financial => ("Financial process anomaly".into(), RootCauseCategory::ProcessGap),
            Domain::Operations => ("Operational process inefficiency".into(), RootCauseCategory::ProcessGap),
            Domain::HumanResources => ("HR process or policy gap".into(), RootCauseCategory::PolicyViolation),
            Domain::CustomerExperience => ("Customer journey friction point".into(), RootCauseCategory::DesignFlaw),
            Domain::SupplyChain => ("Supply chain disruption".into(), RootCauseCategory::ExternalDependency),
            Domain::Legal => ("Legal/regulatory exposure".into(), RootCauseCategory::PolicyViolation),
            Domain::Strategy => ("Strategic misalignment".into(), RootCauseCategory::DesignFlaw),
        }
    }

    fn build_default_patterns() -> Vec<DiagnosticPattern> {
        vec![
            DiagnosticPattern {
                domain: Domain::Infrastructure,
                keywords: vec!["down".into(), "unreachable".into(), "timeout".into(), "crash".into(), "oom".into(), "disk full".into(), "connection refused".into()],
                root_cause_category: RootCauseCategory::ResourceExhaustion,
                root_cause_template: "Resource exhaustion causing service degradation".into(),
                contributing_factors: vec!["Insufficient capacity planning".into(), "Missing autoscaling".into(), "No resource limits configured".into()],
            },
            DiagnosticPattern {
                domain: Domain::Infrastructure,
                keywords: vec!["config".into(), "misconfigured".into(), "wrong value".into(), "typo".into(), "env var".into(), "missing setting".into()],
                root_cause_category: RootCauseCategory::Configuration,
                root_cause_template: "Configuration error in deployment or service settings".into(),
                contributing_factors: vec!["No config validation".into(), "Missing CI checks".into(), "Manual deployment".into()],
            },
            DiagnosticPattern {
                domain: Domain::Security,
                keywords: vec!["breach".into(), "unauthorized".into(), "exploit".into(), "vulnerability".into(), "injection".into(), "xss".into(), "leaked".into()],
                root_cause_category: RootCauseCategory::SecurityBreach,
                root_cause_template: "Security vulnerability exploited or detected".into(),
                contributing_factors: vec!["Outdated dependencies".into(), "Missing input validation".into(), "Insufficient access controls".into()],
            },
            DiagnosticPattern {
                domain: Domain::Performance,
                keywords: vec!["slow".into(), "latency".into(), "lag".into(), "bottleneck".into(), "spike".into(), "response time".into(), "throughput".into()],
                root_cause_category: RootCauseCategory::CapacityLimit,
                root_cause_template: "Performance degradation from capacity or optimization issues".into(),
                contributing_factors: vec!["N+1 queries".into(), "Missing indices".into(), "No caching layer".into(), "Unoptimized algorithms".into()],
            },
            DiagnosticPattern {
                domain: Domain::DataIntegrity,
                keywords: vec!["corrupt".into(), "inconsistent".into(), "duplicate".into(), "missing data".into(), "stale".into(), "out of sync".into()],
                root_cause_category: RootCauseCategory::DataCorruption,
                root_cause_template: "Data integrity violation — inconsistency or corruption detected".into(),
                contributing_factors: vec!["No transaction isolation".into(), "Race conditions".into(), "Missing constraints".into()],
            },
            DiagnosticPattern {
                domain: Domain::Compliance,
                keywords: vec!["gdpr".into(), "hipaa".into(), "sox".into(), "pci".into(), "audit".into(), "regulation".into(), "non-compliant".into()],
                root_cause_category: RootCauseCategory::PolicyViolation,
                root_cause_template: "Regulatory compliance gap requiring remediation".into(),
                contributing_factors: vec!["Missing compliance controls".into(), "Outdated policies".into(), "No automated checks".into()],
            },
            DiagnosticPattern {
                domain: Domain::Financial,
                keywords: vec!["budget".into(), "cost".into(), "overrun".into(), "billing".into(), "invoice".into(), "revenue".into(), "margin".into()],
                root_cause_category: RootCauseCategory::ProcessGap,
                root_cause_template: "Financial process gap causing misalignment or loss".into(),
                contributing_factors: vec!["No cost monitoring".into(), "Missing approval workflows".into(), "Unclear ownership".into()],
            },
            DiagnosticPattern {
                domain: Domain::Operations,
                keywords: vec!["manual".into(), "bottleneck".into(), "blocked".into(), "process".into(), "handoff".into(), "silo".into(), "backlog".into()],
                root_cause_category: RootCauseCategory::ProcessGap,
                root_cause_template: "Operational inefficiency from manual processes or organizational silos".into(),
                contributing_factors: vec!["Lack of automation".into(), "Unclear RACI".into(), "Missing runbooks".into()],
            },
            DiagnosticPattern {
                domain: Domain::CustomerExperience,
                keywords: vec!["churn".into(), "complaint".into(), "nps".into(), "abandon".into(), "friction".into(), "ux".into(), "onboarding".into()],
                root_cause_category: RootCauseCategory::DesignFlaw,
                root_cause_template: "Customer experience degradation from design or journey issues".into(),
                contributing_factors: vec!["No user research".into(), "Missing feedback loops".into(), "Broken user flows".into()],
            },
            DiagnosticPattern {
                domain: Domain::SupplyChain,
                keywords: vec!["supplier".into(), "delay".into(), "shortage".into(), "vendor".into(), "logistics".into(), "inventory".into(), "lead time".into()],
                root_cause_category: RootCauseCategory::ExternalDependency,
                root_cause_template: "Supply chain disruption from external dependency failure".into(),
                contributing_factors: vec!["Single-source dependency".into(), "No safety stock".into(), "Missing contingency plans".into()],
            },
            DiagnosticPattern {
                domain: Domain::Infrastructure,
                keywords: vec!["network".into(), "dns".into(), "ssl".into(), "certificate".into(), "routing".into(), "firewall".into(), "packet loss".into()],
                root_cause_category: RootCauseCategory::NetworkFailure,
                root_cause_template: "Network infrastructure failure causing connectivity issues".into(),
                contributing_factors: vec!["No redundant paths".into(), "Certificate expiry".into(), "Firewall misconfiguration".into()],
            },
            DiagnosticPattern {
                domain: Domain::Operations,
                keywords: vec!["deploy".into(), "rollback".into(), "release".into(), "pipeline".into(), "ci".into(), "cd".into(), "build".into()],
                root_cause_category: RootCauseCategory::CodeDefect,
                root_cause_template: "Deployment pipeline issue causing release failures".into(),
                contributing_factors: vec!["No canary deploys".into(), "Missing rollback strategy".into(), "Insufficient testing".into()],
            },
        ]
    }
}

impl Default for DiagnosticEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnose_infrastructure() {
        let engine = DiagnosticEngine::new();
        let p = Problem::new("DB timeout", "Database connection timeout after 30s", Domain::Infrastructure, Severity::Critical, "sre");
        let d = engine.diagnose(&p);
        assert_eq!(d.root_cause_category, RootCauseCategory::ResourceExhaustion);
        assert!(d.confidence > 0.1);
    }

    #[test]
    fn test_diagnose_security() {
        let engine = DiagnosticEngine::new();
        let p = Problem::new("SQL injection", "SQL injection vulnerability in login form", Domain::Security, Severity::Critical, "security-team");
        let d = engine.diagnose(&p);
        assert_eq!(d.root_cause_category, RootCauseCategory::SecurityBreach);
    }

    #[test]
    fn test_diagnose_unknown() {
        let engine = DiagnosticEngine::new();
        let p = Problem::new("Strange", "Something weird happened", Domain::Strategy, Severity::Info, "user");
        let d = engine.diagnose(&p);
        assert!(d.confidence <= 0.4);
    }
}
