//! # Sovereign Infrastructure Integration
//!
//! Self-financing ecosystem connecting the sovereign stack to:
//! - **Skool**: Community monetization (avatarvers conference)
//! - **AvatarVers**: Virtual world / conference portal (avatarvers.com)
//! - **iAgenticFlow**: Agentic workflow portal (iagenticflow.org)
//! - **Infomaniak**: Swiss domain hosting (DNS, email)
//! - **Email Agent**: Automated SMTP notifications via Infomaniak
//! - **Gitea**: Self-hosted open-source git (git.iagenticflow.org)
//! - **Cloudflare Tunnels**: Zero-trust exposure without open ports
//!
//! All services are open-source compatible and auditable.
//! No vendor lock-in. The sovereign stack stays sovereign.

use crate::config::{InfraConfig, TunnelRoute};
use serde::{Deserialize, Serialize};

// === Service Types ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceType {
    Skool,
    AvatarVers,
    AgenticFlow,
    Infomaniak,
    EmailAgent,
    Gitea,
    CloudflareTunnel,
}

impl ServiceType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Skool => "Skool",
            Self::AvatarVers => "AvatarVers",
            Self::AgenticFlow => "iAgenticFlow",
            Self::Infomaniak => "Infomaniak",
            Self::EmailAgent => "Email Agent",
            Self::Gitea => "Gitea",
            Self::CloudflareTunnel => "Cloudflare Tunnel",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Up,
    Down,
    Degraded,
    Unknown,
    Disabled,
}

impl HealthStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Up => "UP",
            Self::Down => "DOWN",
            Self::Degraded => "WARN",
            Self::Unknown => "????",
            Self::Disabled => "SKIP",
        }
    }
}

// === Service Health ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub name: String,
    pub service_type: ServiceType,
    pub base_url: String,
    pub enabled: bool,
    pub status: HealthStatus,
    pub latency_ms: Option<u64>,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfraStatus {
    pub services: Vec<ServiceHealth>,
    pub total: usize,
    pub healthy: usize,
    pub degraded: usize,
    pub down: usize,
    pub disabled: usize,
    pub tunnel_routes: Vec<TunnelRoute>,
    pub total_webhooks: u64,
}

// === Skool Webhook ===

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SkoolWebhookPayload {
    pub event_type: SkoolEvent,
    pub member_email: Option<String>,
    pub member_name: Option<String>,
    pub amount_cents: Option<u64>,
    pub currency: Option<String>,
    pub course_name: Option<String>,
    pub community: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SkoolEvent {
    MemberJoined,
    CoursePurchased,
    PostCreated,
    MemberLeft,
}

impl std::fmt::Display for SkoolEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MemberJoined => write!(f, "member_joined"),
            Self::CoursePurchased => write!(f, "course_purchased"),
            Self::PostCreated => write!(f, "post_created"),
            Self::MemberLeft => write!(f, "member_left"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SkoolWebhookResponse {
    pub received: bool,
    pub webhook_id: String,
    pub event: String,
    pub contribution_recorded: bool,
    pub tokens_minted: f64,
}

// === Email Agent ===

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMessage {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// Generate a sovereign onboarding email body
#[allow(dead_code)]
pub fn onboarding_email(name: &str, community: &str) -> EmailMessage {
    EmailMessage {
        to: String::new(), // filled by caller
        subject: format!("Bienvenue dans {} - Infrastructure Souveraine", community),
        body: format!(
            "Bonjour {},\n\n\
             Bienvenue dans la communaute {} !\n\n\
             Votre acces a l'infrastructure souveraine est actif:\n\
             - Portail: https://iagenticflow.org\n\
             - Git: https://git.iagenticflow.org\n\
             - Conference AvatarVers: https://avatarvers.com\n\
             - API: https://api.iagenticflow.org\n\n\
             Aucune donnee ne transite par le cloud. Tout reste souverain.\n\n\
             -- Apophy Sovereign Agent\n",
            name, community
        ),
    }
}

/// Generate a contribution receipt email body
#[allow(dead_code)]
pub fn contribution_receipt_email(name: &str, tokens: f64, course: &str) -> EmailMessage {
    EmailMessage {
        to: String::new(),
        subject: format!("Contribution enregistree - {:.2} tokens", tokens),
        body: format!(
            "Bonjour {},\n\n\
             Votre contribution a ete enregistree dans le Commons Ledger:\n\
             - Cours: {}\n\
             - Tokens credites: {:.2}\n\
             - Type: Knowledge\n\n\
             Ces tokens sont gagnes, jamais empruntes. Pas de dette.\n\
             Consultez votre solde: https://iagenticflow.org/commons\n\n\
             -- Apophy Sovereign Agent\n",
            name, course, tokens
        ),
    }
}

// === Health Check Logic ===

pub async fn check_service_health(
    client: &reqwest::Client,
    name: &str,
    service_type: ServiceType,
    base_url: &str,
    enabled: bool,
) -> ServiceHealth {
    if !enabled {
        return ServiceHealth {
            name: name.to_string(),
            service_type,
            base_url: base_url.to_string(),
            enabled: false,
            status: HealthStatus::Disabled,
            latency_ms: None,
            details: None,
        };
    }

    let start = std::time::Instant::now();
    let result = client
        .get(base_url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;

    match result {
        Ok(resp) => {
            let latency = start.elapsed().as_millis() as u64;
            let status_code = resp.status();
            let status = if status_code.is_success() || status_code.is_redirection() {
                HealthStatus::Up
            } else if status_code.is_server_error() {
                HealthStatus::Degraded
            } else {
                HealthStatus::Up // 4xx from root is normal for APIs
            };

            ServiceHealth {
                name: name.to_string(),
                service_type,
                base_url: base_url.to_string(),
                enabled: true,
                status,
                latency_ms: Some(latency),
                details: Some(format!("HTTP {}", status_code.as_u16())),
            }
        }
        Err(e) => ServiceHealth {
            name: name.to_string(),
            service_type,
            base_url: base_url.to_string(),
            enabled: true,
            status: HealthStatus::Down,
            latency_ms: None,
            details: Some(format!("{}", e)),
        },
    }
}

pub async fn check_all_services(
    config: &InfraConfig,
    client: &reqwest::Client,
) -> Vec<ServiceHealth> {
    let (skool, avatarvers, agenticflow, infomaniak, gitea) = tokio::join!(
        check_service_health(
            client,
            "Skool",
            ServiceType::Skool,
            &config.skool.base_url,
            config.skool.enabled,
        ),
        check_service_health(
            client,
            "AvatarVers",
            ServiceType::AvatarVers,
            &config.avatarvers.base_url,
            config.avatarvers.enabled,
        ),
        check_service_health(
            client,
            "iAgenticFlow",
            ServiceType::AgenticFlow,
            &config.agenticflow.base_url,
            config.agenticflow.enabled,
        ),
        check_service_health(
            client,
            "Infomaniak",
            ServiceType::Infomaniak,
            &config.infomaniak.base_url,
            config.infomaniak.enabled,
        ),
        check_service_health(
            client,
            "Gitea",
            ServiceType::Gitea,
            &config.gitea.base_url,
            config.gitea.enabled,
        ),
    );

    // Email and Cloudflare are local configs, not HTTP-checkable the same way
    let email_health = ServiceHealth {
        name: "Email Agent".to_string(),
        service_type: ServiceType::EmailAgent,
        base_url: format!("{}:{}", config.email.smtp_host, config.email.smtp_port),
        enabled: config.email.enabled,
        status: if config.email.enabled {
            HealthStatus::Unknown
        } else {
            HealthStatus::Disabled
        },
        latency_ms: None,
        details: Some(format!("SMTP {} (from: {})", config.email.smtp_host, config.email.from_address)),
    };

    let tunnel_health = ServiceHealth {
        name: "Cloudflare Tunnel".to_string(),
        service_type: ServiceType::CloudflareTunnel,
        base_url: format!("tunnel:{}", config.cloudflare.tunnel_name),
        enabled: config.cloudflare.enabled,
        status: if config.cloudflare.enabled {
            HealthStatus::Unknown
        } else {
            HealthStatus::Disabled
        },
        latency_ms: None,
        details: Some(format!(
            "{} routes configured",
            config.cloudflare.routes.len()
        )),
    };

    vec![
        skool,
        avatarvers,
        agenticflow,
        infomaniak,
        email_health,
        gitea,
        tunnel_health,
    ]
}

pub fn build_infra_status(
    services: Vec<ServiceHealth>,
    tunnel_routes: &[TunnelRoute],
    webhook_count: u64,
) -> InfraStatus {
    let total = services.len();
    let healthy = services.iter().filter(|s| s.status == HealthStatus::Up).count();
    let degraded = services.iter().filter(|s| s.status == HealthStatus::Degraded).count();
    let down = services.iter().filter(|s| s.status == HealthStatus::Down).count();
    let disabled = services.iter().filter(|s| s.status == HealthStatus::Disabled).count();

    InfraStatus {
        services,
        total,
        healthy,
        degraded,
        down,
        disabled,
        tunnel_routes: tunnel_routes.to_vec(),
        total_webhooks: webhook_count,
    }
}

/// Process a Skool webhook: calculate tokens for knowledge contribution
pub fn process_skool_contribution(payload: &SkoolWebhookPayload) -> (bool, f64) {
    match payload.event_type {
        SkoolEvent::CoursePurchased => {
            // Knowledge contribution: fixed 10 tokens (same as apophy-commons)
            (true, 10.0)
        }
        SkoolEvent::MemberJoined => {
            // Community contribution: 5 tokens for joining
            (true, 5.0)
        }
        _ => (false, 0.0),
    }
}
