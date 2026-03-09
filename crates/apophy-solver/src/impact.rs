//! Impact Analyzer — Business impact assessment and blast radius calculation
//!
//! Quantifies the business impact of problems across multiple dimensions:
//! revenue, users, reputation, operations, and compliance.

use crate::{Diagnosis, DiagnosticCategory, Domain, Problem, Severity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactReport {
    pub id: Uuid,
    pub problem_id: Uuid,
    pub overall_impact_score: f64,
    pub blast_radius: BlastRadius,
    pub dimensions: ImpactDimensions,
    pub estimated_cost_per_hour: f64,
    pub affected_user_estimate: u64,
    pub cascading_risks: Vec<CascadingRisk>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastRadius {
    pub direct_systems: usize,
    pub indirect_systems: usize,
    pub total_scope: BlastScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BlastScope {
    Isolated,
    Team,
    Department,
    Organization,
    External,
}

impl std::fmt::Display for BlastScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactDimensions {
    pub revenue: f64,
    pub user_experience: f64,
    pub reputation: f64,
    pub operational: f64,
    pub compliance_risk: f64,
    pub data_loss_risk: f64,
}

impl ImpactDimensions {
    pub fn weighted_score(&self) -> f64 {
        self.revenue * 0.25
            + self.user_experience * 0.20
            + self.reputation * 0.20
            + self.operational * 0.15
            + self.compliance_risk * 0.10
            + self.data_loss_risk * 0.10
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CascadingRisk {
    pub description: String,
    pub probability: f64,
    pub impact_if_triggered: f64,
}

pub struct ImpactAnalyzer {
    pub cost_model: CostModel,
}

#[derive(Debug, Clone)]
pub struct CostModel {
    pub revenue_per_hour: f64,
    pub avg_users_affected_pct: f64,
    pub total_users: u64,
}

impl Default for CostModel {
    fn default() -> Self {
        Self {
            revenue_per_hour: 10_000.0,
            avg_users_affected_pct: 0.1,
            total_users: 100_000,
        }
    }
}

impl ImpactAnalyzer {
    pub fn new() -> Self {
        Self {
            cost_model: CostModel::default(),
        }
    }

    pub fn analyze(&self, problem: &Problem, diagnosis: &Diagnosis) -> ImpactReport {
        let severity_factor = problem.severity.weight();
        let confidence_factor = diagnosis.confidence;

        // Calculate impact dimensions
        let dimensions = self.calculate_dimensions(problem, diagnosis, severity_factor);

        // Calculate blast radius
        let blast_radius = self.calculate_blast_radius(problem, diagnosis);

        // Estimate costs
        let cost_per_hour = self.cost_model.revenue_per_hour * severity_factor * confidence_factor;
        let affected_users = (self.cost_model.total_users as f64
            * self.cost_model.avg_users_affected_pct
            * severity_factor) as u64;

        // Identify cascading risks
        let cascading_risks = self.identify_cascading_risks(problem, diagnosis);

        ImpactReport {
            id: Uuid::new_v4(),
            problem_id: problem.id,
            overall_impact_score: dimensions.weighted_score(),
            blast_radius,
            dimensions,
            estimated_cost_per_hour: cost_per_hour,
            affected_user_estimate: affected_users,
            cascading_risks,
            created_at: Utc::now(),
        }
    }

    fn calculate_dimensions(
        &self,
        problem: &Problem,
        diagnosis: &Diagnosis,
        severity_factor: f64,
    ) -> ImpactDimensions {
        let base = severity_factor;

        let (revenue, ux, reputation, ops, compliance, data_loss) = match problem.domain {
            Domain::Infrastructure => (base * 0.8, base * 0.9, base * 0.6, base * 1.0, base * 0.3, base * 0.5),
            Domain::Security => (base * 0.7, base * 0.5, base * 1.0, base * 0.6, base * 0.9, base * 0.8),
            Domain::Performance => (base * 0.6, base * 1.0, base * 0.7, base * 0.5, base * 0.2, base * 0.1),
            Domain::DataIntegrity => (base * 0.5, base * 0.6, base * 0.8, base * 0.7, base * 0.7, base * 1.0),
            Domain::Compliance => (base * 0.3, base * 0.2, base * 0.9, base * 0.4, base * 1.0, base * 0.3),
            Domain::Financial => (base * 1.0, base * 0.3, base * 0.7, base * 0.5, base * 0.8, base * 0.2),
            Domain::Operations => (base * 0.5, base * 0.4, base * 0.3, base * 1.0, base * 0.3, base * 0.2),
            Domain::HumanResources => (base * 0.2, base * 0.1, base * 0.5, base * 0.6, base * 0.7, base * 0.3),
            Domain::CustomerExperience => (base * 0.7, base * 1.0, base * 0.9, base * 0.3, base * 0.2, base * 0.1),
            Domain::SupplyChain => (base * 0.8, base * 0.4, base * 0.5, base * 0.9, base * 0.3, base * 0.2),
            Domain::Legal => (base * 0.6, base * 0.1, base * 0.8, base * 0.4, base * 1.0, base * 0.2),
            Domain::Strategy => (base * 0.4, base * 0.2, base * 0.6, base * 0.5, base * 0.3, base * 0.1),
        };

        // Boost based on root cause
        let boost = match diagnosis.category {
            DiagnosticCategory::SecurityBreach => 1.2,
            DiagnosticCategory::DataCorruption => 1.15,
            _ => 1.0,
        };

        ImpactDimensions {
            revenue: (revenue * boost).min(1.0),
            user_experience: (ux * boost).min(1.0),
            reputation: (reputation * boost).min(1.0),
            operational: (ops * boost).min(1.0),
            compliance_risk: (compliance * boost).min(1.0),
            data_loss_risk: (data_loss * boost).min(1.0),
        }
    }

    fn calculate_blast_radius(&self, problem: &Problem, _diagnosis: &Diagnosis) -> BlastRadius {
        let direct = problem.affected_systems.len().max(1);
        let indirect = (direct as f64 * problem.severity.weight() * 2.0) as usize;

        let scope = match (problem.severity.clone(), &problem.domain) {
            (Severity::Critical, Domain::Infrastructure) => BlastScope::External,
            (Severity::Critical, _) => BlastScope::Organization,
            (Severity::High, Domain::CustomerExperience) => BlastScope::External,
            (Severity::High, _) => BlastScope::Department,
            (Severity::Medium, _) => BlastScope::Team,
            _ => BlastScope::Isolated,
        };

        BlastRadius {
            direct_systems: direct,
            indirect_systems: indirect,
            total_scope: scope,
        }
    }

    fn identify_cascading_risks(
        &self,
        problem: &Problem,
        diagnosis: &Diagnosis,
    ) -> Vec<CascadingRisk> {
        let mut risks = Vec::new();

        match diagnosis.category {
            DiagnosticCategory::ResourceExhaustion => {
                risks.push(CascadingRisk {
                    description: "Service cascade failure — dependent services may fail".into(),
                    probability: 0.6,
                    impact_if_triggered: 0.9,
                });
            }
            DiagnosticCategory::SecurityBreach => {
                risks.push(CascadingRisk {
                    description: "Data exfiltration — sensitive data may be compromised".into(),
                    probability: 0.4,
                    impact_if_triggered: 1.0,
                });
                risks.push(CascadingRisk {
                    description: "Lateral movement — attacker may pivot to other systems".into(),
                    probability: 0.3,
                    impact_if_triggered: 0.95,
                });
            }
            DiagnosticCategory::DataCorruption => {
                risks.push(CascadingRisk {
                    description: "Downstream data pollution — corrupted data propagates".into(),
                    probability: 0.5,
                    impact_if_triggered: 0.85,
                });
            }
            _ => {}
        }

        if problem.severity == Severity::Critical {
            risks.push(CascadingRisk {
                description: "Customer trust erosion — SLA breach may trigger contractual penalties".into(),
                probability: 0.4,
                impact_if_triggered: 0.7,
            });
        }

        risks
    }
}

impl Default for ImpactAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DiagnosticEngine;

    #[test]
    fn test_impact_analysis() {
        let analyzer = ImpactAnalyzer::new();
        let p = Problem::new("Outage", "Full service outage", Domain::Infrastructure, Severity::Critical, "ops");
        let d = DiagnosticEngine::new().diagnose(&p);
        let impact = analyzer.analyze(&p, &d);
        assert!(impact.overall_impact_score > 0.0);
        assert!(impact.estimated_cost_per_hour > 0.0);
        assert_eq!(impact.blast_radius.total_scope, BlastScope::External);
    }

    #[test]
    fn test_low_impact() {
        let analyzer = ImpactAnalyzer::new();
        let p = Problem::new("Minor", "Small UI glitch", Domain::CustomerExperience, Severity::Low, "qa");
        let d = DiagnosticEngine::new().diagnose(&p);
        let impact = analyzer.analyze(&p, &d);
        assert!(impact.overall_impact_score < 0.5);
    }
}
