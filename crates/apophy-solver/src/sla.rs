//! SLA Tracker — Service Level Agreement monitoring
//!
//! Tracks response and resolution times against SLA policies.
//! Triggers alerts when SLA breach thresholds are approached.

use crate::{Problem, Severity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaRecord {
    pub id: Uuid,
    pub problem_id: Uuid,
    pub policy: SlaPolicy,
    pub status: SlaStatus,
    pub response_deadline: DateTime<Utc>,
    pub resolution_deadline: DateTime<Utc>,
    pub time_to_respond_min: Option<f64>,
    pub time_to_resolve_min: Option<f64>,
    pub breach_risk_pct: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaPolicy {
    pub name: String,
    pub severity: Severity,
    pub response_time_min: f64,
    pub resolution_time_min: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SlaStatus {
    OnTrack,
    AtRisk,
    Breached,
}

impl std::fmt::Display for SlaStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlaStatus::OnTrack => write!(f, "ON_TRACK"),
            SlaStatus::AtRisk => write!(f, "AT_RISK"),
            SlaStatus::Breached => write!(f, "BREACHED"),
        }
    }
}

pub struct SlaTracker {
    pub policies: Vec<SlaPolicy>,
}

impl SlaTracker {
    pub fn new() -> Self {
        Self {
            policies: vec![
                SlaPolicy {
                    name: "Critical — P1".into(),
                    severity: Severity::Critical,
                    response_time_min: 15.0,
                    resolution_time_min: 240.0, // 4 hours
                },
                SlaPolicy {
                    name: "High — P2".into(),
                    severity: Severity::High,
                    response_time_min: 60.0,
                    resolution_time_min: 480.0, // 8 hours
                },
                SlaPolicy {
                    name: "Medium — P3".into(),
                    severity: Severity::Medium,
                    response_time_min: 240.0,  // 4 hours
                    resolution_time_min: 2880.0, // 48 hours
                },
                SlaPolicy {
                    name: "Low — P4".into(),
                    severity: Severity::Low,
                    response_time_min: 1440.0,  // 24 hours
                    resolution_time_min: 10080.0, // 7 days
                },
                SlaPolicy {
                    name: "Info — P5".into(),
                    severity: Severity::Info,
                    response_time_min: 4320.0,   // 3 days
                    resolution_time_min: 43200.0, // 30 days
                },
            ],
        }
    }

    pub fn track(&self, problem: &Problem) -> SlaRecord {
        let policy = self.policies.iter()
            .find(|p| p.severity == problem.severity)
            .cloned()
            .unwrap_or(SlaPolicy {
                name: "Default".into(),
                severity: problem.severity.clone(),
                response_time_min: 60.0,
                resolution_time_min: 480.0,
            });

        let now = Utc::now();
        let elapsed_min = (now - problem.created_at).num_seconds() as f64 / 60.0;

        let response_deadline = problem.created_at
            + chrono::Duration::minutes(policy.response_time_min as i64);
        let resolution_deadline = problem.created_at
            + chrono::Duration::minutes(policy.resolution_time_min as i64);

        let breach_risk = (elapsed_min / policy.resolution_time_min * 100.0).min(100.0);

        let status = if elapsed_min > policy.resolution_time_min {
            SlaStatus::Breached
        } else if breach_risk > 75.0 {
            SlaStatus::AtRisk
        } else {
            SlaStatus::OnTrack
        };

        SlaRecord {
            id: Uuid::new_v4(),
            problem_id: problem.id,
            policy,
            status,
            response_deadline,
            resolution_deadline,
            time_to_respond_min: Some(elapsed_min.min(15.0)), // Assume immediate response from solver
            time_to_resolve_min: None, // Not yet resolved
            breach_risk_pct: breach_risk,
            created_at: now,
        }
    }
}

impl Default for SlaTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Domain;

    #[test]
    fn test_sla_tracking() {
        let tracker = SlaTracker::new();
        let p = Problem::new("Test", "Desc", Domain::Infrastructure, Severity::Critical, "ops");
        let sla = tracker.track(&p);
        assert_eq!(sla.status, SlaStatus::OnTrack);
        assert!(sla.breach_risk_pct < 50.0);
    }

    #[test]
    fn test_sla_policies() {
        let tracker = SlaTracker::new();
        assert_eq!(tracker.policies.len(), 5);
        assert_eq!(tracker.policies[0].response_time_min, 15.0);
    }
}
