//! Risk Matrix — Probability x Impact risk scoring with thresholds
//!
//! Enterprise risk assessment engine using configurable risk matrix.
//! Computes composite risk scores and recommends mitigation priority.

use crate::{Diagnosis, DiagnosticCategory, Domain, Problem};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub id: Uuid,
    pub problem_id: Uuid,
    pub risk_score: f64,
    pub probability: f64,
    pub impact: f64,
    pub risk_level: RiskLevel,
    pub risk_factors: Vec<RiskFactor>,
    pub mitigation_priority: MitigationPriority,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Critical,
    High,
    Medium,
    Low,
    Negligible,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub name: String,
    pub score: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MitigationPriority {
    Immediate,
    Urgent,
    High,
    Normal,
    Low,
}

impl std::fmt::Display for MitigationPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct RiskMatrix {
    pub thresholds: Vec<RiskThreshold>,
}

#[derive(Debug, Clone)]
pub struct RiskThreshold {
    pub level: RiskLevel,
    pub min_score: f64,
    pub priority: MitigationPriority,
}

impl RiskMatrix {
    pub fn new() -> Self {
        Self {
            thresholds: vec![
                RiskThreshold { level: RiskLevel::Critical, min_score: 0.8, priority: MitigationPriority::Immediate },
                RiskThreshold { level: RiskLevel::High, min_score: 0.6, priority: MitigationPriority::Urgent },
                RiskThreshold { level: RiskLevel::Medium, min_score: 0.4, priority: MitigationPriority::High },
                RiskThreshold { level: RiskLevel::Low, min_score: 0.2, priority: MitigationPriority::Normal },
                RiskThreshold { level: RiskLevel::Negligible, min_score: 0.0, priority: MitigationPriority::Low },
            ],
        }
    }

    pub fn assess(&self, problem: &Problem, diagnosis: &Diagnosis) -> RiskAssessment {
        let mut risk_factors = Vec::new();

        // Factor 1: Severity
        let severity_score = problem.severity.weight();
        risk_factors.push(RiskFactor {
            name: "Severity".into(),
            score: severity_score,
            description: format!("Problem severity: {}", problem.severity),
        });

        // Factor 2: Root cause exploitability
        let exploitability = self.root_cause_risk(&diagnosis.category);
        risk_factors.push(RiskFactor {
            name: "Exploitability".into(),
            score: exploitability,
            description: format!("Root cause category: {}", diagnosis.category),
        });

        // Factor 3: Diagnosis confidence (inverse — low confidence = higher risk)
        let uncertainty = 1.0 - diagnosis.confidence;
        risk_factors.push(RiskFactor {
            name: "Uncertainty".into(),
            score: uncertainty,
            description: format!("Diagnostic confidence: {:.0}%", diagnosis.confidence * 100.0),
        });

        // Factor 4: Domain criticality
        let domain_risk = self.domain_risk(&problem.domain);
        risk_factors.push(RiskFactor {
            name: "Domain Criticality".into(),
            score: domain_risk,
            description: format!("Domain: {}", problem.domain),
        });

        // Factor 5: Blast radius
        let blast_risk = (problem.affected_systems.len() as f64 / 10.0).min(1.0);
        risk_factors.push(RiskFactor {
            name: "Blast Radius".into(),
            score: blast_risk,
            description: format!("{} affected systems", problem.affected_systems.len()),
        });

        // Compute composite probability and impact
        let probability = (severity_score * 0.3 + exploitability * 0.4 + uncertainty * 0.3).min(1.0);
        let impact = (severity_score * 0.4 + domain_risk * 0.3 + blast_risk * 0.3).min(1.0);
        let risk_score = (probability * impact).sqrt(); // geometric mean

        // Determine risk level from thresholds
        let (risk_level, priority) = self.classify(risk_score);

        RiskAssessment {
            id: Uuid::new_v4(),
            problem_id: problem.id,
            risk_score,
            probability,
            impact,
            risk_level,
            risk_factors,
            mitigation_priority: priority,
            created_at: Utc::now(),
        }
    }

    fn root_cause_risk(&self, category: &DiagnosticCategory) -> f64 {
        match category {
            DiagnosticCategory::SecurityBreach => 0.95,
            DiagnosticCategory::DataCorruption => 0.85,
            DiagnosticCategory::ResourceExhaustion => 0.7,
            DiagnosticCategory::DependencyFailure => 0.65,
            DiagnosticCategory::IntegrationFailure => 0.6,
            DiagnosticCategory::ExternalDisruption => 0.55,
            DiagnosticCategory::ConfigurationDrift => 0.5,
            DiagnosticCategory::CapacityLimit => 0.45,
            DiagnosticCategory::HumanError => 0.4,
            DiagnosticCategory::DesignFlaw => 0.35,
            DiagnosticCategory::PolicyViolation => 0.3,
            DiagnosticCategory::ProcessBottleneck => 0.25,
        }
    }

    fn domain_risk(&self, domain: &Domain) -> f64 {
        match domain {
            Domain::Security => 0.95,
            Domain::Financial => 0.9,
            Domain::Infrastructure => 0.85,
            Domain::DataIntegrity => 0.8,
            Domain::Compliance => 0.75,
            Domain::CustomerExperience => 0.65,
            Domain::SupplyChain => 0.6,
            Domain::Operations => 0.5,
            Domain::Legal => 0.7,
            Domain::HumanResources => 0.35,
            Domain::Strategy => 0.3,
            Domain::Performance => 0.55,
        }
    }

    fn classify(&self, score: f64) -> (RiskLevel, MitigationPriority) {
        for threshold in &self.thresholds {
            if score >= threshold.min_score {
                return (threshold.level.clone(), threshold.priority.clone());
            }
        }
        (RiskLevel::Negligible, MitigationPriority::Low)
    }
}

impl Default for RiskMatrix {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DiagnosticEngine, Severity};

    #[test]
    fn test_critical_risk() {
        let matrix = RiskMatrix::new();
        let p = Problem::new("Breach", "Active security breach", Domain::Security, Severity::Critical, "sec")
            .with_affected_systems(vec!["auth".into(), "db".into(), "api".into()]);
        let d = DiagnosticEngine::new().diagnose(&p);
        let r = matrix.assess(&p, &d);
        assert!(r.risk_score > 0.6);
        assert!(r.mitigation_priority == MitigationPriority::Immediate || r.mitigation_priority == MitigationPriority::Urgent);
    }

    #[test]
    fn test_low_risk() {
        let matrix = RiskMatrix::new();
        let p = Problem::new("Typo", "Typo in docs", Domain::Operations, Severity::Info, "dev");
        let d = DiagnosticEngine::new().diagnose(&p);
        let r = matrix.assess(&p, &d);
        assert!(r.risk_score < 0.5);
    }

    #[test]
    fn test_risk_factors_count() {
        let matrix = RiskMatrix::new();
        let p = Problem::new("Test", "Test issue", Domain::Performance, Severity::Medium, "test");
        let d = DiagnosticEngine::new().diagnose(&p);
        let r = matrix.assess(&p, &d);
        assert_eq!(r.risk_factors.len(), 5);
    }
}
