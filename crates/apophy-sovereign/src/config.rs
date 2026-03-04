use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereignConfig {
    pub server: ServerConfig,
    pub hardware: HardwareConfig,
    pub chat: ChatConfig,
    pub ai: AiConfig,
    pub security: SecurityConfig,
    pub database: DatabaseConfig,
    #[serde(default)]
    pub infra: InfraConfig,
    #[serde(default)]
    pub fleet: FleetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareConfig {
    pub auto_detect: bool,
    pub force_backend: Option<String>,
    pub max_memory_percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    pub enable_e2e: bool,
    pub max_group_size: usize,
    pub auto_destroy_days: Option<u32>,
    pub max_message_size_kb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub model_path: PathBuf,
    pub model_name: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub context_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub encryption: String,
    pub key_rotation_days: u32,
    pub audit_log: bool,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

// === Infrastructure Integration Config ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfraConfig {
    pub enabled: bool,
    pub skool: SkoolConfig,
    pub avatarvers: ServiceEndpoint,
    pub agenticflow: ServiceEndpoint,
    pub infomaniak: InfomaniakConfig,
    pub email: EmailConfig,
    pub gitea: GiteaConfig,
    pub cloudflare: CloudflareConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub enabled: bool,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkoolConfig {
    pub enabled: bool,
    pub base_url: String,
    pub community_slug: String,
    pub webhook_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfomaniakConfig {
    pub enabled: bool,
    pub base_url: String,
    pub api_key_env: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub from_address: String,
    pub from_name: String,
    pub use_tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaConfig {
    pub enabled: bool,
    pub base_url: String,
    pub ssh_port: u16,
    pub domain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareConfig {
    pub enabled: bool,
    pub tunnel_name: String,
    pub tunnel_token_env: Option<String>,
    pub routes: Vec<TunnelRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelRoute {
    pub hostname: String,
    pub service: String,
}

// === Agent Fleet Config ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetConfig {
    pub enabled: bool,
    pub synarchy_mode: String,
    pub max_agents: usize,
    pub auto_spawn: bool,
    pub domains_enabled: Vec<String>,
}

impl Default for FleetConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            synarchy_mode: "divine".to_string(),
            max_agents: 3000,
            auto_spawn: true,
            domains_enabled: vec![
                "startups".into(), "tech".into(), "support".into(),
                "sales".into(), "hr".into(), "marketing".into(),
                "ecommerce".into(), "pm".into(), "legal".into(),
            ],
        }
    }
}

impl Default for InfraConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            skool: SkoolConfig {
                enabled: true,
                base_url: "https://www.skool.com".to_string(),
                community_slug: "avatarvers".to_string(),
                webhook_secret: None,
            },
            avatarvers: ServiceEndpoint {
                enabled: true,
                base_url: "https://avatarvers.com".to_string(),
            },
            agenticflow: ServiceEndpoint {
                enabled: true,
                base_url: "https://iagenticflow.org".to_string(),
            },
            infomaniak: InfomaniakConfig {
                enabled: true,
                base_url: "https://api.infomaniak.com".to_string(),
                api_key_env: Some("INFOMANIAK_API_KEY".to_string()),
            },
            email: EmailConfig {
                enabled: true,
                smtp_host: "mail.infomaniak.com".to_string(),
                smtp_port: 587,
                from_address: "agent@iagenticflow.org".to_string(),
                from_name: "Apophy Sovereign".to_string(),
                use_tls: true,
            },
            gitea: GiteaConfig {
                enabled: true,
                base_url: "http://localhost:3000".to_string(),
                ssh_port: 2222,
                domain: "git.iagenticflow.org".to_string(),
            },
            cloudflare: CloudflareConfig {
                enabled: true,
                tunnel_name: "apophy-sovereign".to_string(),
                tunnel_token_env: Some("CF_TUNNEL_TOKEN".to_string()),
                routes: vec![
                    TunnelRoute {
                        hostname: "api.iagenticflow.org".to_string(),
                        service: "http://localhost:8080".to_string(),
                    },
                    TunnelRoute {
                        hostname: "iagenticflow.org".to_string(),
                        service: "http://localhost:8080".to_string(),
                    },
                    TunnelRoute {
                        hostname: "git.iagenticflow.org".to_string(),
                        service: "http://localhost:3000".to_string(),
                    },
                    TunnelRoute {
                        hostname: "avatarvers.com".to_string(),
                        service: "http://localhost:8080".to_string(),
                    },
                ],
            },
        }
    }
}

impl Default for SovereignConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
            },
            hardware: HardwareConfig {
                auto_detect: true,
                force_backend: None,
                max_memory_percent: 90,
            },
            chat: ChatConfig {
                enable_e2e: true,
                max_group_size: 10_000,
                auto_destroy_days: Some(30),
                max_message_size_kb: 1024,
            },
            ai: AiConfig {
                model_path: PathBuf::from("models/default.gguf"),
                model_name: "qwen-3.5-coder".to_string(),
                max_tokens: 4096,
                temperature: 0.7,
                context_length: 4096,
            },
            security: SecurityConfig {
                encryption: "chacha20poly1305".to_string(),
                key_rotation_days: 90,
                audit_log: true,
                tls_cert: None,
                tls_key: None,
            },
            database: DatabaseConfig {
                path: PathBuf::from("data/sovereign.db"),
            },
            infra: InfraConfig::default(),
            fleet: FleetConfig::default(),
        }
    }
}

impl SovereignConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    #[allow(dead_code)]
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }
}
