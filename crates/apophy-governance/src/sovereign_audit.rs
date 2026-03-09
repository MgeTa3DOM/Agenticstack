//! # Audit Souverain — Le Rapport Complet
//!
//! "La différence entre Mode Outil et Mode Infrastructure:
//! le Mode Outil espère. Le Mode Infrastructure vérifie."
//!
//! Combine tous les modules de gouvernance en un audit unique:
//! - Surface d'attaque (leçons Clawdbot)
//! - Observabilité (traçabilité LLM)
//! - Vérification (gates)
//! - Permissions (enveloppes)
//! - Taxonomie des défaillances
//! - Évaluation (scorecards)
//!
//! Produit un rapport d'Infrastructure Mode complet avec score de maturité.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::attack_surface::{AttackSurfaceAnalyzer, SurfaceAnalysis, SystemProfile};
use crate::taxonomy::{FailureTaxonomy, TaxonomyReport};
use crate::eval::{EvalHarness, EvalResult};

/// Niveau de maturité Infrastructure Mode.
/// Arbre de compétences à 4 niveaux.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MaturityLevel {
    /// Niveau 0: Mode Outil — pas de gouvernance
    ToolMode,
    /// Niveau 1: Conditionnement — intent spec, contraintes, contexte
    Conditioning,
    /// Niveau 2: Autorité — vérification, provenance, permissions
    Authority,
    /// Niveau 3: Workflows — pipelines, taxonomie, observabilité
    Workflows,
    /// Niveau 4: Compounding — eval, feedback loops, drift governance
    Compounding,
}

impl MaturityLevel {
    pub fn label_fr(&self) -> &'static str {
        match self {
            Self::ToolMode => "Mode Outil (Niveau 0)",
            Self::Conditioning => "Conditionnement (Niveau 1)",
            Self::Authority => "Autorité (Niveau 2)",
            Self::Workflows => "Workflows (Niveau 3)",
            Self::Compounding => "Compounding (Niveau 4)",
        }
    }

    pub fn description_fr(&self) -> &'static str {
        match self {
            Self::ToolMode => "Envoyer prompt → recevoir sortie → espérer → réessayer si erreur",
            Self::Conditioning => "Spécification d'intention, ingénierie de contexte, contraintes",
            Self::Authority => "Vérification, provenance, enveloppes de permission",
            Self::Workflows => "Décomposition en pipelines, taxonomie des défaillances, observabilité",
            Self::Compounding => "Évaluation continue, boucles de feedback, gouvernance de drift",
        }
    }

    pub fn from_score(score: f64) -> Self {
        match score {
            s if s >= 0.9 => Self::Compounding,
            s if s >= 0.7 => Self::Workflows,
            s if s >= 0.5 => Self::Authority,
            s if s >= 0.3 => Self::Conditioning,
            _ => Self::ToolMode,
        }
    }
}

/// Rapport d'audit souverain complet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereignAuditReport {
    pub id: Uuid,
    pub generated_at: DateTime<Utc>,
    /// Score de maturité global (0.0 à 1.0)
    pub maturity_score: f64,
    /// Niveau de maturité atteint
    pub maturity_level: MaturityLevel,
    /// Analyse de surface d'attaque
    pub surface_analysis: SurfaceAnalysis,
    /// Rapport de taxonomie des défaillances
    pub taxonomy_report: Option<TaxonomyReport>,
    /// Dernier résultat d'évaluation
    pub eval_result: Option<EvalResult>,
    /// Métriques d'observabilité
    pub observability: ObservabilityMetrics,
    /// Recommandations
    pub recommendations: Vec<Recommendation>,
    /// Comparaison Clawdbot vs Apophy
    pub clawdbot_comparison: ClawdbotComparison,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityMetrics {
    /// Nombre total de traces LLM
    pub total_traces: usize,
    /// Taux de gaspillage de tokens
    pub waste_rate: f64,
    /// Budget consommé (pourcentage)
    pub budget_consumed_pct: f64,
    /// Coût total en dollars
    pub total_cost_dollars: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub priority: RecommendationPriority,
    pub category: String,
    pub message_fr: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Critique,
    Élevé,
    Moyen,
    Bas,
}

/// Comparaison directe avec les vulnérabilités Clawdbot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClawdbotComparison {
    pub apophy_score: f64,
    pub clawdbot_score: f64,
    pub improvement_factor: f64,
    pub vulnerabilities_addressed: usize,
    pub vulnerabilities_remaining: usize,
}

/// Constructeur d'audit souverain.
pub struct SovereignAuditor {
    analyzer: AttackSurfaceAnalyzer,
}

impl SovereignAuditor {
    pub fn new() -> Self {
        Self {
            analyzer: AttackSurfaceAnalyzer::new(),
        }
    }

    /// Exécuter un audit complet.
    pub fn audit(
        &self,
        profile: &SystemProfile,
        taxonomy: Option<&FailureTaxonomy>,
        eval_harness: Option<&EvalHarness>,
        obs_metrics: ObservabilityMetrics,
    ) -> SovereignAuditReport {
        // 1. Analyse de surface d'attaque
        let surface_analysis = self.analyzer.analyze(profile);

        // 2. Rapport de taxonomie
        let taxonomy_report = taxonomy.map(|t| t.report());

        // 3. Dernier résultat d'évaluation
        let eval_result = eval_harness.and_then(|h| h.latest().cloned());

        // 4. Comparaison Clawdbot
        let clawdbot_analysis = self.analyzer.analyze(&AttackSurfaceAnalyzer::clawdbot_profile());
        let vulnerabilities_remaining = surface_analysis.findings.iter()
            .filter(|f| !f.passed).count();

        let clawdbot_comparison = ClawdbotComparison {
            apophy_score: surface_analysis.score,
            clawdbot_score: clawdbot_analysis.score,
            improvement_factor: if clawdbot_analysis.score > 0.0 {
                surface_analysis.score / clawdbot_analysis.score
            } else if surface_analysis.score > 0.0 {
                f64::INFINITY
            } else {
                1.0
            },
            vulnerabilities_addressed: surface_analysis.findings.iter()
                .filter(|f| f.passed).count(),
            vulnerabilities_remaining,
        };

        // 5. Score de maturité composite
        let surface_weight = 0.3;
        let taxonomy_weight = 0.2;
        let eval_weight = 0.3;
        let obs_weight = 0.2;

        let taxonomy_score = taxonomy_report.as_ref()
            .map(|r| r.overall_catch_rate)
            .unwrap_or(0.0);

        let eval_score = eval_result.as_ref()
            .map(|r| r.avg_score)
            .unwrap_or(0.0);

        let obs_score = 1.0 - obs_metrics.waste_rate.min(1.0);

        let maturity_score = (surface_analysis.score * surface_weight)
            + (taxonomy_score * taxonomy_weight)
            + (eval_score * eval_weight)
            + (obs_score * obs_weight);

        let maturity_level = MaturityLevel::from_score(maturity_score);

        // 6. Recommandations
        let mut recommendations = Vec::new();
        self.generate_recommendations(&surface_analysis, &taxonomy_report,
            &eval_result, &obs_metrics, &mut recommendations);

        SovereignAuditReport {
            id: Uuid::new_v4(),
            generated_at: Utc::now(),
            maturity_score,
            maturity_level,
            surface_analysis,
            taxonomy_report,
            eval_result,
            observability: obs_metrics,
            recommendations,
            clawdbot_comparison,
        }
    }

    fn generate_recommendations(
        &self,
        surface: &SurfaceAnalysis,
        taxonomy: &Option<TaxonomyReport>,
        eval: &Option<EvalResult>,
        obs: &ObservabilityMetrics,
        recs: &mut Vec<Recommendation>,
    ) {
        // Surface d'attaque
        if !surface.hardened {
            let failed: Vec<_> = surface.findings.iter()
                .filter(|f| !f.passed)
                .collect();
            for finding in failed {
                recs.push(Recommendation {
                    priority: RecommendationPriority::Critique,
                    category: finding.class.label_fr().to_string(),
                    message_fr: finding.detail.clone(),
                });
            }
        }

        // Taxonomie
        if let Some(report) = taxonomy {
            if report.overall_catch_rate < 0.8 {
                recs.push(Recommendation {
                    priority: RecommendationPriority::Élevé,
                    category: "Taxonomie".into(),
                    message_fr: format!(
                        "Taux de détection à {:.0}% — objectif 80%+. \
                        Ajouter des VerificationGates pour les modes de défaillance non capturés.",
                        report.overall_catch_rate * 100.0
                    ),
                });
            }
            if report.critical_count > 0 {
                recs.push(Recommendation {
                    priority: RecommendationPriority::Critique,
                    category: "Taxonomie".into(),
                    message_fr: format!(
                        "{} défaillances critiques détectées — investigation immédiate requise.",
                        report.critical_count
                    ),
                });
            }
        }

        // Évaluation
        if let Some(result) = eval {
            if result.regression_failures > 0 {
                recs.push(Recommendation {
                    priority: RecommendationPriority::Critique,
                    category: "Évaluation".into(),
                    message_fr: format!(
                        "{} régressions détectées — bloquer le déploiement.",
                        result.regression_failures
                    ),
                });
            }
            if !result.deployable {
                recs.push(Recommendation {
                    priority: RecommendationPriority::Élevé,
                    category: "Évaluation".into(),
                    message_fr: "Système non déployable — score d'évaluation insuffisant.".into(),
                });
            }
        }

        // Observabilité
        if obs.waste_rate > 0.3 {
            recs.push(Recommendation {
                priority: RecommendationPriority::Moyen,
                category: "Observabilité".into(),
                message_fr: format!(
                    "Gaspillage de tokens à {:.0}% — réduire à <30% via contraintes.",
                    obs.waste_rate * 100.0
                ),
            });
        }
    }
}

impl Default for SovereignAuditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_obs() -> ObservabilityMetrics {
        ObservabilityMetrics {
            total_traces: 100,
            waste_rate: 0.15,
            budget_consumed_pct: 0.45,
            total_cost_dollars: 12.50,
        }
    }

    #[test]
    fn test_maturity_levels() {
        assert_eq!(MaturityLevel::from_score(0.95), MaturityLevel::Compounding);
        assert_eq!(MaturityLevel::from_score(0.75), MaturityLevel::Workflows);
        assert_eq!(MaturityLevel::from_score(0.55), MaturityLevel::Authority);
        assert_eq!(MaturityLevel::from_score(0.35), MaturityLevel::Conditioning);
        assert_eq!(MaturityLevel::from_score(0.1), MaturityLevel::ToolMode);
    }

    #[test]
    fn test_maturity_labels_fr() {
        let level = MaturityLevel::Compounding;
        assert!(level.label_fr().contains("Niveau 4"));
        assert!(!level.description_fr().is_empty());
    }

    #[test]
    fn test_sovereign_audit_hardened() {
        let auditor = SovereignAuditor::new();
        let profile = AttackSurfaceAnalyzer::sovereign_profile();
        let obs = default_obs();

        let report = auditor.audit(&profile, None, None, obs);
        assert!(report.surface_analysis.hardened);
        assert!(report.maturity_score > 0.0);
    }

    #[test]
    fn test_clawdbot_audit_vulnerable() {
        let auditor = SovereignAuditor::new();
        let profile = AttackSurfaceAnalyzer::clawdbot_profile();
        let obs = ObservabilityMetrics {
            total_traces: 0,
            waste_rate: 0.6,
            budget_consumed_pct: 0.0,
            total_cost_dollars: 0.0,
        };

        let report = auditor.audit(&profile, None, None, obs);
        assert!(!report.surface_analysis.hardened);
        assert_eq!(report.maturity_level, MaturityLevel::ToolMode);
        assert!(!report.recommendations.is_empty());
    }

    #[test]
    fn test_clawdbot_comparison() {
        let auditor = SovereignAuditor::new();
        let profile = AttackSurfaceAnalyzer::sovereign_profile();
        let obs = default_obs();

        let report = auditor.audit(&profile, None, None, obs);
        assert!(report.clawdbot_comparison.apophy_score > report.clawdbot_comparison.clawdbot_score);
        assert!(report.clawdbot_comparison.improvement_factor > 5.0);
    }

    #[test]
    fn test_recommendations_for_vulnerable_system() {
        let auditor = SovereignAuditor::new();
        let profile = AttackSurfaceAnalyzer::clawdbot_profile();
        let obs = ObservabilityMetrics {
            total_traces: 50,
            waste_rate: 0.5,
            budget_consumed_pct: 0.8,
            total_cost_dollars: 100.0,
        };

        let report = auditor.audit(&profile, None, None, obs);

        let critical_recs: Vec<_> = report.recommendations.iter()
            .filter(|r| r.priority == RecommendationPriority::Critique)
            .collect();
        assert!(!critical_recs.is_empty(), "Système vulnérable devrait avoir des recommandations critiques");
    }

    #[test]
    fn test_audit_with_taxonomy() {
        use crate::taxonomy::{Diagnosis, FailureMode, FailureTaxonomy};

        let mut taxonomy = FailureTaxonomy::new();
        taxonomy.record(
            Diagnosis::new(FailureMode::Hallucination, "Test hallucination")
                .caught()
        );
        taxonomy.record(
            Diagnosis::new(FailureMode::Drift, "Test drift")
        );

        let auditor = SovereignAuditor::new();
        let profile = AttackSurfaceAnalyzer::sovereign_profile();
        let obs = default_obs();

        let report = auditor.audit(&profile, Some(&taxonomy), None, obs);
        assert!(report.taxonomy_report.is_some());
        let tax_report = report.taxonomy_report.unwrap();
        assert_eq!(tax_report.total_failures, 2);
    }

    #[test]
    fn test_audit_serialization() {
        let auditor = SovereignAuditor::new();
        let profile = AttackSurfaceAnalyzer::sovereign_profile();
        let obs = default_obs();

        let report = auditor.audit(&profile, None, None, obs);
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("maturity_score"));
        assert!(json.contains("clawdbot_comparison"));
        assert!(json.contains("surface_analysis"));
    }

    #[test]
    fn test_waste_recommendation() {
        let auditor = SovereignAuditor::new();
        let profile = AttackSurfaceAnalyzer::sovereign_profile();
        let obs = ObservabilityMetrics {
            total_traces: 100,
            waste_rate: 0.45, // > 0.3 threshold
            budget_consumed_pct: 0.8,
            total_cost_dollars: 50.0,
        };

        let report = auditor.audit(&profile, None, None, obs);
        let waste_recs: Vec<_> = report.recommendations.iter()
            .filter(|r| r.category == "Observabilité")
            .collect();
        assert!(!waste_recs.is_empty(), "Gaspillage élevé devrait générer une recommandation");
    }

    #[test]
    fn test_full_audit_with_eval() {
        use crate::eval::{EvalHarness, GoldenExample, Scorecard};

        let mut harness = EvalHarness::new();
        harness.add_example(GoldenExample::new("Test", "in", "out"));
        harness.run_eval("model", |ex| {
            let mut c = Scorecard::new(ex, "actual");
            c.score("accuracy", 0.95);
            c
        });

        let auditor = SovereignAuditor::new();
        let profile = AttackSurfaceAnalyzer::sovereign_profile();
        let obs = default_obs();

        let report = auditor.audit(&profile, None, Some(&harness), obs);
        assert!(report.eval_result.is_some());
    }
}
