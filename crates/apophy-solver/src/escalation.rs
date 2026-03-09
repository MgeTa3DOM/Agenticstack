//! Escalation Manager — Automated escalation logic
//!
//! Determines when problems require escalation based on risk,
//! SLA status, and organizational rules.

use crate::{Problem, RiskAssessment, RiskLevel, Severity, SlaRecord, SlaStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationDecision {
    pub id: Uuid,
    pub problem_id: Uuid,
    pub should_escalate: bool,
    pub level: EscalationLevel,
    pub reasons: Vec<String>,
    pub notify: Vec<NotificationTarget>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EscalationLevel {
    None,
    L1Support,
    L2Engineering,
    L3Principal,
    Management,
    Executive,
    WarRoom,
}

impl std::fmt::Display for EscalationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EscalationLevel::None => write!(f, "None"),
            EscalationLevel::L1Support => write!(f, "L1 Support"),
            EscalationLevel::L2Engineering => write!(f, "L2 Engineering"),
            EscalationLevel::L3Principal => write!(f, "L3 Principal"),
            EscalationLevel::Management => write!(f, "Management"),
            EscalationLevel::Executive => write!(f, "Executive"),
            EscalationLevel::WarRoom => write!(f, "WAR ROOM"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationTarget {
    pub role: String,
    pub channel: NotificationChannel,
    pub urgency: NotificationUrgency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email,
    Slack,
    PagerDuty,
    Phone,
    Dashboard,
}

impl std::fmt::Display for NotificationChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationUrgency {
    Informational,
    Normal,
    Urgent,
    Critical,
}

pub struct EscalationManager {
    pub rules: Vec<EscalationRule>,
}

#[derive(Debug, Clone)]
pub struct EscalationRule {
    pub name: String,
    pub condition: EscalationCondition,
    pub level: EscalationLevel,
    pub notifications: Vec<NotificationTarget>,
}

#[derive(Debug, Clone)]
pub enum EscalationCondition {
    RiskAbove(f64),
    SlaBreached,
    SlaAtRisk,
    SeverityCritical,
    CombinedHighRiskAndSla,
}

impl EscalationManager {
    pub fn new() -> Self {
        Self {
            rules: Self::default_rules(),
        }
    }

    pub fn evaluate(
        &self,
        problem: &Problem,
        risk: &RiskAssessment,
        sla: &SlaRecord,
    ) -> EscalationDecision {
        let mut should_escalate = false;
        let mut highest_level = EscalationLevel::None;
        let mut reasons = Vec::new();
        let mut notify = Vec::new();

        for rule in &self.rules {
            let triggered = match &rule.condition {
                EscalationCondition::RiskAbove(threshold) => risk.risk_score > *threshold,
                EscalationCondition::SlaBreached => sla.status == SlaStatus::Breached,
                EscalationCondition::SlaAtRisk => sla.status == SlaStatus::AtRisk,
                EscalationCondition::SeverityCritical => problem.severity == Severity::Critical,
                EscalationCondition::CombinedHighRiskAndSla => {
                    risk.risk_level == RiskLevel::High && sla.status == SlaStatus::AtRisk
                }
            };

            if triggered {
                should_escalate = true;
                reasons.push(format!("Rule '{}' triggered", rule.name));
                notify.extend(rule.notifications.clone());

                if Self::level_priority(&rule.level) > Self::level_priority(&highest_level) {
                    highest_level = rule.level.clone();
                }
            }
        }

        EscalationDecision {
            id: Uuid::new_v4(),
            problem_id: problem.id,
            should_escalate,
            level: highest_level,
            reasons,
            notify,
            created_at: Utc::now(),
        }
    }

    fn level_priority(level: &EscalationLevel) -> u8 {
        match level {
            EscalationLevel::None => 0,
            EscalationLevel::L1Support => 1,
            EscalationLevel::L2Engineering => 2,
            EscalationLevel::L3Principal => 3,
            EscalationLevel::Management => 4,
            EscalationLevel::Executive => 5,
            EscalationLevel::WarRoom => 6,
        }
    }

    fn default_rules() -> Vec<EscalationRule> {
        vec![
            EscalationRule {
                name: "Critical severity auto-escalate".into(),
                condition: EscalationCondition::SeverityCritical,
                level: EscalationLevel::L3Principal,
                notifications: vec![
                    NotificationTarget {
                        role: "On-Call Engineer".into(),
                        channel: NotificationChannel::PagerDuty,
                        urgency: NotificationUrgency::Critical,
                    },
                    NotificationTarget {
                        role: "Engineering Manager".into(),
                        channel: NotificationChannel::Slack,
                        urgency: NotificationUrgency::Urgent,
                    },
                ],
            },
            EscalationRule {
                name: "Critical risk score".into(),
                condition: EscalationCondition::RiskAbove(0.8),
                level: EscalationLevel::Executive,
                notifications: vec![
                    NotificationTarget {
                        role: "VP Engineering".into(),
                        channel: NotificationChannel::Phone,
                        urgency: NotificationUrgency::Critical,
                    },
                    NotificationTarget {
                        role: "CTO".into(),
                        channel: NotificationChannel::Email,
                        urgency: NotificationUrgency::Critical,
                    },
                ],
            },
            EscalationRule {
                name: "SLA breach".into(),
                condition: EscalationCondition::SlaBreached,
                level: EscalationLevel::Management,
                notifications: vec![
                    NotificationTarget {
                        role: "Service Owner".into(),
                        channel: NotificationChannel::Slack,
                        urgency: NotificationUrgency::Urgent,
                    },
                ],
            },
            EscalationRule {
                name: "SLA at risk".into(),
                condition: EscalationCondition::SlaAtRisk,
                level: EscalationLevel::L2Engineering,
                notifications: vec![
                    NotificationTarget {
                        role: "Team Lead".into(),
                        channel: NotificationChannel::Slack,
                        urgency: NotificationUrgency::Normal,
                    },
                ],
            },
            EscalationRule {
                name: "High risk + SLA pressure".into(),
                condition: EscalationCondition::CombinedHighRiskAndSla,
                level: EscalationLevel::WarRoom,
                notifications: vec![
                    NotificationTarget {
                        role: "Incident Commander".into(),
                        channel: NotificationChannel::PagerDuty,
                        urgency: NotificationUrgency::Critical,
                    },
                    NotificationTarget {
                        role: "All Hands".into(),
                        channel: NotificationChannel::Slack,
                        urgency: NotificationUrgency::Critical,
                    },
                ],
            },
        ]
    }
}

impl Default for EscalationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DiagnosticEngine, Domain, RiskMatrix, SlaTracker};

    #[test]
    fn test_critical_escalation() {
        let manager = EscalationManager::new();
        let p = Problem::new("Outage", "Full outage", Domain::Infrastructure, Severity::Critical, "ops");
        let d = DiagnosticEngine::new().diagnose(&p);
        let risk = RiskMatrix::new().assess(&p, &d);
        let sla = SlaTracker::new().track(&p);
        let decision = manager.evaluate(&p, &risk, &sla);
        assert!(decision.should_escalate);
        assert!(!decision.reasons.is_empty());
    }

    #[test]
    fn test_low_no_escalation() {
        let manager = EscalationManager::new();
        let p = Problem::new("Typo", "Docs typo", Domain::Operations, Severity::Info, "dev");
        let d = DiagnosticEngine::new().diagnose(&p);
        let risk = RiskMatrix::new().assess(&p, &d);
        let sla = SlaTracker::new().track(&p);
        let decision = manager.evaluate(&p, &risk, &sla);
        assert!(!decision.should_escalate);
        assert_eq!(decision.level, EscalationLevel::None);
    }
}
