//! # Tab State — The Atom of the Hash Chain
//!
//! Each tab state is a block. Each block hashes the previous.
//! Alter one state and the entire chain breaks.
//! Simple. Inalterable. Like Bitcoin.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// A single tab state — one block in the personal blockchain.
///
/// The hash commits to: timestamp + tab_id + url + content_hash + previous_hash.
/// Change ANY field and the hash changes. The chain breaks. Tampering detected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabState {
    /// Which tab this state belongs to
    pub tab_id: Uuid,
    /// When this state was captured
    pub timestamp: DateTime<Utc>,
    /// The URL at this moment
    pub url: String,
    /// SHA-256 of the DOM/content (not the content itself — just the proof)
    pub content_hash: [u8; 32],
    /// Scroll position at capture
    pub scroll_y: u32,
    /// Title at this moment
    pub title: String,
    /// Hash of the previous state (genesis = all zeros)
    pub prev_hash: [u8; 32],
    /// Hash of THIS state (commits to all fields above)
    pub hash: [u8; 32],
}

impl TabState {
    /// Create a new state, chaining from the previous hash.
    pub fn new(
        tab_id: Uuid,
        url: impl Into<String>,
        content: &[u8],
        title: impl Into<String>,
        scroll_y: u32,
        prev_hash: [u8; 32],
    ) -> Self {
        let url = url.into();
        let title = title.into();
        let timestamp = Utc::now();
        let content_hash = Self::hash_content(content);

        let hash = Self::compute_hash(
            tab_id,
            &timestamp,
            &url,
            &content_hash,
            scroll_y,
            &title,
            &prev_hash,
        );

        Self {
            tab_id,
            timestamp,
            url,
            content_hash,
            scroll_y,
            title,
            prev_hash,
            hash,
        }
    }

    /// Genesis state — the first block. prev_hash = all zeros.
    pub fn genesis(tab_id: Uuid) -> Self {
        Self::new(tab_id, "about:blank", b"", "Genesis", 0, [0u8; 32])
    }

    /// Verify this state's hash is correct (recompute and compare).
    pub fn verify(&self) -> bool {
        let expected = Self::compute_hash(
            self.tab_id,
            &self.timestamp,
            &self.url,
            &self.content_hash,
            self.scroll_y,
            &self.title,
            &self.prev_hash,
        );
        self.hash == expected
    }

    /// Verify a chain of states: each state's prev_hash matches the previous state's hash.
    pub fn verify_chain(chain: &[TabState]) -> ChainVerification {
        if chain.is_empty() {
            return ChainVerification {
                valid: true,
                length: 0,
                break_at: None,
                reason: None,
            };
        }

        // Verify each state's self-hash
        for (i, state) in chain.iter().enumerate() {
            if !state.verify() {
                return ChainVerification {
                    valid: false,
                    length: chain.len(),
                    break_at: Some(i),
                    reason: Some(format!("State {} hash mismatch (tampered)", i)),
                };
            }
        }

        // Verify chain linkage
        for (i, window) in chain.windows(2).enumerate() {
            if window[1].prev_hash != window[0].hash {
                return ChainVerification {
                    valid: false,
                    length: chain.len(),
                    break_at: Some(i + 1),
                    reason: Some(format!(
                        "Chain broken at state {}: prev_hash doesn't match state {} hash",
                        i + 1,
                        i
                    )),
                };
            }
        }

        ChainVerification {
            valid: true,
            length: chain.len(),
            break_at: None,
            reason: None,
        }
    }

    /// Short hash display (first 8 hex chars)
    pub fn short_hash(&self) -> String {
        hex::encode(&self.hash[..4])
    }

    /// Short prev_hash display
    pub fn short_prev_hash(&self) -> String {
        hex::encode(&self.prev_hash[..4])
    }

    fn hash_content(content: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(content);
        hasher.finalize().into()
    }

    fn compute_hash(
        tab_id: Uuid,
        timestamp: &DateTime<Utc>,
        url: &str,
        content_hash: &[u8; 32],
        scroll_y: u32,
        title: &str,
        prev_hash: &[u8; 32],
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(tab_id.as_bytes());
        hasher.update(timestamp.to_rfc3339().as_bytes());
        hasher.update(url.as_bytes());
        hasher.update(content_hash);
        hasher.update(scroll_y.to_le_bytes());
        hasher.update(title.as_bytes());
        hasher.update(prev_hash);
        hasher.finalize().into()
    }
}

/// Simple hex encoding (no external dep needed)
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

/// Result of chain verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerification {
    /// Whether the chain is valid
    pub valid: bool,
    /// Total states in chain
    pub length: usize,
    /// Index where the chain breaks (if invalid)
    pub break_at: Option<usize>,
    /// Human-readable reason for failure
    pub reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tab() -> Uuid {
        Uuid::new_v4()
    }

    #[test]
    fn test_genesis() {
        let tab_id = test_tab();
        let genesis = TabState::genesis(tab_id);

        assert_eq!(genesis.prev_hash, [0u8; 32]);
        assert_eq!(genesis.url, "about:blank");
        assert_eq!(genesis.scroll_y, 0);
        assert!(genesis.verify());
    }

    #[test]
    fn test_chain_two_states() {
        let tab_id = test_tab();
        let s0 = TabState::genesis(tab_id);
        let s1 = TabState::new(tab_id, "https://example.com", b"<html>hello</html>", "Example", 0, s0.hash);

        assert!(s0.verify());
        assert!(s1.verify());
        assert_eq!(s1.prev_hash, s0.hash);
    }

    #[test]
    fn test_verify_chain_valid() {
        let tab_id = test_tab();
        let s0 = TabState::genesis(tab_id);
        let s1 = TabState::new(tab_id, "https://a.com", b"a", "A", 0, s0.hash);
        let s2 = TabState::new(tab_id, "https://b.com", b"b", "B", 100, s1.hash);
        let s3 = TabState::new(tab_id, "https://c.com", b"c", "C", 200, s2.hash);

        let result = TabState::verify_chain(&[s0, s1, s2, s3]);
        assert!(result.valid);
        assert_eq!(result.length, 4);
        assert!(result.break_at.is_none());
    }

    #[test]
    fn test_verify_chain_detects_tampering() {
        let tab_id = test_tab();
        let s0 = TabState::genesis(tab_id);
        let s1 = TabState::new(tab_id, "https://a.com", b"a", "A", 0, s0.hash);
        let mut s2 = TabState::new(tab_id, "https://b.com", b"b", "B", 0, s1.hash);

        // Tamper: change URL without recomputing hash
        s2.url = "https://TAMPERED.com".to_string();

        let result = TabState::verify_chain(&[s0, s1, s2]);
        assert!(!result.valid);
        assert_eq!(result.break_at, Some(2));
        assert!(result.reason.unwrap().contains("tampered"));
    }

    #[test]
    fn test_verify_chain_detects_broken_link() {
        let tab_id = test_tab();
        let s0 = TabState::genesis(tab_id);
        let _s1 = TabState::new(tab_id, "https://a.com", b"a", "A", 0, s0.hash);
        // s2 chains from s0 instead of s1 — broken link
        let s2 = TabState::new(tab_id, "https://b.com", b"b", "B", 0, s0.hash);

        let result = TabState::verify_chain(&[s0, _s1, s2]);
        assert!(!result.valid);
        assert_eq!(result.break_at, Some(2));
        assert!(result.reason.unwrap().contains("Chain broken"));
    }

    #[test]
    fn test_empty_chain_valid() {
        let result = TabState::verify_chain(&[]);
        assert!(result.valid);
        assert_eq!(result.length, 0);
    }

    #[test]
    fn test_single_state_chain() {
        let genesis = TabState::genesis(test_tab());
        let result = TabState::verify_chain(&[genesis]);
        assert!(result.valid);
        assert_eq!(result.length, 1);
    }

    #[test]
    fn test_different_content_different_hash() {
        let tab_id = test_tab();
        let s0 = TabState::genesis(tab_id);
        let s1a = TabState::new(tab_id, "https://x.com", b"content-a", "X", 0, s0.hash);
        let s1b = TabState::new(tab_id, "https://x.com", b"content-b", "X", 0, s0.hash);

        // Same URL, different content → different hash
        assert_ne!(s1a.hash, s1b.hash);
    }

    #[test]
    fn test_short_hash() {
        let genesis = TabState::genesis(test_tab());
        let short = genesis.short_hash();
        assert_eq!(short.len(), 8); // 4 bytes = 8 hex chars
    }

    #[test]
    fn test_serialization_roundtrip() {
        let tab_id = test_tab();
        let s0 = TabState::genesis(tab_id);
        let s1 = TabState::new(tab_id, "https://example.com", b"data", "Test", 42, s0.hash);

        let json = serde_json::to_string(&s1).unwrap();
        let restored: TabState = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.hash, s1.hash);
        assert_eq!(restored.url, s1.url);
        assert!(restored.verify());
    }
}
