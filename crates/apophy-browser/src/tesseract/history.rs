//! # Immutable History — The Personal Ledger
//!
//! Like Bitcoin's blockchain: append-only, tamper-proof, checkpoint-verified.
//!
//! - Append: each new state chains from the previous (hash linkage)
//! - Checkpoint: every N states, a Merkle root is computed and stored
//! - Verify: any tampering breaks the chain AND the Merkle root
//! - No delete: history is immutable. Malware can't erase its traces.
//!
//! "Plus de vole, plus de voleur" — the hash is the law.

use crate::tesseract::merkle::MerkleTimeline;
use crate::tesseract::state::{ChainVerification, TabState};
use crate::tesseract::timeline::{Timeline, TimelineSummary};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A Merkle checkpoint — periodic fingerprint of the entire history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Index in the state chain where this checkpoint was taken
    pub at_index: usize,
    /// Merkle root of all states up to this index
    pub merkle_root: [u8; 32],
    /// When this checkpoint was computed
    pub timestamp: DateTime<Utc>,
}

impl Checkpoint {
    pub fn short_root(&self) -> String {
        self.merkle_root.iter().take(4).map(|b| format!("{:02x}", b)).collect()
    }
}

/// Configuration for the immutable history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    /// Take a Merkle checkpoint every N states
    pub checkpoint_interval: usize,
    /// Maximum states to keep in memory (older ones can be pruned but checkpoints remain)
    pub max_states: usize,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            checkpoint_interval: 100,
            max_states: 10_000,
        }
    }
}

/// The Immutable History — append-only personal ledger
pub struct ImmutableHistory {
    /// All timelines (one per tab)
    timelines: HashMap<Uuid, Timeline>,
    /// Global append-only state chain (across all tabs, chronological)
    global_chain: Vec<TabState>,
    /// Merkle checkpoints
    checkpoints: Vec<Checkpoint>,
    /// Configuration
    config: HistoryConfig,
    /// Creation time
    created_at: DateTime<Utc>,
}

impl ImmutableHistory {
    pub fn new(config: HistoryConfig) -> Self {
        Self {
            timelines: HashMap::new(),
            global_chain: Vec::new(),
            checkpoints: Vec::new(),
            config,
            created_at: Utc::now(),
        }
    }

    /// Create a new tab timeline and record genesis in global chain
    pub fn create_tab(&mut self, tab_id: Uuid) -> &Timeline {
        let timeline = Timeline::new(tab_id);

        // Record genesis in global chain
        self.append_to_global(timeline.present().clone());

        self.timelines.insert(tab_id, timeline);
        self.timelines.get(&tab_id).unwrap()
    }

    /// Navigate a tab (creates new state, appends to global chain)
    pub fn navigate(
        &mut self,
        tab_id: Uuid,
        url: impl Into<String>,
        content: &[u8],
        title: impl Into<String>,
        scroll_y: u32,
    ) -> Result<(), HistoryError> {
        let timeline = self.timelines.get_mut(&tab_id).ok_or(HistoryError::TabNotFound(tab_id))?;

        timeline.navigate(url, content, title, scroll_y);

        // Clone present before appending (satisfies borrow checker)
        let new_present = timeline.present().clone();

        // Append new present to global chain
        self.append_to_global(new_present);

        Ok(())
    }

    /// Go back in a tab's timeline
    pub fn back(&mut self, tab_id: Uuid) -> Result<bool, HistoryError> {
        let timeline = self.timelines.get_mut(&tab_id).ok_or(HistoryError::TabNotFound(tab_id))?;
        Ok(timeline.back())
    }

    /// Go forward in a tab's timeline
    pub fn forward(&mut self, tab_id: Uuid) -> Result<bool, HistoryError> {
        let timeline = self.timelines.get_mut(&tab_id).ok_or(HistoryError::TabNotFound(tab_id))?;
        Ok(timeline.forward())
    }

    /// Verify the entire global chain integrity
    pub fn verify_global_chain(&self) -> ChainVerification {
        // Verify each global state's self-hash (detects field tampering)
        for (i, state) in self.global_chain.iter().enumerate() {
            if !state.verify() {
                return ChainVerification {
                    valid: false,
                    length: self.global_chain.len(),
                    break_at: Some(i),
                    reason: Some(format!("Global state {} hash mismatch (tampered)", i)),
                };
            }
        }

        // Also verify each tab's chain independently
        for timeline in self.timelines.values() {
            let result = timeline.verify_integrity();
            if !result.valid {
                return result;
            }
        }

        ChainVerification {
            valid: true,
            length: self.global_chain.len(),
            break_at: None,
            reason: None,
        }
    }

    /// Verify a specific checkpoint: rebuild Merkle tree and compare root
    pub fn verify_checkpoint(&self, checkpoint_index: usize) -> bool {
        if checkpoint_index >= self.checkpoints.len() {
            return false;
        }

        let checkpoint = &self.checkpoints[checkpoint_index];
        let states_up_to = &self.global_chain[..checkpoint.at_index.min(self.global_chain.len())];
        let hashes: Vec<[u8; 32]> = states_up_to.iter().map(|s| s.hash).collect();

        if let Some(tree) = MerkleTimeline::build(&hashes) {
            tree.root_hash() == checkpoint.merkle_root
        } else {
            // Empty tree = valid if checkpoint is at index 0
            checkpoint.at_index == 0
        }
    }

    /// Detect tampering: verify all checkpoints
    pub fn detect_tampering(&self) -> TamperReport {
        let mut broken_checkpoints = Vec::new();

        for (i, _checkpoint) in self.checkpoints.iter().enumerate() {
            if !self.verify_checkpoint(i) {
                broken_checkpoints.push(i);
            }
        }

        let chain_result = self.verify_global_chain();

        TamperReport {
            chain_valid: chain_result.valid,
            chain_break_at: chain_result.break_at,
            total_checkpoints: self.checkpoints.len(),
            broken_checkpoints,
            total_states: self.global_chain.len(),
            total_tabs: self.timelines.len(),
        }
    }

    // === Queries ===

    pub fn get_timeline(&self, tab_id: Uuid) -> Option<&Timeline> {
        self.timelines.get(&tab_id)
    }

    pub fn tab_ids(&self) -> Vec<Uuid> {
        self.timelines.keys().copied().collect()
    }

    pub fn tab_count(&self) -> usize {
        self.timelines.len()
    }

    pub fn total_states(&self) -> usize {
        self.global_chain.len()
    }

    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }

    pub fn last_checkpoint(&self) -> Option<&Checkpoint> {
        self.checkpoints.last()
    }

    pub fn global_chain(&self) -> &[TabState] {
        &self.global_chain
    }

    /// Summary of the entire history
    pub fn summary(&self) -> HistorySummary {
        let tab_summaries: Vec<TimelineSummary> = self.timelines.values().map(|t| t.summary()).collect();

        HistorySummary {
            total_states: self.global_chain.len(),
            total_tabs: self.timelines.len(),
            total_checkpoints: self.checkpoints.len(),
            chain_valid: self.verify_global_chain().valid,
            created_at: self.created_at,
            tab_summaries,
            last_checkpoint: self.checkpoints.last().map(|c| c.short_root()),
        }
    }

    // === Internal ===

    fn append_to_global(&mut self, state: TabState) {
        self.global_chain.push(state);

        // Auto-checkpoint
        if self.global_chain.len() % self.config.checkpoint_interval == 0 {
            self.create_checkpoint();
        }
    }

    fn create_checkpoint(&mut self) {
        let hashes: Vec<[u8; 32]> = self.global_chain.iter().map(|s| s.hash).collect();

        if let Some(tree) = MerkleTimeline::build(&hashes) {
            self.checkpoints.push(Checkpoint {
                at_index: self.global_chain.len(),
                merkle_root: tree.root_hash(),
                timestamp: Utc::now(),
            });
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum HistoryError {
    #[error("Tab not found: {0}")]
    TabNotFound(Uuid),
}

/// Report from tamper detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TamperReport {
    pub chain_valid: bool,
    pub chain_break_at: Option<usize>,
    pub total_checkpoints: usize,
    pub broken_checkpoints: Vec<usize>,
    pub total_states: usize,
    pub total_tabs: usize,
}

impl TamperReport {
    pub fn is_clean(&self) -> bool {
        self.chain_valid && self.broken_checkpoints.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySummary {
    pub total_states: usize,
    pub total_tabs: usize,
    pub total_checkpoints: usize,
    pub chain_valid: bool,
    pub created_at: DateTime<Utc>,
    pub tab_summaries: Vec<TimelineSummary>,
    pub last_checkpoint: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_history() -> ImmutableHistory {
        ImmutableHistory::new(HistoryConfig {
            checkpoint_interval: 5, // Checkpoint every 5 states for testing
            max_states: 1000,
        })
    }

    #[test]
    fn test_create_tab() {
        let mut history = test_history();
        let tab_id = Uuid::new_v4();

        history.create_tab(tab_id);

        assert_eq!(history.tab_count(), 1);
        assert_eq!(history.total_states(), 1); // Genesis
        assert!(history.get_timeline(tab_id).is_some());
    }

    #[test]
    fn test_navigate_appends_to_global() {
        let mut history = test_history();
        let tab = Uuid::new_v4();
        history.create_tab(tab);

        history.navigate(tab, "https://a.com", b"a", "A", 0).unwrap();
        history.navigate(tab, "https://b.com", b"b", "B", 0).unwrap();

        assert_eq!(history.total_states(), 3); // genesis + 2
    }

    #[test]
    fn test_navigate_nonexistent_tab() {
        let mut history = test_history();
        assert!(history.navigate(Uuid::new_v4(), "url", b"", "T", 0).is_err());
    }

    #[test]
    fn test_back_and_forward() {
        let mut history = test_history();
        let tab = Uuid::new_v4();
        history.create_tab(tab);

        history.navigate(tab, "https://a.com", b"a", "A", 0).unwrap();
        history.navigate(tab, "https://b.com", b"b", "B", 0).unwrap();

        assert!(history.back(tab).unwrap());
        assert_eq!(history.get_timeline(tab).unwrap().current_url(), "https://a.com");

        assert!(history.forward(tab).unwrap());
        assert_eq!(history.get_timeline(tab).unwrap().current_url(), "https://b.com");
    }

    #[test]
    fn test_multiple_tabs() {
        let mut history = test_history();
        let tab1 = Uuid::new_v4();
        let tab2 = Uuid::new_v4();

        history.create_tab(tab1);
        history.create_tab(tab2);

        history.navigate(tab1, "https://a.com", b"a", "A", 0).unwrap();
        history.navigate(tab2, "https://b.com", b"b", "B", 0).unwrap();
        history.navigate(tab1, "https://c.com", b"c", "C", 0).unwrap();

        assert_eq!(history.tab_count(), 2);
        assert_eq!(history.total_states(), 5); // 2 genesis + 3 navigations
    }

    #[test]
    fn test_auto_checkpoint() {
        let mut history = test_history(); // checkpoint_interval = 5
        let tab = Uuid::new_v4();
        history.create_tab(tab); // state 1

        for i in 0..4 {
            history.navigate(tab, format!("https://p{}.com", i), format!("c{}", i).as_bytes(), format!("P{}", i), 0).unwrap();
        }

        // 5 states total → should trigger 1 checkpoint
        assert_eq!(history.total_states(), 5);
        assert_eq!(history.checkpoint_count(), 1);
    }

    #[test]
    fn test_multiple_checkpoints() {
        let mut history = test_history(); // checkpoint_interval = 5
        let tab = Uuid::new_v4();
        history.create_tab(tab);

        for i in 0..14 {
            history.navigate(tab, format!("https://p{}.com", i), format!("c{}", i).as_bytes(), format!("P{}", i), 0).unwrap();
        }

        // 15 states → 3 checkpoints (at 5, 10, 15)
        assert_eq!(history.total_states(), 15);
        assert_eq!(history.checkpoint_count(), 3);
    }

    #[test]
    fn test_verify_chain_valid() {
        let mut history = test_history();
        let tab = Uuid::new_v4();
        history.create_tab(tab);

        for i in 0..10 {
            history.navigate(tab, format!("https://p{}.com", i), b"data", format!("P{}", i), 0).unwrap();
        }

        let result = history.verify_global_chain();
        assert!(result.valid);
    }

    #[test]
    fn test_verify_checkpoint_valid() {
        let mut history = test_history();
        let tab = Uuid::new_v4();
        history.create_tab(tab);

        for i in 0..4 {
            history.navigate(tab, format!("https://p{}.com", i), b"data", format!("P{}", i), 0).unwrap();
        }

        assert!(history.verify_checkpoint(0));
    }

    #[test]
    fn test_detect_tampering_clean() {
        let mut history = test_history();
        let tab = Uuid::new_v4();
        history.create_tab(tab);

        for i in 0..10 {
            history.navigate(tab, format!("https://p{}.com", i), b"data", format!("P{}", i), 0).unwrap();
        }

        let report = history.detect_tampering();
        assert!(report.is_clean());
        assert!(report.chain_valid);
        assert!(report.broken_checkpoints.is_empty());
    }

    #[test]
    fn test_detect_tampering_after_modification() {
        let mut history = test_history();
        let tab = Uuid::new_v4();
        history.create_tab(tab);

        for i in 0..9 {
            history.navigate(tab, format!("https://p{}.com", i), b"data", format!("P{}", i), 0).unwrap();
        }

        // Tamper with a global chain state
        if let Some(state) = history.global_chain.get_mut(3) {
            state.url = "https://TAMPERED.com".to_string();
        }

        // Checkpoints should detect it (merkle root mismatch)
        let report = history.detect_tampering();
        // The chain verification checks per-tab, which may or may not catch
        // the global chain tampering directly. But checkpoint verification will.
        assert!(!report.broken_checkpoints.is_empty() || !report.chain_valid);
    }

    #[test]
    fn test_summary() {
        let mut history = test_history();
        let tab = Uuid::new_v4();
        history.create_tab(tab);
        history.navigate(tab, "https://example.com", b"data", "Example", 0).unwrap();

        let summary = history.summary();
        assert_eq!(summary.total_states, 2);
        assert_eq!(summary.total_tabs, 1);
        assert!(summary.chain_valid);
        assert_eq!(summary.tab_summaries.len(), 1);
    }

    #[test]
    fn test_summary_serialization() {
        let mut history = test_history();
        let tab = Uuid::new_v4();
        history.create_tab(tab);

        let summary = history.summary();
        let json = serde_json::to_string_pretty(&summary).unwrap();
        assert!(json.contains("total_states"));
        assert!(json.contains("chain_valid"));
    }

    #[test]
    fn test_checkpoint_short_root() {
        let mut history = test_history();
        let tab = Uuid::new_v4();
        history.create_tab(tab);

        for i in 0..4 {
            history.navigate(tab, format!("https://p{}.com", i), b"d", format!("P{}", i), 0).unwrap();
        }

        let checkpoint = history.last_checkpoint().unwrap();
        assert_eq!(checkpoint.short_root().len(), 8);
    }
}
