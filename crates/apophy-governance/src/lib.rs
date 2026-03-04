//! # Apophy Governance — Infrastructure Mode Souverain
//!
//! "90% des entreprises perdent ce pari. Pas nous."
//!
//! Implémente les 4 niveaux de l'arbre de compétences Infrastructure Mode:
//!
//! - **Niveau 1 Conditionnement** (spéc. d'intention, ingénierie de contexte, contraintes)
//! - **Niveau 2 Autorité** (vérification, provenance, enveloppes de permission)
//! - **Niveau 3 Workflows** (décomposition en pipelines, taxonomie, observabilité)
//! - **Niveau 4 Compounding** (évaluation, boucles de feedback, gouvernance de drift)
//!
//! ## Leçons du Collapse Clawdbot
//!
//! Le module `attack_surface` encode les 3 classes de vulnérabilités qui ont détruit
//! Clawdbot en 72 heures: bypass auth, injection de prompt, supply chain non modéré.
//! L'audit souverain compare automatiquement Apophy vs Clawdbot sur chaque vecteur.
//!
//! ## Mode Outil vs Mode Infrastructure
//!
//! Mode Outil: envoyer prompt → recevoir sortie → espérer → réessayer si erreur.
//! Mode Infrastructure: design flow → vérifier sortie → maintenir autorité → composer les gains.
//!
//! La différence: 30-50% moins de gaspillage, exponentiellement moins de risque, 10x levier.

pub mod observe;
pub mod verify;
pub mod permit;
pub mod taxonomy;
pub mod eval;
pub mod attack_surface;
pub mod sovereign_audit;

pub use observe::{LlmTrace, TraceLog, TraceSummary};
pub use verify::{Verification, VerificationGate, VerificationResult};
pub use permit::{Permission, PermissionEnvelope, PermitDecision};
pub use taxonomy::{FailureMode, FailureTaxonomy, Diagnosis};
pub use eval::{EvalHarness, EvalResult, GoldenExample, Scorecard};
pub use attack_surface::{AttackClass, AttackSurfaceAnalyzer, SurfaceAnalysis, SystemProfile};
pub use sovereign_audit::{SovereignAuditor, SovereignAuditReport, MaturityLevel};
