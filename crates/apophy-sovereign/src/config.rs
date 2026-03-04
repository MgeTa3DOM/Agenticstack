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
