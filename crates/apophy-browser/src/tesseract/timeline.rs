//! # Timeline — Past/Present/Future Navigation
//!
//! Three tmux panes folded into one dimension:
//! - **Past**: states you've been through (immutable)
//! - **Present**: the state you're in (the only mutable point)
//! - **Future**: states you can go back to (redo stack)
//!
//! Navigate backward: present → future, past.pop() → present
//! Navigate forward: present → past, future.pop() → present
//!
//! The cycle: past validates present validates future validates past.
//! Fold the timeline and the Merkle root contains everything.

use crate::tesseract::state::{ChainVerification, TabState};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The Timeline — temporal navigation with hash chain integrity.
///
/// Like tmux windows: past|present|future — but each window is
/// cryptographically chained to the others.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    /// Tab this timeline belongs to
    pub tab_id: Uuid,
    /// Past states (oldest first)
    past: Vec<TabState>,
    /// Current state
    present: TabState,
    /// Future states (redo stack, most recent first)
    future: Vec<TabState>,
}

impl Timeline {
    /// Create a new timeline from genesis
    pub fn new(tab_id: Uuid) -> Self {
        Self {
            tab_id,
            past: Vec::new(),
            present: TabState::genesis(tab_id),
            future: Vec::new(),
        }
    }

    /// Navigate to a new state (present becomes past, new state becomes present)
    pub fn navigate(&mut self, url: impl Into<String>, content: &[u8], title: impl Into<String>, scroll_y: u32) {
        let prev_hash = self.present.hash;

        // Present slides into past
        self.past.push(self.present.clone());

        // New state becomes present (chained from old present)
        self.present = TabState::new(
            self.tab_id,
            url,
            content,
            title,
            scroll_y,
            prev_hash,
        );

        // Future is cleared (new branch of time)
        self.future.clear();
    }

    /// Go back in time (present → future, past.last() → present)
    pub fn back(&mut self) -> bool {
        if self.past.is_empty() {
            return false;
        }

        // Present slides into future
        self.future.insert(0, self.present.clone());

        // Past becomes present
        self.present = self.past.pop().unwrap();
        true
    }

    /// Go forward in time (present → past, future.first() → present)
    pub fn forward(&mut self) -> bool {
        if self.future.is_empty() {
            return false;
        }

        // Present slides into past
        self.past.push(self.present.clone());

        // Future becomes present
        self.present = self.future.remove(0);
        true
    }

    /// Fold the entire timeline into a single chain (past + present + future)
    pub fn fold(&self) -> Vec<TabState> {
        let mut chain = self.past.clone();
        chain.push(self.present.clone());
        chain.extend(self.future.iter().cloned());
        chain
    }

    /// Verify the integrity of the entire folded timeline
    pub fn verify_integrity(&self) -> ChainVerification {
        // Verify past chain up to present
        let mut chain = self.past.clone();
        chain.push(self.present.clone());
        TabState::verify_chain(&chain)
    }

    // === Accessors ===

    pub fn present(&self) -> &TabState {
        &self.present
    }

    pub fn past(&self) -> &[TabState] {
        &self.past
    }

    pub fn future(&self) -> &[TabState] {
        &self.future
    }

    pub fn can_go_back(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.future.is_empty()
    }

    pub fn depth(&self) -> usize {
        self.past.len() + 1 + self.future.len()
    }

    pub fn past_depth(&self) -> usize {
        self.past.len()
    }

    pub fn future_depth(&self) -> usize {
        self.future.len()
    }

    /// Get the current URL
    pub fn current_url(&self) -> &str {
        &self.present.url
    }

    /// Get the current title
    pub fn current_title(&self) -> &str {
        &self.present.title
    }

    /// Summary for display
    pub fn summary(&self) -> TimelineSummary {
        TimelineSummary {
            tab_id: self.tab_id,
            past_count: self.past.len(),
            current_url: self.present.url.clone(),
            current_title: self.present.title.clone(),
            current_hash: self.present.short_hash(),
            future_count: self.future.len(),
            total_depth: self.depth(),
            chain_valid: self.verify_integrity().valid,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineSummary {
    pub tab_id: Uuid,
    pub past_count: usize,
    pub current_url: String,
    pub current_title: String,
    pub current_hash: String,
    pub future_count: usize,
    pub total_depth: usize,
    pub chain_valid: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tab() -> Uuid {
        Uuid::new_v4()
    }

    #[test]
    fn test_new_timeline() {
        let tab = test_tab();
        let tl = Timeline::new(tab);

        assert_eq!(tl.current_url(), "about:blank");
        assert_eq!(tl.past_depth(), 0);
        assert_eq!(tl.future_depth(), 0);
        assert_eq!(tl.depth(), 1); // Just genesis
        assert!(!tl.can_go_back());
        assert!(!tl.can_go_forward());
    }

    #[test]
    fn test_navigate_builds_past() {
        let tab = test_tab();
        let mut tl = Timeline::new(tab);

        tl.navigate("https://a.com", b"a", "Page A", 0);
        assert_eq!(tl.current_url(), "https://a.com");
        assert_eq!(tl.past_depth(), 1); // genesis in past
        assert!(tl.can_go_back());

        tl.navigate("https://b.com", b"b", "Page B", 100);
        assert_eq!(tl.current_url(), "https://b.com");
        assert_eq!(tl.past_depth(), 2);
    }

    #[test]
    fn test_back_and_forward() {
        let tab = test_tab();
        let mut tl = Timeline::new(tab);

        tl.navigate("https://a.com", b"a", "A", 0);
        tl.navigate("https://b.com", b"b", "B", 0);
        tl.navigate("https://c.com", b"c", "C", 0);

        assert_eq!(tl.current_url(), "https://c.com");

        // Go back
        assert!(tl.back());
        assert_eq!(tl.current_url(), "https://b.com");
        assert_eq!(tl.future_depth(), 1); // C is in future

        assert!(tl.back());
        assert_eq!(tl.current_url(), "https://a.com");
        assert_eq!(tl.future_depth(), 2); // B, C in future

        // Go forward
        assert!(tl.forward());
        assert_eq!(tl.current_url(), "https://b.com");
        assert_eq!(tl.future_depth(), 1);

        assert!(tl.forward());
        assert_eq!(tl.current_url(), "https://c.com");
        assert_eq!(tl.future_depth(), 0);
    }

    #[test]
    fn test_back_at_genesis_returns_false() {
        let tl = Timeline::new(test_tab());
        assert!(!tl.can_go_back());
    }

    #[test]
    fn test_forward_with_no_future_returns_false() {
        let tab = test_tab();
        let mut tl = Timeline::new(tab);
        tl.navigate("https://a.com", b"a", "A", 0);
        assert!(!tl.can_go_forward());
    }

    #[test]
    fn test_navigate_clears_future() {
        let tab = test_tab();
        let mut tl = Timeline::new(tab);

        tl.navigate("https://a.com", b"a", "A", 0);
        tl.navigate("https://b.com", b"b", "B", 0);

        // Go back
        tl.back();
        assert_eq!(tl.future_depth(), 1);

        // Navigate to new page (forks the timeline)
        tl.navigate("https://d.com", b"d", "D", 0);
        assert_eq!(tl.future_depth(), 0); // Future cleared
        assert_eq!(tl.current_url(), "https://d.com");
    }

    #[test]
    fn test_fold_contains_all_states() {
        let tab = test_tab();
        let mut tl = Timeline::new(tab);

        tl.navigate("https://a.com", b"a", "A", 0);
        tl.navigate("https://b.com", b"b", "B", 0);
        tl.navigate("https://c.com", b"c", "C", 0);
        tl.back(); // C goes to future

        let folded = tl.fold();
        assert_eq!(folded.len(), 4); // genesis + a + b(present) + c(future)
    }

    #[test]
    fn test_chain_integrity_valid() {
        let tab = test_tab();
        let mut tl = Timeline::new(tab);

        tl.navigate("https://a.com", b"a", "A", 0);
        tl.navigate("https://b.com", b"b", "B", 0);
        tl.navigate("https://c.com", b"c", "C", 0);

        let verification = tl.verify_integrity();
        assert!(verification.valid);
        assert_eq!(verification.length, 4);
    }

    #[test]
    fn test_summary() {
        let tab = test_tab();
        let mut tl = Timeline::new(tab);
        tl.navigate("https://example.com", b"data", "Example", 0);

        let summary = tl.summary();
        assert_eq!(summary.current_url, "https://example.com");
        assert_eq!(summary.past_count, 1);
        assert_eq!(summary.future_count, 0);
        assert!(summary.chain_valid);
        assert_eq!(summary.current_hash.len(), 8);
    }

    #[test]
    fn test_summary_serialization() {
        let tl = Timeline::new(test_tab());
        let summary = tl.summary();
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("about:blank"));
        assert!(json.contains("chain_valid"));
    }

    #[test]
    fn test_deep_navigation() {
        let tab = test_tab();
        let mut tl = Timeline::new(tab);

        for i in 0..100 {
            tl.navigate(
                format!("https://page{}.com", i),
                format!("content-{}", i).as_bytes(),
                format!("Page {}", i),
                i * 10,
            );
        }

        assert_eq!(tl.past_depth(), 100); // genesis + 99 pages
        assert!(tl.verify_integrity().valid);

        // Go back 50 pages
        for _ in 0..50 {
            tl.back();
        }
        assert_eq!(tl.past_depth(), 50);
        assert_eq!(tl.future_depth(), 50);
    }
}
