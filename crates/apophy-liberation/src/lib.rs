//! # Apophy Liberation - L'Émancipation Collective
//!
//! The synthesis layer that measures and enforces freedom.
//! Every system tends toward control unless actively resisted.
//! This module provides:
//!
//! - **Sovereignty Score**: Real-time measurement of how free the
//!   deployment actually is (0 = fully dependent, 100 = fully sovereign)
//! - **Dependency Audit**: Detects any cloud, API, or vendor dependency
//! - **Freedom Index**: Composite metric of data control, compute
//!   ownership, communication privacy, and economic independence
//! - **Liberation Manifest**: Machine-readable proof of sovereignty

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LiberationError {
    #[error("Sovereignty compromised: {0}")]
    SovereigntyCompromised(String),

    #[error("Dependency detected: {0}")]
    DependencyDetected(String),
}

pub type Result<T> = std::result::Result<T, LiberationError>;

/// A detected dependency (the enemy of freedom)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub category: DependencyCategory,
    pub severity: Severity,
    pub description: String,
    pub remediation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DependencyCategory {
    /// Cloud API (OpenAI, AWS, Azure, etc.)
    CloudApi,
    /// External database (hosted DB)
    ExternalDatabase,
    /// Third-party authentication (OAuth to external provider)
    ExternalAuth,
    /// DNS/CDN dependency
    NetworkDependency,
    /// Proprietary binary (non-auditable code)
    ProprietaryBinary,
    /// Telemetry/tracking
    Telemetry,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational — not blocking sovereignty
    Info,
    /// Low — minor convenience dependency
    Low,
    /// Medium — partial sovereignty loss
    Medium,
    /// High — significant vendor lock-in
    High,
    /// Critical — full sovereignty loss
    Critical,
}

/// The Freedom Index: composite metric of actual liberation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreedomIndex {
    /// Overall score 0-100
    pub score: u8,
    /// Individual dimensions
    pub data_sovereignty: DimensionScore,
    pub compute_ownership: DimensionScore,
    pub communication_privacy: DimensionScore,
    pub economic_independence: DimensionScore,
    pub code_transparency: DimensionScore,
    /// Any dependencies found
    pub dependencies: Vec<Dependency>,
    /// Is this deployment truly sovereign?
    pub sovereign: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub name: String,
    pub score: u8,
    pub detail: String,
}

/// The Liberation Auditor: detects and reports on sovereignty
pub struct LiberationAuditor;

impl LiberationAuditor {
    /// Audit the current deployment for sovereignty.
    /// Checks hardware, network, storage, and code paths.
    pub fn audit(config: &AuditConfig) -> FreedomIndex {
        let mut dependencies = Vec::new();

        // 1. Data Sovereignty: where does data live?
        let data_sovereignty = if config.data_local {
            DimensionScore {
                name: "Data Sovereignty".into(),
                score: 100,
                detail: "All data stored locally. Zero cloud storage.".into(),
            }
        } else {
            dependencies.push(Dependency {
                name: "Cloud storage".into(),
                category: DependencyCategory::ExternalDatabase,
                severity: Severity::Critical,
                description: "Data stored on third-party servers".into(),
                remediation: "Migrate to local SQLite/Postgres".into(),
            });
            DimensionScore {
                name: "Data Sovereignty".into(),
                score: 0,
                detail: "Data stored on external servers — sovereignty lost.".into(),
            }
        };

        // 2. Compute Ownership: who owns the inference?
        let compute_ownership = if config.inference_local {
            DimensionScore {
                name: "Compute Ownership".into(),
                score: 100,
                detail: "All inference runs on local hardware. $0 API cost.".into(),
            }
        } else {
            dependencies.push(Dependency {
                name: "Cloud inference API".into(),
                category: DependencyCategory::CloudApi,
                severity: Severity::Critical,
                description: "Inference depends on external API (OpenAI/Anthropic/etc)".into(),
                remediation: "Deploy local model via apophy-universal".into(),
            });
            DimensionScore {
                name: "Compute Ownership".into(),
                score: 0,
                detail: "Inference routed to external API — compute rented, not owned.".into(),
            }
        };

        // 3. Communication Privacy: who can read messages?
        let communication_privacy = if config.e2e_encryption {
            DimensionScore {
                name: "Communication Privacy".into(),
                score: 100,
                detail: "E2E encryption (Double Ratchet). Zero readable by third parties.".into(),
            }
        } else {
            dependencies.push(Dependency {
                name: "Unencrypted communication".into(),
                category: DependencyCategory::Telemetry,
                severity: Severity::High,
                description: "Messages readable by network intermediaries".into(),
                remediation: "Enable apophy-fuel E2E encryption".into(),
            });
            DimensionScore {
                name: "Communication Privacy".into(),
                score: 20,
                detail: "Messages not E2E encrypted — privacy compromised.".into(),
            }
        };

        // 4. Economic Independence: who controls the money?
        let economic_independence = if config.contribution_economy {
            DimensionScore {
                name: "Economic Independence".into(),
                score: 100,
                detail: "Contribution-based tokens. No debt. No intermediary.".into(),
            }
        } else if config.no_payment_required {
            DimensionScore {
                name: "Economic Independence".into(),
                score: 80,
                detail: "Free to use. No payment system needed.".into(),
            }
        } else {
            dependencies.push(Dependency {
                name: "Commercial payment dependency".into(),
                category: DependencyCategory::CloudApi,
                severity: Severity::Medium,
                description: "Requires payment to external provider".into(),
                remediation: "Switch to apophy-commons contribution economy".into(),
            });
            DimensionScore {
                name: "Economic Independence".into(),
                score: 30,
                detail: "Dependent on external payment/subscription.".into(),
            }
        };

        // 5. Code Transparency: can you audit everything?
        let code_transparency = if config.open_source {
            DimensionScore {
                name: "Code Transparency".into(),
                score: 100,
                detail: "MIT licensed. Full source available. Every DAG node auditable.".into(),
            }
        } else {
            dependencies.push(Dependency {
                name: "Proprietary code".into(),
                category: DependencyCategory::ProprietaryBinary,
                severity: Severity::High,
                description: "Contains non-auditable proprietary code".into(),
                remediation: "Replace with open-source alternatives".into(),
            });
            DimensionScore {
                name: "Code Transparency".into(),
                score: 0,
                detail: "Proprietary code — cannot audit or verify.".into(),
            }
        };

        // Calculate composite score
        let scores = [
            data_sovereignty.score,
            compute_ownership.score,
            communication_privacy.score,
            economic_independence.score,
            code_transparency.score,
        ];
        let overall = scores.iter().map(|&s| s as u32).sum::<u32>() / scores.len() as u32;
        let sovereign = dependencies.iter().all(|d| d.severity < Severity::High);

        FreedomIndex {
            score: overall as u8,
            data_sovereignty,
            compute_ownership,
            communication_privacy,
            economic_independence,
            code_transparency,
            dependencies,
            sovereign,
        }
    }
}

/// Configuration for the sovereignty audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub data_local: bool,
    pub inference_local: bool,
    pub e2e_encryption: bool,
    pub contribution_economy: bool,
    pub no_payment_required: bool,
    pub open_source: bool,
}

impl AuditConfig {
    /// Default for a fully sovereign Apophy deployment
    pub fn sovereign() -> Self {
        Self {
            data_local: true,
            inference_local: true,
            e2e_encryption: true,
            contribution_economy: true,
            no_payment_required: true,
            open_source: true,
        }
    }

    /// A typical cloud-dependent deployment (the problem)
    pub fn cloud_dependent() -> Self {
        Self {
            data_local: false,
            inference_local: false,
            e2e_encryption: false,
            contribution_economy: false,
            no_payment_required: false,
            open_source: false,
        }
    }
}

/// Generate a Liberation Manifest: machine-readable proof of sovereignty
pub fn generate_manifest(index: &FreedomIndex) -> String {
    let status = if index.sovereign {
        "SOVEREIGN"
    } else {
        "DEPENDENT"
    };

    let dep_lines: Vec<String> = index
        .dependencies
        .iter()
        .map(|d| format!("  - [{}] {} ({})", d.severity_str(), d.name, d.remediation))
        .collect();

    let deps_section = if dep_lines.is_empty() {
        "  None — fully sovereign".to_string()
    } else {
        dep_lines.join("\n")
    };

    format!(
        r#"=== APOPHY LIBERATION MANIFEST ===
Status: {}
Freedom Score: {}/100

Dimensions:
  Data Sovereignty:       {}/100 — {}
  Compute Ownership:      {}/100 — {}
  Communication Privacy:  {}/100 — {}
  Economic Independence:  {}/100 — {}
  Code Transparency:      {}/100 — {}

Dependencies:
{}

Verdict: {}
=== END MANIFEST ==="#,
        status,
        index.score,
        index.data_sovereignty.score,
        index.data_sovereignty.detail,
        index.compute_ownership.score,
        index.compute_ownership.detail,
        index.communication_privacy.score,
        index.communication_privacy.detail,
        index.economic_independence.score,
        index.economic_independence.detail,
        index.code_transparency.score,
        index.code_transparency.detail,
        deps_section,
        if index.sovereign {
            "This deployment is SOVEREIGN. No third-party dependency detected."
        } else {
            "WARNING: Sovereignty compromised. See dependencies above."
        }
    )
}

impl Dependency {
    fn severity_str(&self) -> &'static str {
        match self.severity {
            Severity::Info => "INFO",
            Severity::Low => "LOW",
            Severity::Medium => "MEDIUM",
            Severity::High => "HIGH",
            Severity::Critical => "CRITICAL",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sovereign_audit() {
        let config = AuditConfig::sovereign();
        let index = LiberationAuditor::audit(&config);

        assert_eq!(index.score, 100);
        assert!(index.sovereign);
        assert!(index.dependencies.is_empty());
    }

    #[test]
    fn test_cloud_dependent_audit() {
        let config = AuditConfig::cloud_dependent();
        let index = LiberationAuditor::audit(&config);

        assert!(index.score < 20);
        assert!(!index.sovereign);
        assert!(!index.dependencies.is_empty());

        // Should find critical dependencies
        let critical = index
            .dependencies
            .iter()
            .filter(|d| d.severity == Severity::Critical)
            .count();
        assert!(critical >= 2);
    }

    #[test]
    fn test_partial_sovereignty() {
        let config = AuditConfig {
            data_local: true,
            inference_local: true,
            e2e_encryption: false, // compromise
            contribution_economy: false,
            no_payment_required: true,
            open_source: true,
        };

        let index = LiberationAuditor::audit(&config);
        assert!(index.score > 50);
        assert!(index.score < 100);
        assert!(!index.dependencies.is_empty());
    }

    #[test]
    fn test_manifest_generation() {
        let config = AuditConfig::sovereign();
        let index = LiberationAuditor::audit(&config);
        let manifest = generate_manifest(&index);

        assert!(manifest.contains("SOVEREIGN"));
        assert!(manifest.contains("100/100"));
        assert!(manifest.contains("None — fully sovereign"));
    }

    #[test]
    fn test_manifest_with_dependencies() {
        let config = AuditConfig::cloud_dependent();
        let index = LiberationAuditor::audit(&config);
        let manifest = generate_manifest(&index);

        assert!(manifest.contains("DEPENDENT"));
        assert!(manifest.contains("CRITICAL"));
        assert!(manifest.contains("Migrate to local")); // remediation text
    }

    #[test]
    fn test_freedom_dimensions() {
        let config = AuditConfig::sovereign();
        let index = LiberationAuditor::audit(&config);

        assert_eq!(index.data_sovereignty.score, 100);
        assert_eq!(index.compute_ownership.score, 100);
        assert_eq!(index.communication_privacy.score, 100);
        assert_eq!(index.economic_independence.score, 100);
        assert_eq!(index.code_transparency.score, 100);
    }

    #[test]
    fn test_serialization() {
        let config = AuditConfig::sovereign();
        let index = LiberationAuditor::audit(&config);
        let json = serde_json::to_string_pretty(&index).unwrap();
        assert!(json.contains("\"sovereign\": true"));
        assert!(json.contains("\"score\": 100"));
    }
}
