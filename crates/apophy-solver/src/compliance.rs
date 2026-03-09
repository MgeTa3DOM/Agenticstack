//! Compliance Checker — Regulatory framework validation
//!
//! Checks problems and solutions against regulatory frameworks
//! (GDPR, SOC2, ISO27001, HIPAA, PCI-DSS, SOX).

use crate::{Diagnosis, DiagnosticCategory, Domain, Problem};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceResult {
    pub id: Uuid,
    pub problem_id: Uuid,
    pub compliant: bool,
    pub frameworks_checked: Vec<FrameworkCheck>,
    pub violations: Vec<Violation>,
    pub required_actions: Vec<String>,
    pub reporting_obligations: Vec<ReportingObligation>,
    pub created_at: DateTime<Utc>,
}

impl std::fmt::Display for ComplianceResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Compliant: {} | Frameworks: {} | Violations: {}",
            self.compliant,
            self.frameworks_checked.len(),
            self.violations.len()
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkCheck {
    pub framework: ComplianceFramework,
    pub applicable: bool,
    pub status: FrameworkStatus,
    pub relevant_controls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComplianceFramework {
    GDPR,
    SOC2,
    ISO27001,
    HIPAA,
    PCIDSS,
    SOX,
    NIST,
    CCPA,
}

impl std::fmt::Display for ComplianceFramework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComplianceFramework::GDPR => write!(f, "GDPR"),
            ComplianceFramework::SOC2 => write!(f, "SOC 2"),
            ComplianceFramework::ISO27001 => write!(f, "ISO 27001"),
            ComplianceFramework::HIPAA => write!(f, "HIPAA"),
            ComplianceFramework::PCIDSS => write!(f, "PCI-DSS"),
            ComplianceFramework::SOX => write!(f, "SOX"),
            ComplianceFramework::NIST => write!(f, "NIST CSF"),
            ComplianceFramework::CCPA => write!(f, "CCPA"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FrameworkStatus {
    Pass,
    Warning,
    Violation,
    NotApplicable,
}

impl std::fmt::Display for FrameworkStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameworkStatus::Pass => write!(f, "PASS"),
            FrameworkStatus::Warning => write!(f, "WARNING"),
            FrameworkStatus::Violation => write!(f, "VIOLATION"),
            FrameworkStatus::NotApplicable => write!(f, "N/A"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub framework: ComplianceFramework,
    pub control_id: String,
    pub description: String,
    pub remediation: String,
    pub severity: ViolationSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Critical,
    Major,
    Minor,
    Observation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportingObligation {
    pub framework: ComplianceFramework,
    pub action: String,
    pub deadline_hours: u64,
    pub authority: String,
}

pub struct ComplianceChecker {
    pub frameworks: Vec<ComplianceFramework>,
}

impl ComplianceChecker {
    pub fn new() -> Self {
        Self {
            frameworks: vec![
                ComplianceFramework::GDPR,
                ComplianceFramework::SOC2,
                ComplianceFramework::ISO27001,
                ComplianceFramework::HIPAA,
                ComplianceFramework::PCIDSS,
                ComplianceFramework::SOX,
                ComplianceFramework::NIST,
                ComplianceFramework::CCPA,
            ],
        }
    }

    pub fn check(&self, problem: &Problem, diagnosis: &Diagnosis) -> ComplianceResult {
        let mut framework_checks = Vec::new();
        let mut violations = Vec::new();
        let mut required_actions = Vec::new();
        let mut reporting_obligations = Vec::new();

        for framework in &self.frameworks {
            let (applicable, status, controls, violation) =
                self.check_framework(framework, problem, diagnosis);

            framework_checks.push(FrameworkCheck {
                framework: framework.clone(),
                applicable,
                status: status.clone(),
                relevant_controls: controls,
            });

            if let Some(v) = violation {
                violations.push(v);
            }

            if status == FrameworkStatus::Violation {
                if let Some(obligation) = self.get_reporting_obligation(framework, problem) {
                    reporting_obligations.push(obligation);
                }
            }
        }

        // Generate required actions
        if !violations.is_empty() {
            required_actions.push("Document the incident with full timeline".into());
            required_actions.push("Notify Data Protection Officer if personal data affected".into());
            for v in &violations {
                required_actions.push(v.remediation.clone());
            }
        }

        let compliant = violations.is_empty();

        ComplianceResult {
            id: Uuid::new_v4(),
            problem_id: problem.id,
            compliant,
            frameworks_checked: framework_checks,
            violations,
            required_actions,
            reporting_obligations,
            created_at: Utc::now(),
        }
    }

    fn check_framework(
        &self,
        framework: &ComplianceFramework,
        problem: &Problem,
        diagnosis: &Diagnosis,
    ) -> (bool, FrameworkStatus, Vec<String>, Option<Violation>) {
        match framework {
            ComplianceFramework::GDPR => self.check_gdpr(problem, diagnosis),
            ComplianceFramework::SOC2 => self.check_soc2(problem, diagnosis),
            ComplianceFramework::ISO27001 => self.check_iso27001(problem, diagnosis),
            ComplianceFramework::HIPAA => self.check_hipaa(problem, diagnosis),
            ComplianceFramework::PCIDSS => self.check_pcidss(problem, diagnosis),
            ComplianceFramework::SOX => self.check_sox(problem, diagnosis),
            ComplianceFramework::NIST => self.check_nist(problem, diagnosis),
            ComplianceFramework::CCPA => self.check_ccpa(problem, diagnosis),
        }
    }

    fn check_gdpr(&self, problem: &Problem, diagnosis: &Diagnosis) -> (bool, FrameworkStatus, Vec<String>, Option<Violation>) {
        let applicable = matches!(problem.domain, Domain::Security | Domain::DataIntegrity | Domain::Compliance);
        if !applicable {
            return (false, FrameworkStatus::NotApplicable, vec![], None);
        }
        let is_breach = diagnosis.category == DiagnosticCategory::SecurityBreach
            || diagnosis.category == DiagnosticCategory::DataCorruption;
        if is_breach {
            (true, FrameworkStatus::Violation, vec!["Art. 33 — Breach notification".into(), "Art. 34 — Data subject notification".into()],
                Some(Violation {
                    framework: ComplianceFramework::GDPR,
                    control_id: "GDPR-Art33".into(),
                    description: "Personal data breach requires notification within 72 hours".into(),
                    remediation: "Notify supervisory authority within 72h, assess data subject impact".into(),
                    severity: ViolationSeverity::Critical,
                }))
        } else {
            (true, FrameworkStatus::Pass, vec!["Art. 32 — Security of processing".into()], None)
        }
    }

    fn check_soc2(&self, problem: &Problem, diagnosis: &Diagnosis) -> (bool, FrameworkStatus, Vec<String>, Option<Violation>) {
        let applicable = matches!(problem.domain, Domain::Security | Domain::Infrastructure | Domain::Operations);
        if !applicable {
            return (false, FrameworkStatus::NotApplicable, vec![], None);
        }
        let has_issue = diagnosis.category == DiagnosticCategory::SecurityBreach
            || (problem.domain == Domain::Infrastructure && diagnosis.category == DiagnosticCategory::ResourceExhaustion);
        if has_issue {
            (true, FrameworkStatus::Warning, vec!["CC6.1 — Logical access".into(), "CC7.2 — System monitoring".into()],
                Some(Violation {
                    framework: ComplianceFramework::SOC2,
                    control_id: "SOC2-CC7.2".into(),
                    description: "System monitoring control deficiency detected".into(),
                    remediation: "Review and enhance monitoring controls, update SOC 2 evidence".into(),
                    severity: ViolationSeverity::Major,
                }))
        } else {
            (true, FrameworkStatus::Pass, vec!["CC6.1 — Logical access".into()], None)
        }
    }

    fn check_iso27001(&self, _problem: &Problem, diagnosis: &Diagnosis) -> (bool, FrameworkStatus, Vec<String>, Option<Violation>) {
        let is_security = diagnosis.category == DiagnosticCategory::SecurityBreach;
        if is_security {
            (true, FrameworkStatus::Warning, vec!["A.16.1 — Incident management".into(), "A.12.4 — Logging and monitoring".into()],
                Some(Violation {
                    framework: ComplianceFramework::ISO27001,
                    control_id: "ISO27001-A.16.1".into(),
                    description: "Security incident management procedure triggered".into(),
                    remediation: "Execute incident response plan, document findings".into(),
                    severity: ViolationSeverity::Major,
                }))
        } else {
            (true, FrameworkStatus::Pass, vec!["A.12.1 — Operational procedures".into()], None)
        }
    }

    fn check_hipaa(&self, problem: &Problem, diagnosis: &Diagnosis) -> (bool, FrameworkStatus, Vec<String>, Option<Violation>) {
        let applicable = matches!(problem.domain, Domain::DataIntegrity | Domain::Security);
        if !applicable {
            return (false, FrameworkStatus::NotApplicable, vec![], None);
        }
        let is_phi_risk = diagnosis.category == DiagnosticCategory::DataCorruption
            || diagnosis.category == DiagnosticCategory::SecurityBreach;
        if is_phi_risk {
            (true, FrameworkStatus::Violation, vec!["§164.308 — Administrative safeguards".into()],
                Some(Violation {
                    framework: ComplianceFramework::HIPAA,
                    control_id: "HIPAA-164.308".into(),
                    description: "Potential PHI exposure — breach notification required".into(),
                    remediation: "Assess PHI impact, notify HHS within 60 days if >500 individuals".into(),
                    severity: ViolationSeverity::Critical,
                }))
        } else {
            (true, FrameworkStatus::Pass, vec!["§164.312 — Technical safeguards".into()], None)
        }
    }

    fn check_pcidss(&self, problem: &Problem, diagnosis: &Diagnosis) -> (bool, FrameworkStatus, Vec<String>, Option<Violation>) {
        let applicable = matches!(problem.domain, Domain::Security | Domain::Financial);
        if !applicable {
            return (false, FrameworkStatus::NotApplicable, vec![], None);
        }
        if diagnosis.category == DiagnosticCategory::SecurityBreach {
            (true, FrameworkStatus::Violation, vec!["Req 10 — Track and monitor access".into(), "Req 12 — Security policy".into()],
                Some(Violation {
                    framework: ComplianceFramework::PCIDSS,
                    control_id: "PCI-Req10".into(),
                    description: "Cardholder data environment may be compromised".into(),
                    remediation: "Engage PCI Forensic Investigator, notify payment brands".into(),
                    severity: ViolationSeverity::Critical,
                }))
        } else {
            (true, FrameworkStatus::Pass, vec!["Req 6 — Secure systems".into()], None)
        }
    }

    fn check_sox(&self, problem: &Problem, _diagnosis: &Diagnosis) -> (bool, FrameworkStatus, Vec<String>, Option<Violation>) {
        let applicable = matches!(problem.domain, Domain::Financial | Domain::Compliance);
        if !applicable {
            return (false, FrameworkStatus::NotApplicable, vec![], None);
        }
        if problem.domain == Domain::Financial {
            (true, FrameworkStatus::Warning, vec!["Section 302 — Corporate responsibility".into(), "Section 404 — Internal controls".into()], None)
        } else {
            (true, FrameworkStatus::Pass, vec!["Section 404 — Internal controls".into()], None)
        }
    }

    fn check_nist(&self, _problem: &Problem, diagnosis: &Diagnosis) -> (bool, FrameworkStatus, Vec<String>, Option<Violation>) {
        let is_security = diagnosis.category == DiagnosticCategory::SecurityBreach;
        if is_security {
            (true, FrameworkStatus::Warning, vec!["DE.CM — Security continuous monitoring".into(), "RS.RP — Response planning".into()], None)
        } else {
            (true, FrameworkStatus::Pass, vec!["ID.AM — Asset management".into()], None)
        }
    }

    fn check_ccpa(&self, problem: &Problem, diagnosis: &Diagnosis) -> (bool, FrameworkStatus, Vec<String>, Option<Violation>) {
        let applicable = matches!(problem.domain, Domain::DataIntegrity | Domain::Security);
        if !applicable {
            return (false, FrameworkStatus::NotApplicable, vec![], None);
        }
        if diagnosis.category == DiagnosticCategory::SecurityBreach {
            (true, FrameworkStatus::Violation, vec!["§1798.150 — Data breach".into()],
                Some(Violation {
                    framework: ComplianceFramework::CCPA,
                    control_id: "CCPA-1798.150".into(),
                    description: "California consumer data may be affected".into(),
                    remediation: "Assess CA consumer impact, provide breach notification".into(),
                    severity: ViolationSeverity::Major,
                }))
        } else {
            (true, FrameworkStatus::Pass, vec!["§1798.100 — Consumer rights".into()], None)
        }
    }

    fn get_reporting_obligation(
        &self,
        framework: &ComplianceFramework,
        _problem: &Problem,
    ) -> Option<ReportingObligation> {
        match framework {
            ComplianceFramework::GDPR => Some(ReportingObligation {
                framework: ComplianceFramework::GDPR,
                action: "Notify supervisory authority".into(),
                deadline_hours: 72,
                authority: "Data Protection Authority (DPA)".into(),
            }),
            ComplianceFramework::HIPAA => Some(ReportingObligation {
                framework: ComplianceFramework::HIPAA,
                action: "Notify HHS and affected individuals".into(),
                deadline_hours: 1440, // 60 days
                authority: "U.S. Department of Health and Human Services".into(),
            }),
            ComplianceFramework::PCIDSS => Some(ReportingObligation {
                framework: ComplianceFramework::PCIDSS,
                action: "Notify payment card brands and engage PFI".into(),
                deadline_hours: 24,
                authority: "Payment Card Brands (Visa, Mastercard, etc.)".into(),
            }),
            _ => None,
        }
    }
}

impl Default for ComplianceChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DiagnosticEngine;

    #[test]
    fn test_security_breach_compliance() {
        let checker = ComplianceChecker::new();
        let p = Problem::new("Data breach", "Unauthorized access to user database", Domain::Security, crate::Severity::Critical, "sec");
        let d = DiagnosticEngine::new().diagnose(&p);
        let result = checker.check(&p, &d);
        assert!(!result.compliant);
        assert!(!result.violations.is_empty());
        assert!(!result.reporting_obligations.is_empty());
    }

    #[test]
    fn test_ops_issue_compliant() {
        let checker = ComplianceChecker::new();
        let p = Problem::new("Slow deploy", "Deployment takes 30 minutes", Domain::Operations, crate::Severity::Low, "dev");
        let d = DiagnosticEngine::new().diagnose(&p);
        let result = checker.check(&p, &d);
        assert!(result.compliant);
    }
}
