//! # Initializer Agent
//!
//! Runs once (or infrequently) to transform a high-level goal into
//! a machine-readable feature backlog with testable pass/fail criteria.
//!
//! The Initializer is NOT the worker — it doesn't implement features.
//! It creates the scaffolding that makes the Worker effective.
//!
//! Think of it as "the architect who draws the blueprints before
//! the builders start."

use crate::domain::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum InitializerError {
    #[error("Goal is empty")]
    EmptyGoal,
    #[error("Failed to decompose goal: {0}")]
    DecompositionFailed(String),
}

pub type Result<T> = std::result::Result<T, InitializerError>;

/// Configuration for the Initializer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializerConfig {
    /// Project name
    pub project_name: String,
    /// High-level goal to decompose
    pub goal: String,
    /// Which scaffolding rules to use
    pub rules: ScaffoldingRules,
    /// Tags to auto-apply to features
    pub default_tags: Vec<String>,
}

/// The Initializer agent — transforms goals into backlogs
pub struct Initializer {
    config: InitializerConfig,
}

impl Initializer {
    pub fn new(config: InitializerConfig) -> Self {
        Self { config }
    }

    /// Bootstrap domain memory from a structured feature list.
    ///
    /// In a full LLM-powered system, `decompose_goal()` would call the local
    /// inference engine. Here we provide the manual path: you supply the
    /// features and the Initializer structures them into proper domain memory.
    pub fn bootstrap(&self, features: Vec<FeatureSpec>) -> Result<DomainMemory> {
        if self.config.goal.is_empty() {
            return Err(InitializerError::EmptyGoal);
        }

        let mut backlog = FeatureBacklog::new(&self.config.project_name);

        // Build features with proper ordering and dependencies
        let mut feature_map: std::collections::HashMap<String, Uuid> = std::collections::HashMap::new();

        for spec in &features {
            let mut feature = Feature::new(&spec.name, spec.priority);
            feature = feature.with_criteria(spec.acceptance_criteria.clone());

            let mut tags = spec.tags.clone();
            tags.extend(self.config.default_tags.clone());
            let tag_refs: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
            feature = feature.with_tags(tag_refs);

            feature_map.insert(spec.name.clone(), feature.id);
            backlog.add_feature(feature);
        }

        // Resolve string-based dependencies to UUIDs
        for spec in &features {
            if let Some(&feature_id) = feature_map.get(&spec.name) {
                for dep_name in &spec.depends_on {
                    if let Some(&dep_id) = feature_map.get(dep_name) {
                        if let Some(feature) = backlog.features.iter_mut().find(|f| f.id == feature_id) {
                            feature.depends_on.push(dep_id);
                        }
                    }
                }
            }
        }

        Ok(DomainMemory::new(backlog, self.config.rules.clone()))
    }

    /// Create a sovereign AI infrastructure backlog (pre-built template)
    pub fn sovereign_infrastructure() -> DomainMemory {
        let config = InitializerConfig {
            project_name: "Apophy Sovereign Infrastructure".into(),
            goal: "Build a fully sovereign AI infrastructure that runs locally, \
                   needs no external API, maintains identity across sessions, \
                   and is hardened against all 2026 threat vectors".into(),
            rules: ScaffoldingRules::sovereign(),
            default_tags: vec!["sovereign".into(), "2026".into()],
        };

        let initializer = Initializer::new(config);

        let features = vec![
            FeatureSpec {
                name: "Cryptographic identity (X25519 + Ed25519)".into(),
                priority: 1,
                acceptance_criteria: vec![
                    "Key pair generation works".into(),
                    "Key exchange produces shared secret".into(),
                    "Signatures verify correctly".into(),
                    "Keys zeroize on drop".into(),
                ],
                tags: vec!["crypto".into()],
                depends_on: vec![],
            },
            FeatureSpec {
                name: "Hardware detection (CPU, GPU, RAM, NPU)".into(),
                priority: 1,
                acceptance_criteria: vec![
                    "CPU cores detected".into(),
                    "RAM amount detected".into(),
                    "GPU presence checked".into(),
                    "NPU/AI accelerator checked".into(),
                ],
                tags: vec!["hardware".into()],
                depends_on: vec![],
            },
            FeatureSpec {
                name: "Persistent memory palace (SQLite)".into(),
                priority: 2,
                acceptance_criteria: vec![
                    "Identity persists across restarts".into(),
                    "Episodic memory stores and recalls".into(),
                    "Semantic memory learns and retrieves".into(),
                    "Hash chain is tamper-proof".into(),
                ],
                tags: vec!["memory".into()],
                depends_on: vec!["Cryptographic identity (X25519 + Ed25519)".into()],
            },
            FeatureSpec {
                name: "Transparent DAG workflow engine".into(),
                priority: 2,
                acceptance_criteria: vec![
                    "Nodes execute in dependency order".into(),
                    "Cycle detection prevents infinite loops".into(),
                    "Full audit trail for every execution".into(),
                    "Parallel groups identified correctly".into(),
                ],
                tags: vec!["workflow".into()],
                depends_on: vec![],
            },
            FeatureSpec {
                name: "Local inference (no external API)".into(),
                priority: 3,
                acceptance_criteria: vec![
                    "Model loads from local path".into(),
                    "Inference produces valid output".into(),
                    "No network calls during inference".into(),
                    "GPU acceleration when available".into(),
                ],
                tags: vec!["inference".into(), "ai".into()],
                depends_on: vec!["Hardware detection (CPU, GPU, RAM, NPU)".into()],
            },
            FeatureSpec {
                name: "Security fortress (hardening + vuln scan)".into(),
                priority: 3,
                acceptance_criteria: vec![
                    "Kernel sysctl hardened".into(),
                    "Firewall zero-trust active".into(),
                    "SSH locked down".into(),
                    "AI sandbox policies configured".into(),
                    "Vulnerability scan produces report".into(),
                ],
                tags: vec!["security".into()],
                depends_on: vec![],
            },
            FeatureSpec {
                name: "Agent harness (Initializer/Worker pattern)".into(),
                priority: 4,
                acceptance_criteria: vec![
                    "Goals decompose into feature backlog".into(),
                    "Worker picks ONE feature atomically".into(),
                    "Progress log tracks all runs".into(),
                    "Loop detection prevents thrashing".into(),
                    "Domain memory persists between runs".into(),
                ],
                tags: vec!["harness".into(), "pattern".into()],
                depends_on: vec![
                    "Persistent memory palace (SQLite)".into(),
                    "Transparent DAG workflow engine".into(),
                ],
            },
            FeatureSpec {
                name: "Sovereign orchestrator (CLI + API)".into(),
                priority: 5,
                acceptance_criteria: vec![
                    "HTTP API serves all endpoints".into(),
                    "CLI commands work for all subsystems".into(),
                    "Merkabah alignment passes all crucibles".into(),
                    "Configuration loads from TOML".into(),
                ],
                tags: vec!["orchestrator".into()],
                depends_on: vec![
                    "Cryptographic identity (X25519 + Ed25519)".into(),
                    "Hardware detection (CPU, GPU, RAM, NPU)".into(),
                    "Persistent memory palace (SQLite)".into(),
                    "Transparent DAG workflow engine".into(),
                    "Security fortress (hardening + vuln scan)".into(),
                    "Agent harness (Initializer/Worker pattern)".into(),
                ],
            },
        ];

        initializer.bootstrap(features).expect("sovereign bootstrap should not fail")
    }
}

/// A feature specification (input to the Initializer)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSpec {
    pub name: String,
    pub priority: u32,
    pub acceptance_criteria: Vec<String>,
    pub tags: Vec<String>,
    /// Names of features this depends on (resolved to UUIDs by Initializer)
    pub depends_on: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> InitializerConfig {
        InitializerConfig {
            project_name: "test".into(),
            goal: "build something".into(),
            rules: ScaffoldingRules::default(),
            default_tags: vec!["test".into()],
        }
    }

    #[test]
    fn test_bootstrap_empty_goal() {
        let config = InitializerConfig {
            project_name: "test".into(),
            goal: String::new(),
            rules: ScaffoldingRules::default(),
            default_tags: vec![],
        };
        let init = Initializer::new(config);
        assert!(init.bootstrap(vec![]).is_err());
    }

    #[test]
    fn test_bootstrap_creates_backlog() {
        let init = Initializer::new(test_config());
        let features = vec![
            FeatureSpec {
                name: "Feature A".into(),
                priority: 1,
                acceptance_criteria: vec!["it works".into()],
                tags: vec!["core".into()],
                depends_on: vec![],
            },
            FeatureSpec {
                name: "Feature B".into(),
                priority: 2,
                acceptance_criteria: vec!["it also works".into()],
                tags: vec![],
                depends_on: vec!["Feature A".into()],
            },
        ];

        let memory = init.bootstrap(features).unwrap();
        assert_eq!(memory.backlog.total(), 2);
        assert_eq!(memory.backlog.pending(), 2);

        // Feature B depends on Feature A
        let fb = &memory.backlog.features[1];
        assert_eq!(fb.depends_on.len(), 1);

        // Next actionable should be A (B is blocked)
        let next = memory.backlog.next_actionable().unwrap();
        assert_eq!(next.name, "Feature A");
    }

    #[test]
    fn test_bootstrap_applies_default_tags() {
        let init = Initializer::new(test_config());
        let features = vec![FeatureSpec {
            name: "Tagged".into(),
            priority: 1,
            acceptance_criteria: vec![],
            tags: vec!["custom".into()],
            depends_on: vec![],
        }];

        let memory = init.bootstrap(features).unwrap();
        let tags = &memory.backlog.features[0].tags;
        assert!(tags.contains(&"custom".to_string()));
        assert!(tags.contains(&"test".to_string()));
    }

    #[test]
    fn test_sovereign_infrastructure_template() {
        let memory = Initializer::sovereign_infrastructure();
        assert_eq!(memory.backlog.project_name, "Apophy Sovereign Infrastructure");
        assert_eq!(memory.backlog.total(), 8);
        assert!(memory.rules.stop_on_regression);

        // First actionable should be priority 1 (crypto or hardware)
        let next = memory.backlog.next_actionable().unwrap();
        assert_eq!(next.priority, 1);

        // The orchestrator should depend on everything
        let orchestrator = memory.backlog.features.iter()
            .find(|f| f.name.contains("orchestrator"))
            .unwrap();
        assert!(orchestrator.depends_on.len() >= 5);
    }

    #[test]
    fn test_sovereign_template_serializes() {
        let memory = Initializer::sovereign_infrastructure();
        let json = serde_json::to_string_pretty(&memory).unwrap();
        assert!(json.contains("Apophy Sovereign Infrastructure"));
        assert!(json.contains("sovereign"));
    }
}
