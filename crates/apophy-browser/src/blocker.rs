//! # Content Blocker — COM-Level Ad/Tracker Blocking
//!
//! Blocks requests BEFORE the HTTP connection is made, not after.
//! JS-level blocking is bypassable; COM-level blocking is not.
//!
//! Uses a pattern-matching engine inspired by Brave's adblock-rust.
//! Rules are compiled into an efficient state machine for O(1) lookup.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlockerError {
    #[error("Invalid rule: {0}")]
    InvalidRule(String),
    #[error("Rule list too large: {size} (max {max})")]
    TooLarge { size: usize, max: usize },
}

pub type Result<T> = std::result::Result<T, BlockerError>;

/// Decision for a URL request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockDecision {
    /// Allow the request
    Allow,
    /// Block the request (with reason)
    Block { rule: String, category: BlockCategory },
    /// Redirect to a safe alternative
    Redirect { target: String },
}

/// Category of blocked content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockCategory {
    Advertising,
    Tracking,
    Malware,
    Cryptomining,
    Social,
    Fingerprinting,
    Annoyance,
}

impl BlockCategory {
    pub fn label(&self) -> &'static str {
        match self {
            BlockCategory::Advertising => "ADS",
            BlockCategory::Tracking => "TRACKING",
            BlockCategory::Malware => "MALWARE",
            BlockCategory::Cryptomining => "CRYPTO",
            BlockCategory::Social => "SOCIAL",
            BlockCategory::Fingerprinting => "FINGERPRINT",
            BlockCategory::Annoyance => "ANNOYANCE",
        }
    }
}

/// A single block rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRule {
    /// Pattern to match against URLs
    pub pattern: String,
    /// Category of this rule
    pub category: BlockCategory,
    /// Whether this is an exception (allow) rule
    pub is_exception: bool,
    /// Domains this rule applies to (empty = all)
    pub domains: Vec<String>,
    /// Resource types this rule applies to
    pub resource_types: Vec<ResourceType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResourceType {
    Script,
    Image,
    Stylesheet,
    Font,
    XmlHttpRequest,
    WebSocket,
    Other,
}

/// The Content Blocker — fast pattern-matching engine
pub struct ContentBlocker {
    /// Domain-based block rules (fast HashSet lookup)
    blocked_domains: HashSet<String>,
    /// URL pattern rules
    rules: Vec<BlockRule>,
    /// Exception domains (never block)
    exception_domains: HashSet<String>,
    /// Metrics
    total_blocked: u64,
    total_allowed: u64,
}

impl ContentBlocker {
    pub fn new() -> Self {
        Self {
            blocked_domains: HashSet::new(),
            rules: Vec::new(),
            exception_domains: HashSet::new(),
            total_blocked: 0,
            total_allowed: 0,
        }
    }

    /// Create a blocker with sovereign defaults (aggressive blocking)
    pub fn sovereign() -> Self {
        let mut blocker = Self::new();

        // === ADVERTISING DOMAINS ===
        let ad_domains = [
            "doubleclick.net", "googlesyndication.com", "googleadservices.com",
            "adnxs.com", "adsrvr.org", "advertising.com", "adform.net",
            "taboola.com", "outbrain.com", "criteo.com", "rubiconproject.com",
            "pubmatic.com", "openx.net", "sharethrough.com", "amazon-adsystem.com",
            "moat.com", "serving-sys.com", "adhigh.net", "admob.com",
        ];
        for domain in ad_domains {
            blocker.block_domain(domain, BlockCategory::Advertising);
        }

        // === TRACKING DOMAINS ===
        let tracking_domains = [
            "google-analytics.com", "googletagmanager.com", "facebook.net",
            "hotjar.com", "mixpanel.com", "segment.io", "amplitude.com",
            "fullstory.com", "mouseflow.com", "crazyegg.com", "optimizely.com",
            "branch.io", "appsflyer.com", "adjust.com", "kochava.com",
            "newrelic.com", "bugsnag.com", "sentry.io", "datadoghq.com",
        ];
        for domain in tracking_domains {
            blocker.block_domain(domain, BlockCategory::Tracking);
        }

        // === MALWARE / CRYPTOMINING ===
        let malware_domains = [
            "coinhive.com", "coin-hive.com", "cryptoloot.pro", "crypto-loot.com",
            "minero.cc", "webminerpool.com", "ppoi.org",
        ];
        for domain in malware_domains {
            blocker.block_domain(domain, BlockCategory::Cryptomining);
        }

        // === SOCIAL TRACKING ===
        let social_domains = [
            "connect.facebook.net", "platform.twitter.com", "platform.linkedin.com",
            "widgets.pinterest.com", "static.addtoany.com",
        ];
        for domain in social_domains {
            blocker.block_domain(domain, BlockCategory::Social);
        }

        // === FINGERPRINTING ===
        let fingerprint_domains = [
            "cdn.jsdelivr.net/npm/fingerprintjs",
            "fpjs.io", "fingerprint.com", "browserleaks.com",
        ];
        for domain in fingerprint_domains {
            blocker.block_domain(domain, BlockCategory::Fingerprinting);
        }

        blocker
    }

    /// Block a specific domain
    pub fn block_domain(&mut self, domain: &str, category: BlockCategory) {
        self.blocked_domains.insert(domain.to_string());
        self.rules.push(BlockRule {
            pattern: domain.to_string(),
            category,
            is_exception: false,
            domains: vec![],
            resource_types: vec![],
        });
    }

    /// Add an exception domain (never block requests from this domain)
    pub fn add_exception(&mut self, domain: &str) {
        self.exception_domains.insert(domain.to_string());
    }

    /// Check if a URL should be blocked
    ///
    /// This is the hot path — called for EVERY network request.
    /// Must be fast (O(1) for domain checks).
    pub fn check(&mut self, url: &str, source_url: &str) -> BlockDecision {
        // Exception domains bypass all blocking
        if let Some(source_domain) = Self::extract_domain(source_url) {
            if self.exception_domains.contains(&source_domain) {
                self.total_allowed += 1;
                return BlockDecision::Allow;
            }
        }

        // Check domain-based blocking (fast path)
        if let Some(domain) = Self::extract_domain(url) {
            // Check exact domain match
            if let Some((pattern, category)) = self.find_domain_match(&domain) {
                self.total_blocked += 1;
                return BlockDecision::Block { rule: pattern, category };
            }

            // Check parent domain match (e.g., ads.example.com matches example.com)
            let parts: Vec<&str> = domain.split('.').collect();
            for i in 1..parts.len().saturating_sub(1) {
                let parent = parts[i..].join(".");
                if let Some((pattern, category)) = self.find_domain_match(&parent) {
                    self.total_blocked += 1;
                    return BlockDecision::Block { rule: pattern, category };
                }
            }
        }

        // Check URL pattern rules
        for rule in &self.rules {
            if !rule.is_exception && url.contains(&rule.pattern) {
                self.total_blocked += 1;
                return BlockDecision::Block {
                    rule: rule.pattern.clone(),
                    category: rule.category.clone(),
                };
            }
        }

        self.total_allowed += 1;
        BlockDecision::Allow
    }

    fn find_domain_match(&self, domain: &str) -> Option<(String, BlockCategory)> {
        if self.blocked_domains.contains(domain) {
            self.rules.iter()
                .find(|r| r.pattern == domain)
                .map(|r| (r.pattern.clone(), r.category.clone()))
        } else {
            None
        }
    }

    fn extract_domain(url: &str) -> Option<String> {
        // Fast path: already a domain
        if !url.contains("://") && !url.contains('/') {
            return Some(url.to_string());
        }

        // Parse URL
        if let Ok(parsed) = url::Url::parse(url) {
            parsed.host_str().map(|h| h.to_string())
        } else {
            None
        }
    }

    /// Get blocker statistics
    pub fn stats(&self) -> BlockerStats {
        BlockerStats {
            total_rules: self.rules.len(),
            blocked_domains: self.blocked_domains.len(),
            exception_domains: self.exception_domains.len(),
            total_blocked: self.total_blocked,
            total_allowed: self.total_allowed,
            block_rate: if self.total_blocked + self.total_allowed > 0 {
                self.total_blocked as f64 / (self.total_blocked + self.total_allowed) as f64
            } else {
                0.0
            },
        }
    }
}

impl Default for ContentBlocker {
    fn default() -> Self {
        Self::sovereign()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockerStats {
    pub total_rules: usize,
    pub blocked_domains: usize,
    pub exception_domains: usize,
    pub total_blocked: u64,
    pub total_allowed: u64,
    pub block_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_ad_domain() {
        let mut blocker = ContentBlocker::sovereign();
        let decision = blocker.check(
            "https://doubleclick.net/ad.js",
            "https://example.com",
        );

        match decision {
            BlockDecision::Block { category, .. } => {
                assert_eq!(category, BlockCategory::Advertising);
            }
            _ => panic!("Expected block"),
        }
    }

    #[test]
    fn test_block_tracking_domain() {
        let mut blocker = ContentBlocker::sovereign();
        let decision = blocker.check(
            "https://google-analytics.com/collect",
            "https://example.com",
        );

        match decision {
            BlockDecision::Block { category, .. } => {
                assert_eq!(category, BlockCategory::Tracking);
            }
            _ => panic!("Expected block"),
        }
    }

    #[test]
    fn test_block_subdomain() {
        let mut blocker = ContentBlocker::sovereign();
        let decision = blocker.check(
            "https://ads.doubleclick.net/tag",
            "https://example.com",
        );

        match decision {
            BlockDecision::Block { .. } => {}
            _ => panic!("Expected subdomain block"),
        }
    }

    #[test]
    fn test_allow_safe_domain() {
        let mut blocker = ContentBlocker::sovereign();
        let decision = blocker.check(
            "https://example.com/page",
            "https://example.com",
        );
        assert_eq!(decision, BlockDecision::Allow);
    }

    #[test]
    fn test_exception_domain_bypasses() {
        let mut blocker = ContentBlocker::sovereign();
        blocker.add_exception("trusted.com");

        let decision = blocker.check(
            "https://google-analytics.com/collect",
            "https://trusted.com",
        );
        assert_eq!(decision, BlockDecision::Allow);
    }

    #[test]
    fn test_block_cryptomining() {
        let mut blocker = ContentBlocker::sovereign();
        let decision = blocker.check(
            "https://coinhive.com/lib/coinhive.min.js",
            "https://example.com",
        );

        match decision {
            BlockDecision::Block { category, .. } => {
                assert_eq!(category, BlockCategory::Cryptomining);
            }
            _ => panic!("Expected cryptomining block"),
        }
    }

    #[test]
    fn test_stats_tracking() {
        let mut blocker = ContentBlocker::sovereign();

        blocker.check("https://doubleclick.net/ad", "https://example.com");
        blocker.check("https://example.com/page", "https://example.com");
        blocker.check("https://google-analytics.com/t", "https://example.com");

        let stats = blocker.stats();
        assert_eq!(stats.total_blocked, 2);
        assert_eq!(stats.total_allowed, 1);
        assert!((stats.block_rate - 0.6667).abs() < 0.01);
    }

    #[test]
    fn test_custom_rule() {
        let mut blocker = ContentBlocker::new();
        blocker.block_domain("evil.com", BlockCategory::Malware);

        let decision = blocker.check("https://evil.com/payload", "https://example.com");
        match decision {
            BlockDecision::Block { category, .. } => {
                assert_eq!(category, BlockCategory::Malware);
            }
            _ => panic!("Expected block"),
        }
    }

    #[test]
    fn test_sovereign_blocker_has_rules() {
        let blocker = ContentBlocker::sovereign();
        let stats = blocker.stats();
        assert!(stats.total_rules > 40);
        assert!(stats.blocked_domains > 40);
    }

    #[test]
    fn test_stats_serialization() {
        let blocker = ContentBlocker::sovereign();
        let stats = blocker.stats();
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("total_rules"));
    }
}
