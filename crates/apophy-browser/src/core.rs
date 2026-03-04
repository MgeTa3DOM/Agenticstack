//! # Browser Core — The Orchestrator
//!
//! Ties together all browser components:
//! - MemoryVault (crypto)
//! - TabManager (lifecycle)
//! - ContentBlocker (ads/trackers)
//! - FingerprintShield (mitigations)
//! - LocalSync (zero-cloud sync)
//!
//! This is what Tauri's `main.rs` instantiates and exposes via IPC commands.

use crate::blocker::{BlockDecision, ContentBlocker};
use crate::shield::FingerprintShield;
use crate::sync::LocalSync;
use crate::tabs::{TabManager, TabManagerConfig, TabManagerSummary};
use crate::vault::{MemoryVault, VaultMetrics};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Configuration for the entire browser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    /// Device name for sync
    pub device_name: String,
    /// Tab manager configuration
    pub tab_config: TabManagerConfig,
    /// Enable content blocker
    pub blocker_enabled: bool,
    /// Enable fingerprint shield
    pub shield_enabled: bool,
    /// Shield mode: "full" or "safe"
    pub shield_mode: String,
    /// Enable local sync
    pub sync_enabled: bool,
    /// HTTPS-only mode (block all HTTP)
    pub https_only: bool,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            device_name: "Apophy Sovereign Browser".into(),
            tab_config: TabManagerConfig::default(),
            blocker_enabled: true,
            shield_enabled: true,
            shield_mode: "full".into(),
            sync_enabled: true,
            https_only: true,
        }
    }
}

/// The Browser Core — everything in one place
pub struct BrowserCore {
    pub vault: MemoryVault,
    pub tabs: TabManager,
    pub blocker: ContentBlocker,
    pub shield: FingerprintShield,
    pub sync: LocalSync,
    config: BrowserConfig,
}

impl BrowserCore {
    /// Create a new browser instance with a random vault key
    pub fn new(config: BrowserConfig) -> Self {
        let shield = if config.shield_mode == "safe" {
            FingerprintShield::safe_mode()
        } else {
            FingerprintShield::full_protection()
        };

        Self {
            vault: MemoryVault::generate(),
            tabs: TabManager::new(config.tab_config.clone()),
            blocker: ContentBlocker::sovereign(),
            shield,
            sync: LocalSync::new(&config.device_name),
            config,
        }
    }

    /// Create a browser with a specific vault key (for persistence)
    pub fn with_key(config: BrowserConfig, key: [u8; 32]) -> Self {
        let shield = if config.shield_mode == "safe" {
            FingerprintShield::safe_mode()
        } else {
            FingerprintShield::full_protection()
        };

        Self {
            vault: MemoryVault::from_key(key),
            tabs: TabManager::new(config.tab_config.clone()),
            blocker: ContentBlocker::sovereign(),
            shield,
            sync: LocalSync::new(&config.device_name),
            config,
        }
    }

    // === Tab Operations (IPC commands for Tauri) ===

    /// Create a new tab
    pub fn create_tab(&mut self, url: &str) -> Result<Uuid, String> {
        // HTTPS-only enforcement
        if self.config.https_only && url.starts_with("http://") {
            let https_url = url.replacen("http://", "https://", 1);
            return self.tabs.create_tab(https_url).map_err(|e| e.to_string());
        }
        self.tabs.create_tab(url).map_err(|e| e.to_string())
    }

    /// Close a tab
    pub fn close_tab(&mut self, id: Uuid) -> Result<(), String> {
        self.tabs.close_tab(id).map(|_| ()).map_err(|e| e.to_string())
    }

    /// Suspend a tab (encrypt DOM to disk)
    pub fn suspend_tab(&mut self, id: Uuid, dom: &[u8], scroll: &[u8]) -> Result<(), String> {
        self.tabs.suspend_tab(id, &mut self.vault, dom, scroll).map_err(|e| e.to_string())
    }

    /// Resume a suspended tab
    pub fn resume_tab(&mut self, id: Uuid) -> Result<(Vec<u8>, Vec<u8>), String> {
        self.tabs.resume_tab(id, &mut self.vault).map_err(|e| e.to_string())
    }

    /// Navigate a tab (with content blocking check)
    pub fn navigate(&mut self, id: Uuid, url: &str, source_url: &str) -> Result<NavigateResult, String> {
        // Check content blocker
        if self.config.blocker_enabled {
            let decision = self.blocker.check(url, source_url);
            if let BlockDecision::Block { rule, category } = decision {
                return Ok(NavigateResult::Blocked {
                    rule,
                    category: category.label().to_string(),
                });
            }
        }

        // HTTPS-only enforcement
        let final_url = if self.config.https_only && url.starts_with("http://") {
            url.replacen("http://", "https://", 1)
        } else {
            url.to_string()
        };

        self.tabs.navigate(id, &final_url).map_err(|e| e.to_string())?;

        Ok(NavigateResult::Allowed { url: final_url })
    }

    // === Content Blocking ===

    /// Check if a URL should be blocked
    pub fn should_block(&mut self, url: &str, source_url: &str) -> BlockDecision {
        if !self.config.blocker_enabled {
            return BlockDecision::Allow;
        }
        self.blocker.check(url, source_url)
    }

    // === Fingerprint Shield ===

    /// Get the JavaScript injection script for WebView
    pub fn shield_script(&self) -> &str {
        if !self.config.shield_enabled {
            return "";
        }
        self.shield.injection_script()
    }

    // === Sync ===

    /// Start pairing with another device
    pub fn start_pairing(&mut self) -> String {
        self.sync.initiate_pairing()
    }

    /// Accept a pairing code
    pub fn accept_pairing(&mut self, code: &str, peer_name: &str) -> Result<Uuid, String> {
        self.sync.accept_pairing(code, peer_name).map_err(|e| e.to_string())
    }

    // === Status ===

    /// Get comprehensive browser status
    pub fn status(&self) -> BrowserStatus {
        BrowserStatus {
            tabs: self.tabs.summary(),
            vault: self.vault.metrics(),
            blocker: self.blocker.stats(),
            shield: self.shield.summary(),
            sync: self.sync.summary(),
            config: self.config.clone(),
        }
    }
}

/// Result of a navigation attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NavigateResult {
    Allowed { url: String },
    Blocked { rule: String, category: String },
}

/// Comprehensive browser status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserStatus {
    pub tabs: TabManagerSummary,
    pub vault: VaultMetrics,
    pub blocker: crate::blocker::BlockerStats,
    pub shield: crate::shield::ShieldSummary,
    pub sync: crate::sync::SyncSummary,
    pub config: BrowserConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_browser() -> BrowserCore {
        BrowserCore::new(BrowserConfig::default())
    }

    #[test]
    fn test_create_browser() {
        let browser = test_browser();
        assert_eq!(browser.tabs.tab_count(), 0);
        assert!(browser.vault.is_unlocked());
        assert_eq!(browser.shield.enabled_count(), 23);
    }

    #[test]
    fn test_create_and_close_tab() {
        let mut browser = test_browser();

        let id = browser.create_tab("https://example.com").unwrap();
        assert_eq!(browser.tabs.tab_count(), 1);

        browser.close_tab(id).unwrap();
        assert_eq!(browser.tabs.tab_count(), 0);
    }

    #[test]
    fn test_https_upgrade() {
        let mut browser = test_browser();

        // HTTP should be upgraded to HTTPS
        let id = browser.create_tab("http://example.com").unwrap();
        let tab = browser.tabs.get_tab(id).unwrap();
        assert_eq!(tab.url, "https://example.com");
    }

    #[test]
    fn test_navigate_with_blocking() {
        let mut browser = test_browser();
        let id = browser.create_tab("https://example.com").unwrap();

        // Normal navigation should work
        let result = browser.navigate(id, "https://example.com/page2", "https://example.com").unwrap();
        assert!(matches!(result, NavigateResult::Allowed { .. }));

        // Ad domain should be blocked
        let result = browser.navigate(id, "https://doubleclick.net/ad.js", "https://example.com").unwrap();
        assert!(matches!(result, NavigateResult::Blocked { .. }));
    }

    #[test]
    fn test_suspend_and_resume_through_core() {
        let mut browser = test_browser();
        let id = browser.create_tab("https://example.com").unwrap();

        let dom = b"<html>test</html>";
        browser.suspend_tab(id, dom, b"100").unwrap();

        let (recovered_dom, recovered_scroll) = browser.resume_tab(id).unwrap();
        assert_eq!(recovered_dom, dom);
        assert_eq!(recovered_scroll, b"100");
    }

    #[test]
    fn test_shield_script_available() {
        let browser = test_browser();
        let script = browser.shield_script();
        assert!(script.contains("Apophy Fingerprint Shield"));
        assert!(script.contains("hardwareConcurrency"));
    }

    #[test]
    fn test_shield_disabled() {
        let mut config = BrowserConfig::default();
        config.shield_enabled = false;
        let browser = BrowserCore::new(config);
        assert_eq!(browser.shield_script(), "");
    }

    #[test]
    fn test_safe_mode_shield() {
        let mut config = BrowserConfig::default();
        config.shield_mode = "safe".into();
        let browser = BrowserCore::new(config);
        assert!(browser.shield.enabled_count() < 23);
    }

    #[test]
    fn test_pairing_through_core() {
        let mut browser = test_browser();
        let code = browser.start_pairing();
        assert_eq!(code.len(), 6);
    }

    #[test]
    fn test_status_comprehensive() {
        let mut browser = test_browser();
        browser.create_tab("https://example.com").unwrap();
        browser.should_block("https://doubleclick.net/ad", "https://example.com");

        let status = browser.status();
        assert_eq!(status.tabs.total, 1);
        assert_eq!(status.shield.total_mitigations, 23);
        assert!(status.config.https_only);
    }

    #[test]
    fn test_status_serialization() {
        let browser = test_browser();
        let status = browser.status();
        let json = serde_json::to_string_pretty(&status).unwrap();
        assert!(json.contains("tabs"));
        assert!(json.contains("vault"));
        assert!(json.contains("shield"));
    }

    #[test]
    fn test_with_key_persistence() {
        let key = [42u8; 32];
        let config = BrowserConfig::default();

        let mut browser1 = BrowserCore::with_key(config.clone(), key);
        let blob = browser1.vault.seal(b"persistent data").unwrap();

        let mut browser2 = BrowserCore::with_key(config, key);
        let recovered = browser2.vault.unseal(&blob).unwrap();
        assert_eq!(recovered, b"persistent data");
    }

    #[test]
    fn test_blocker_disabled() {
        let mut config = BrowserConfig::default();
        config.blocker_enabled = false;
        let mut browser = BrowserCore::new(config);

        let id = browser.create_tab("https://example.com").unwrap();
        let result = browser.navigate(id, "https://doubleclick.net/ad.js", "https://example.com").unwrap();

        // Should be allowed when blocker is disabled
        assert!(matches!(result, NavigateResult::Allowed { .. }));
    }
}
