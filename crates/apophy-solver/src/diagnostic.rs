//! # Diagnostic Engine
//!
//! Automated root-cause analysis for enterprise problems.
//! Uses pattern matching, symptom correlation, and domain-specific
//! heuristics to identify the underlying cause of issues.

use crate::{Domain, Problem, Severity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnosis {
    pub id: Uuid,
    pub problem_id: Uuid,
    pub root_cause: String,
    pub category: DiagnosticCategory,
    pub confidence: f64,
    pub symptoms: Vec<Symptom>,
    pub contributing_factors: Vec<String>,
    pub affected_layers: Vec<String>,
    pub diagnostic_path: Vec<DiagnosticStep>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiagnosticCategory {
    ResourceExhaustion,
    ConfigurationDrift,
    DependencyFailure,
    SecurityBreach,
    DataCorruption,
    ProcessBottleneck,
    HumanError,
    ExternalDisruption,
    DesignFlaw,
    CapacityLimit,
    IntegrationFailure,
    PolicyViolation,
}

impl std::fmt::Display for DiagnosticCategory {
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
    pub eliminated: Vec<String>,
}

pub struct DiagnosticEngine {
    pub patterns: Vec<DiagnosticPattern>,
}

#[derive(Debug, Clone)]
pub struct DiagnosticPattern {
    pub domain: Domain,
    pub keywords: Vec<String>,
    pub category: DiagnosticCategory,
    pub root_cause_template: String,
    pub contributing_factors: Vec<String>,
    pub affected_layers: Vec<String>,
}

impl DiagnosticEngine {
    pub fn new() -> Self {
        Self {
            patterns: Self::builtin_patterns(),
        }
    }

    pub fn diagnose(&self, problem: &Problem) -> Diagnosis {
        let description_lower = problem.description.to_lowercase();
        let title_lower = problem.title.to_lowercase();
        let combined = format!("{} {}", title_lower, description_lower);

        // Find best matching pattern
        let (category, root_cause, factors, layers, confidence) = self
            .patterns
            .iter()
            .filter(|p| p.domain == problem.domain || self.keyword_match(&combined, &p.keywords))
            .max_by(|a, b| {
                let score_a = self.pattern_score(&combined, a);
                let score_b = self.pattern_score(&combined, b);
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|p| {
                let score = self.pattern_score(&combined, p);
                (
                    p.category.clone(),
                    p.root_cause_template.replace("{title}", &problem.title),
                    p.contributing_factors.clone(),
                    p.affected_layers.clone(),
                    (score * 0.85).min(0.95),
                )
            })
            .unwrap_or_else(|| {
                (
                    self.infer_category(&problem.domain),
                    format!("Unresolved issue in {} domain: {}", problem.domain, problem.title),
                    vec!["Insufficient diagnostic data".into()],
                    vec![problem.domain.to_string()],
                    0.3,
                )
            });

        // Build diagnostic path
        let diagnostic_path = vec![
            DiagnosticStep {
                step: 1,
                action: "Symptom collection".into(),
                finding: format!("Problem in {} domain, severity {}", problem.domain, problem.severity),
                eliminated: vec![],
            },
            DiagnosticStep {
                step: 2,
                action: "Pattern matching".into(),
                finding: format!("Matched category: {}", category),
                eliminated: self.eliminated_categories(&category),
            },
            DiagnosticStep {
                step: 3,
                action: "Root cause isolation".into(),
                finding: root_cause.clone(),
                eliminated: vec![],
            },
        ];

        // Extract symptoms from description
        let symptoms = self.extract_symptoms(problem);

        Diagnosis {
            id: Uuid::new_v4(),
            problem_id: problem.id,
            root_cause,
            category,
            confidence,
            symptoms,
            contributing_factors: factors,
            affected_layers: layers,
            diagnostic_path,
            created_at: Utc::now(),
        }
    }

    fn keyword_match(&self, text: &str, keywords: &[String]) -> bool {
        keywords.iter().any(|kw| text.contains(&kw.to_lowercase()))
    }

    fn pattern_score(&self, text: &str, pattern: &DiagnosticPattern) -> f64 {
        let matches = pattern
            .keywords
            .iter()
            .filter(|kw| text.contains(&kw.to_lowercase()))
            .count();
        matches as f64 / pattern.keywords.len().max(1) as f64
    }

    fn infer_category(&self, domain: &Domain) -> DiagnosticCategory {
        match domain {
            Domain::Infrastructure => DiagnosticCategory::ResourceExhaustion,
            Domain::Security => DiagnosticCategory::SecurityBreach,
            Domain::Performance => DiagnosticCategory::CapacityLimit,
            Domain::DataIntegrity => DiagnosticCategory::DataCorruption,
            Domain::Compliance => DiagnosticCategory::PolicyViolation,
            Domain::Financial => DiagnosticCategory::ProcessBottleneck,
            Domain::Operations => DiagnosticCategory::ProcessBottleneck,
            Domain::HumanResources => DiagnosticCategory::HumanError,
            Domain::CustomerExperience => DiagnosticCategory::DesignFlaw,
            Domain::SupplyChain => DiagnosticCategory::ExternalDisruption,
            Domain::Legal => DiagnosticCategory::PolicyViolation,
            Domain::Strategy => DiagnosticCategory::DesignFlaw,
        }
    }

    fn eliminated_categories(&self, matched: &DiagnosticCategory) -> Vec<String> {
        let all = [
            "ResourceExhaustion", "ConfigurationDrift", "DependencyFailure",
            "SecurityBreach", "DataCorruption", "ProcessBottleneck",
        ];
        let matched_str = format!("{:?}", matched);
        all.iter()
            .filter(|c| **c != matched_str)
            .take(3)
            .map(|s| s.to_string())
            .collect()
    }

    fn extract_symptoms(&self, problem: &Problem) -> Vec<Symptom> {
        let mut symptoms = vec![Symptom {
            description: format!("Primary: {}", problem.title),
            severity: problem.severity.clone(),
            first_observed: problem.created_at,
            recurring: false,
        }];

        // Heuristic symptom extraction from description
        let desc = &problem.description;
        if desc.contains("slow") || desc.contains("latency") || desc.contains("timeout") {
            symptoms.push(Symptom {
                description: "Performance degradation detected".into(),
                severity: Severity::Medium,
                first_observed: problem.created_at,
                recurring: true,
            });
        }
        if desc.contains("error") || desc.contains("fail") || desc.contains("crash") {
            symptoms.push(Symptom {
                description: "Error/failure condition observed".into(),
                severity: Severity::High,
                first_observed: problem.created_at,
                recurring: false,
            });
        }
        if desc.contains("data") || desc.contains("corrupt") || desc.contains("inconsisten") {
            symptoms.push(Symptom {
                description: "Data integrity concern".into(),
                severity: Severity::High,
                first_observed: problem.created_at,
                recurring: false,
            });
        }
        if desc.contains("security") || desc.contains("breach") || desc.contains("unauthorized") {
            symptoms.push(Symptom {
                description: "Security incident indicator".into(),
                severity: Severity::Critical,
                first_observed: problem.created_at,
                recurring: false,
            });
        }

        symptoms
    }

    fn builtin_patterns() -> Vec<DiagnosticPattern> {
        vec![
            // Infrastructure
            DiagnosticPattern {
                domain: Domain::Infrastructure,
                keywords: vec!["down".into(), "unreachable".into(), "outage".into(), "disk".into(), "memory".into(), "cpu".into(), "oom".into()],
                category: DiagnosticCategory::ResourceExhaustion,
                root_cause_template: "Resource exhaustion causing infrastructure failure: {title}".into(),
                contributing_factors: vec!["Insufficient capacity planning".into(), "Missing auto-scaling".into(), "No alerting threshold".into()],
                affected_layers: vec!["Infrastructure".into(), "Compute".into(), "Storage".into()],
            },
            DiagnosticPattern {
                domain: Domain::Infrastructure,
                keywords: vec!["config".into(), "misconfigur".into(), "drift".into(), "deploy".into(), "rollback".into()],
                category: DiagnosticCategory::ConfigurationDrift,
                root_cause_template: "Configuration drift in deployment: {title}".into(),
                contributing_factors: vec!["Manual configuration changes".into(), "Missing IaC".into(), "No config validation".into()],
                affected_layers: vec!["Configuration".into(), "Deployment".into()],
            },
            // Security
            DiagnosticPattern {
                domain: Domain::Security,
                keywords: vec!["breach".into(), "unauthorized".into(), "exploit".into(), "vulnerability".into(), "attack".into(), "leak".into()],
                category: DiagnosticCategory::SecurityBreach,
                root_cause_template: "Security compromise detected: {title}".into(),
                contributing_factors: vec!["Unpatched vulnerabilities".into(), "Weak access controls".into(), "Missing monitoring".into()],
                affected_layers: vec!["Security".into(), "Network".into(), "Identity".into()],
            },
            // Performance
            DiagnosticPattern {
                domain: Domain::Performance,
                keywords: vec!["slow".into(), "latency".into(), "timeout".into(), "bottleneck".into(), "throughput".into(), "response time".into()],
                category: DiagnosticCategory::CapacityLimit,
                root_cause_template: "Capacity/performance bottleneck: {title}".into(),
                contributing_factors: vec!["Unoptimized queries".into(), "Missing caching".into(), "Insufficient resources".into()],
                affected_layers: vec!["Application".into(), "Database".into(), "Network".into()],
            },
            // Data
            DiagnosticPattern {
                domain: Domain::DataIntegrity,
                keywords: vec!["corrupt".into(), "inconsisten".into(), "missing data".into(), "duplicate".into(), "orphan".into()],
                category: DiagnosticCategory::DataCorruption,
                root_cause_template: "Data integrity violation: {title}".into(),
                contributing_factors: vec!["Missing constraints".into(), "Race conditions".into(), "Failed migrations".into()],
                affected_layers: vec!["Database".into(), "ETL".into(), "Application".into()],
            },
            // Compliance
            DiagnosticPattern {
                domain: Domain::Compliance,
                keywords: vec!["gdpr".into(), "hipaa".into(), "sox".into(), "pci".into(), "audit".into(), "regulation".into(), "violation".into()],
                category: DiagnosticCategory::PolicyViolation,
                root_cause_template: "Regulatory/policy non-compliance: {title}".into(),
                contributing_factors: vec!["Outdated policies".into(), "Missing controls".into(), "Insufficient training".into()],
                affected_layers: vec!["Governance".into(), "Data".into(), "Process".into()],
            },
            // Financial
            DiagnosticPattern {
                domain: Domain::Financial,
                keywords: vec!["cost".into(), "budget".into(), "overrun".into(), "billing".into(), "revenue".into(), "expense".into()],
                category: DiagnosticCategory::ProcessBottleneck,
                root_cause_template: "Financial process issue: {title}".into(),
                contributing_factors: vec!["Lack of cost visibility".into(), "Missing budget controls".into(), "Unoptimized spending".into()],
                affected_layers: vec!["Financial".into(), "Operations".into()],
            },
            // Operations
            DiagnosticPattern {
                domain: Domain::Operations,
                keywords: vec!["process".into(), "workflow".into(), "manual".into(), "inefficien".into(), "bottleneck".into(), "delay".into()],
                category: DiagnosticCategory::ProcessBottleneck,
                root_cause_template: "Operational process bottleneck: {title}".into(),
                contributing_factors: vec!["Manual processes".into(), "Missing automation".into(), "Poor tooling".into()],
                affected_layers: vec!["Operations".into(), "Workflow".into()],
            },
            // Dependencies
            DiagnosticPattern {
                domain: Domain::Infrastructure,
                keywords: vec!["dependency".into(), "third-party".into(), "vendor".into(), "api".into(), "upstream".into(), "downstream".into()],
                category: DiagnosticCategory::DependencyFailure,
                root_cause_template: "External dependency failure: {title}".into(),
                contributing_factors: vec!["No fallback strategy".into(), "Tight coupling".into(), "Missing circuit breaker".into()],
                affected_layers: vec!["Integration".into(), "External".into()],
            },
            // Supply Chain
            DiagnosticPattern {
                domain: Domain::SupplyChain,
                keywords: vec!["supplier".into(), "delivery".into(), "inventory".into(), "logistics".into(), "shortage".into()],
                category: DiagnosticCategory::ExternalDisruption,
                root_cause_template: "Supply chain disruption: {title}".into(),
                contributing_factors: vec!["Single source dependency".into(), "No buffer stock".into(), "Geopolitical factors".into()],
                affected_layers: vec!["Supply Chain".into(), "Logistics".into(), "Procurement".into()],
            },
            // Customer Experience
            DiagnosticPattern {
                domain: Domain::CustomerExperience,
                keywords: vec!["ux".into(), "usability".into(), "complaint".into(), "churn".into(), "satisfaction".into(), "nps".into()],
                category: DiagnosticCategory::DesignFlaw,
                root_cause_template: "Customer experience design issue: {title}".into(),
                contributing_factors: vec!["Missing user research".into(), "Technical debt in UI".into(), "No feedback loop".into()],
                affected_layers: vec!["Frontend".into(), "UX".into(), "Product".into()],
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
    fn test_diagnose_infra() {
        let engine = DiagnosticEngine::new();
        let problem = Problem::new("DB OOM", "Database out of memory crash", Domain::Infrastructure, Severity::Critical, "ops");
        let diag = engine.diagnose(&problem);
        assert!(!diag.root_cause.is_empty());
        assert!(diag.confidence > 0.0);
        assert!(!diag.diagnostic_path.is_empty());
    }

    #[test]
    fn test_diagnose_security() {
        let engine = DiagnosticEngine::new();
        let problem = Problem::new("Data breach", "Unauthorized access to user data via exploit", Domain::Security, Severity::Critical, "sec-team");
        let diag = engine.diagnose(&problem);
        assert!(matches!(diag.category, DiagnosticCategory::SecurityBreach));
    }

    #[test]
    fn test_symptom_extraction() {
        let engine = DiagnosticEngine::new();
        let problem = Problem::new("Slow API", "API response timeout and slow latency with data corruption", Domain::Performance, Severity::High, "dev");
        let diag = engine.diagnose(&problem);
        assert!(diag.symptoms.len() >= 2);
    }
}
