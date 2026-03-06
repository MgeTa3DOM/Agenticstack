//! # Surface d'Attaque — Leçons du Collapse Clawdbot
//!
//! "Clawdbot a prouvé que capable = dangereux.
//! Apophy prouve que capable + souverain = invulnérable."
//!
//! Trois classes de vulnérabilités identifiées dans l'incident Clawdbot/Moltbot:
//!
//! 1. **Contournement d'Authentification** — confiance aveugle en localhost
//! 2. **Injection de Prompt** — contenu externe qui détourne l'agent
//! 3. **Chaîne d'Approvisionnement** — plugins non modérés avec permissions complètes
//!
//! Ce module encode ces classes comme des vérifications automatiques
//! qui s'exécutent AVANT chaque action d'agent.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Les 3 classes d'attaques identifiées par l'incident Clawdbot.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackClass {
    /// Classe 1: Contournement d'authentification
    /// Clawdbot: proxy inverse traité comme localhost → accès sans auth
    /// Prévention: vérification explicite d'origine sur chaque connexion
    AuthBypass,
    /// Classe 2: Injection de prompt via contenu externe
    /// Clawdbot: email malveillant extrait clé SSH en 5 minutes
    /// Prévention: analyse de contenu, isolation des instructions
    PromptInjection,
    /// Classe 3: Chaîne d'approvisionnement non modérée
    /// Clawdbot: plugin bénin → 4000 téléchargements gonflés → vecteur d'attaque
    /// Prévention: signatures, audits, révocation de composants tiers
    SupplyChain,
}

impl AttackClass {
    pub fn label_fr(&self) -> &'static str {
        match self {
            Self::AuthBypass => "Contournement d'Authentification",
            Self::PromptInjection => "Injection de Prompt",
            Self::SupplyChain => "Chaîne d'Approvisionnement",
        }
    }

    pub fn severity(&self) -> &'static str {
        match self {
            Self::AuthBypass => "CRITIQUE",
            Self::PromptInjection => "CRITIQUE — structurellement incorrigible sans isolation",
            Self::SupplyChain => "ÉLEVÉ",
        }
    }

    pub fn clawdbot_lesson(&self) -> &'static str {
        match self {
            Self::AuthBypass => "4500+ instances exposées via balayage internet basique. \
                La confiance aveugle en localhost est un anti-pattern mortel.",
            Self::PromptInjection => "Clé SSH privée exfiltrée en 5 minutes via un seul email. \
                Les LLM ne distinguent pas instructions du contenu.",
            Self::SupplyChain => "Plugin à 4000 téléchargements gonflés installé par des devs de 7 pays. \
                Zero modération = zero sécurité.",
        }
    }

    pub fn all() -> Vec<AttackClass> {
        vec![Self::AuthBypass, Self::PromptInjection, Self::SupplyChain]
    }
}

/// Résultat d'une analyse de surface d'attaque.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceAnalysis {
    pub id: Uuid,
    pub analyzed_at: DateTime<Utc>,
    pub findings: Vec<SurfaceFinding>,
    pub score: f64, // 0.0 (vulnérable) à 1.0 (durci)
    pub hardened: bool,
}

/// Un finding dans l'analyse de surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceFinding {
    pub class: AttackClass,
    pub check_name: String,
    pub passed: bool,
    pub detail: String,
}

/// Analyseur de surface d'attaque souverain.
pub struct AttackSurfaceAnalyzer {
    checks: Vec<SurfaceCheck>,
}

struct SurfaceCheck {
    class: AttackClass,
    name: String,
    checker: Box<dyn Fn(&SystemProfile) -> (bool, String)>,
}

/// Profil système à analyser.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemProfile {
    /// L'agent écoute-t-il sur localhost uniquement?
    pub binds_localhost_only: bool,
    /// Auth explicite requise sur chaque connexion?
    pub requires_explicit_auth: bool,
    /// Derrière un proxy inverse?
    pub behind_reverse_proxy: bool,
    /// Si proxy, valide-t-il l'origine?
    pub proxy_validates_origin: bool,
    /// Les identifiants sont-ils chiffrés au repos?
    pub credentials_encrypted: bool,
    /// Rotation automatique des identifiants?
    pub credential_rotation_enabled: bool,
    /// Isolation des instructions du contenu externe?
    pub instruction_content_isolation: bool,
    /// Analyse de contenu entrant avant traitement?
    pub content_scanning_enabled: bool,
    /// Liste blanche d'actions autorisées?
    pub action_allowlist_enabled: bool,
    /// Sandbox pour exécution de commandes?
    pub command_sandbox_enabled: bool,
    /// Plugins tiers signés?
    pub plugins_signed: bool,
    /// Mécanisme de révocation de plugins?
    pub plugin_revocation_enabled: bool,
    /// Audit de dépendances tiers?
    pub dependency_audit_enabled: bool,
    /// Nombre de plugins tiers installés
    pub third_party_plugin_count: usize,
    /// Permissions des plugins limitées?
    pub plugin_permissions_scoped: bool,
    /// MemoryVault activé (chiffrement RAM)?
    pub memory_vault_enabled: bool,
    /// Hash chain immuable pour l'historique?
    pub immutable_history_enabled: bool,
    /// Enveloppes de permission actives?
    pub permission_envelopes_active: bool,
}

impl AttackSurfaceAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = Self { checks: Vec::new() };
        analyzer.register_auth_checks();
        analyzer.register_injection_checks();
        analyzer.register_supply_chain_checks();
        analyzer
    }

    fn register_auth_checks(&mut self) {
        self.checks.push(SurfaceCheck {
            class: AttackClass::AuthBypass,
            name: "Authentification explicite requise".into(),
            checker: Box::new(|p| {
                if p.requires_explicit_auth {
                    (true, "Auth explicite active sur chaque connexion".into())
                } else {
                    (false, "DANGER: pas d'auth explicite — vulnérable au bypass Clawdbot".into())
                }
            }),
        });

        self.checks.push(SurfaceCheck {
            class: AttackClass::AuthBypass,
            name: "Proxy inverse vérifie l'origine".into(),
            checker: Box::new(|p| {
                if !p.behind_reverse_proxy {
                    (true, "Pas de proxy inverse — pas de risque de confusion localhost".into())
                } else if p.proxy_validates_origin {
                    (true, "Proxy valide l'origine explicitement".into())
                } else {
                    (false, "CRITIQUE: proxy inverse sans validation d'origine — \
                        exact vecteur d'attaque Clawdbot".into())
                }
            }),
        });

        self.checks.push(SurfaceCheck {
            class: AttackClass::AuthBypass,
            name: "Identifiants chiffrés au repos".into(),
            checker: Box::new(|p| {
                if p.credentials_encrypted {
                    (true, "Identifiants protégés par MemoryVault".into())
                } else {
                    (false, "Identifiants en plaintext — exfiltration triviale".into())
                }
            }),
        });

        self.checks.push(SurfaceCheck {
            class: AttackClass::AuthBypass,
            name: "Permission envelopes actives".into(),
            checker: Box::new(|p| {
                if p.permission_envelopes_active {
                    (true, "Enveloppes de permission zero-trust actives".into())
                } else {
                    (false, "Pas d'enveloppes — agent sans limites".into())
                }
            }),
        });
    }

    fn register_injection_checks(&mut self) {
        self.checks.push(SurfaceCheck {
            class: AttackClass::PromptInjection,
            name: "Isolation instructions/contenu".into(),
            checker: Box::new(|p| {
                if p.instruction_content_isolation {
                    (true, "Instructions isolées du contenu externe".into())
                } else {
                    (false, "CRITIQUE: contenu externe mélangé aux instructions — \
                        vulnérable à l'attaque email Clawdbot".into())
                }
            }),
        });

        self.checks.push(SurfaceCheck {
            class: AttackClass::PromptInjection,
            name: "Analyse de contenu entrant".into(),
            checker: Box::new(|p| {
                if p.content_scanning_enabled {
                    (true, "Contenu scanné avant traitement par l'agent".into())
                } else {
                    (false, "Pas de scan — injection via email/message possible".into())
                }
            }),
        });

        self.checks.push(SurfaceCheck {
            class: AttackClass::PromptInjection,
            name: "Liste blanche d'actions".into(),
            checker: Box::new(|p| {
                if p.action_allowlist_enabled {
                    (true, "Seules les actions autorisées sont exécutables".into())
                } else {
                    (false, "Toute action possible — injection peut tout faire".into())
                }
            }),
        });

        self.checks.push(SurfaceCheck {
            class: AttackClass::PromptInjection,
            name: "Sandbox d'exécution de commandes".into(),
            checker: Box::new(|p| {
                if p.command_sandbox_enabled {
                    (true, "Commandes exécutées dans un sandbox isolé".into())
                } else {
                    (false, "Exécution directe — injection = accès système complet".into())
                }
            }),
        });
    }

    fn register_supply_chain_checks(&mut self) {
        self.checks.push(SurfaceCheck {
            class: AttackClass::SupplyChain,
            name: "Plugins signés".into(),
            checker: Box::new(|p| {
                if p.third_party_plugin_count == 0 {
                    (true, "Aucun plugin tiers installé".into())
                } else if p.plugins_signed {
                    (true, format!("{} plugins tiers — tous signés", p.third_party_plugin_count))
                } else {
                    (false, format!("{} plugins non signés — vecteur supply chain Clawdbot",
                        p.third_party_plugin_count))
                }
            }),
        });

        self.checks.push(SurfaceCheck {
            class: AttackClass::SupplyChain,
            name: "Révocation de plugins".into(),
            checker: Box::new(|p| {
                if p.third_party_plugin_count == 0 {
                    (true, "N/A — aucun plugin".into())
                } else if p.plugin_revocation_enabled {
                    (true, "Mécanisme de révocation actif".into())
                } else {
                    (false, "Pas de révocation — mise à jour malveillante irréversible".into())
                }
            }),
        });

        self.checks.push(SurfaceCheck {
            class: AttackClass::SupplyChain,
            name: "Permissions des plugins scopées".into(),
            checker: Box::new(|p| {
                if p.third_party_plugin_count == 0 {
                    (true, "N/A — aucun plugin".into())
                } else if p.plugin_permissions_scoped {
                    (true, "Chaque plugin a des permissions limitées".into())
                } else {
                    (false, "Plugins avec permissions complètes — \
                        exact problème Claude Hub de Clawdbot".into())
                }
            }),
        });

        self.checks.push(SurfaceCheck {
            class: AttackClass::SupplyChain,
            name: "Audit de dépendances".into(),
            checker: Box::new(|p| {
                if p.dependency_audit_enabled {
                    (true, "Dépendances auditées automatiquement".into())
                } else {
                    (false, "Pas d'audit — vulnérabilités silencieuses possibles".into())
                }
            }),
        });
    }

    /// Analyser un profil système.
    pub fn analyze(&self, profile: &SystemProfile) -> SurfaceAnalysis {
        let mut findings = Vec::new();

        for check in &self.checks {
            let (passed, detail) = (check.checker)(profile);
            findings.push(SurfaceFinding {
                class: check.class.clone(),
                check_name: check.name.clone(),
                passed,
                detail,
            });
        }

        let passed = findings.iter().filter(|f| f.passed).count();
        let total = findings.len();
        let score = if total == 0 { 0.0 } else { passed as f64 / total as f64 };

        SurfaceAnalysis {
            id: Uuid::new_v4(),
            analyzed_at: Utc::now(),
            findings,
            score,
            hardened: score >= 0.9,
        }
    }

    /// Profil Apophy souverain — toutes protections actives.
    pub fn sovereign_profile() -> SystemProfile {
        SystemProfile {
            binds_localhost_only: true,
            requires_explicit_auth: true,
            behind_reverse_proxy: false,
            proxy_validates_origin: false,
            credentials_encrypted: true,
            credential_rotation_enabled: true,
            instruction_content_isolation: true,
            content_scanning_enabled: true,
            action_allowlist_enabled: true,
            command_sandbox_enabled: true,
            plugins_signed: true,
            plugin_revocation_enabled: true,
            dependency_audit_enabled: true,
            third_party_plugin_count: 0,
            plugin_permissions_scoped: true,
            memory_vault_enabled: true,
            immutable_history_enabled: true,
            permission_envelopes_active: true,
        }
    }

    /// Profil Clawdbot — reproduit les vulnérabilités exactes.
    pub fn clawdbot_profile() -> SystemProfile {
        SystemProfile {
            binds_localhost_only: false,
            requires_explicit_auth: false,
            behind_reverse_proxy: true,
            proxy_validates_origin: false,
            credentials_encrypted: false,
            credential_rotation_enabled: false,
            instruction_content_isolation: false,
            content_scanning_enabled: false,
            action_allowlist_enabled: false,
            command_sandbox_enabled: false,
            plugins_signed: false,
            plugin_revocation_enabled: false,
            dependency_audit_enabled: false,
            third_party_plugin_count: 12,
            plugin_permissions_scoped: false,
            memory_vault_enabled: false,
            immutable_history_enabled: false,
            permission_envelopes_active: false,
        }
    }
}

impl Default for AttackSurfaceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attack_classes() {
        let classes = AttackClass::all();
        assert_eq!(classes.len(), 3);
        assert_eq!(classes[0].label_fr(), "Contournement d'Authentification");
    }

    #[test]
    fn test_clawdbot_lessons() {
        for class in AttackClass::all() {
            let lesson = class.clawdbot_lesson();
            assert!(!lesson.is_empty());
            assert!(class.severity().contains("CRITIQUE") || class.severity().contains("ÉLEVÉ"));
        }
    }

    #[test]
    fn test_sovereign_profile_fully_hardened() {
        let analyzer = AttackSurfaceAnalyzer::new();
        let profile = AttackSurfaceAnalyzer::sovereign_profile();
        let analysis = analyzer.analyze(&profile);

        assert_eq!(analysis.score, 1.0);
        assert!(analysis.hardened);
        assert!(analysis.findings.iter().all(|f| f.passed));
    }

    #[test]
    fn test_clawdbot_profile_fully_vulnerable() {
        let analyzer = AttackSurfaceAnalyzer::new();
        let profile = AttackSurfaceAnalyzer::clawdbot_profile();
        let analysis = analyzer.analyze(&profile);

        assert!(analysis.score < 0.1);
        assert!(!analysis.hardened);

        // Toutes les 3 classes doivent avoir des échecs
        let auth_fails: Vec<_> = analysis.findings.iter()
            .filter(|f| f.class == AttackClass::AuthBypass && !f.passed)
            .collect();
        assert!(!auth_fails.is_empty(), "Auth bypass devrait échouer");

        let injection_fails: Vec<_> = analysis.findings.iter()
            .filter(|f| f.class == AttackClass::PromptInjection && !f.passed)
            .collect();
        assert!(!injection_fails.is_empty(), "Injection devrait échouer");

        let supply_fails: Vec<_> = analysis.findings.iter()
            .filter(|f| f.class == AttackClass::SupplyChain && !f.passed)
            .collect();
        assert!(!supply_fails.is_empty(), "Supply chain devrait échouer");
    }

    #[test]
    fn test_partial_hardening() {
        let profile = SystemProfile {
            requires_explicit_auth: true,
            credentials_encrypted: true,
            permission_envelopes_active: true,
            // Everything else default (false/0)
            ..Default::default()
        };

        let analyzer = AttackSurfaceAnalyzer::new();
        let analysis = analyzer.analyze(&profile);

        assert!(analysis.score > 0.0);
        assert!(analysis.score < 1.0);
        assert!(!analysis.hardened);
    }

    #[test]
    fn test_no_plugins_safe() {
        let profile = SystemProfile {
            third_party_plugin_count: 0,
            ..Default::default()
        };

        let analyzer = AttackSurfaceAnalyzer::new();
        let analysis = analyzer.analyze(&profile);

        // Supply chain checks should all pass when no plugins
        let supply_results: Vec<_> = analysis.findings.iter()
            .filter(|f| f.class == AttackClass::SupplyChain)
            .collect();
        let supply_passed = supply_results.iter().filter(|f| f.passed).count();
        // 3 out of 4 supply chain checks pass (audit still fails)
        assert!(supply_passed >= 3);
    }

    #[test]
    fn test_proxy_without_validation_is_critical() {
        let profile = SystemProfile {
            behind_reverse_proxy: true,
            proxy_validates_origin: false,
            ..Default::default()
        };

        let analyzer = AttackSurfaceAnalyzer::new();
        let analysis = analyzer.analyze(&profile);

        let proxy_check = analysis.findings.iter()
            .find(|f| f.check_name.contains("Proxy"))
            .unwrap();
        assert!(!proxy_check.passed);
        assert!(proxy_check.detail.contains("Clawdbot"));
    }

    #[test]
    fn test_proxy_with_validation_passes() {
        let profile = SystemProfile {
            behind_reverse_proxy: true,
            proxy_validates_origin: true,
            ..Default::default()
        };

        let analyzer = AttackSurfaceAnalyzer::new();
        let analysis = analyzer.analyze(&profile);

        let proxy_check = analysis.findings.iter()
            .find(|f| f.check_name.contains("Proxy"))
            .unwrap();
        assert!(proxy_check.passed);
    }

    #[test]
    fn test_analysis_serialization() {
        let analyzer = AttackSurfaceAnalyzer::new();
        let analysis = analyzer.analyze(&AttackSurfaceAnalyzer::sovereign_profile());
        let json = serde_json::to_string(&analysis).unwrap();
        assert!(json.contains("hardened"));
        assert!(json.contains("findings"));
        assert!(json.contains("score"));
    }

    #[test]
    fn test_finding_counts() {
        let analyzer = AttackSurfaceAnalyzer::new();
        let analysis = analyzer.analyze(&SystemProfile::default());

        // 4 auth + 4 injection + 4 supply chain = 12 checks
        assert_eq!(analysis.findings.len(), 12);
    }

    #[test]
    fn test_clawdbot_vs_apophy_score_gap() {
        let analyzer = AttackSurfaceAnalyzer::new();

        let clawdbot = analyzer.analyze(&AttackSurfaceAnalyzer::clawdbot_profile());
        let apophy = analyzer.analyze(&AttackSurfaceAnalyzer::sovereign_profile());

        // Le gap doit être massif
        assert!(apophy.score - clawdbot.score > 0.8,
            "Apophy ({}) devrait écraser Clawdbot ({}) en sécurité",
            apophy.score, clawdbot.score);
    }
}
