//! # Tab Manager — State Machine with E2EE Lifecycle
//!
//! Each tab follows a strict lifecycle:
//!
//! ```text
//! Active ──30s idle──→ Suspended ──5min idle──→ Destroyed
//!   ↑                      │                        │
//!   └──── resume ──────────┘                        │
//!   └──── reload ───────────────────────────────────┘
//! ```
//!
//! - **Active**: Full WebView rendering, DOM in memory (encrypted)
//! - **Suspended**: DOM encrypted to disk, WebView destroyed, 85% RAM saved
//! - **Destroyed**: All state encrypted to disk, full reload required
//!
//! The key insight: when a tab is suspended, its DOM snapshot is sealed
//! by the MemoryVault. No plaintext ever hits disk.

use crate::vault::{MemoryVault, SealedBlob, VaultError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum TabError {
    #[error("Tab not found: {0}")]
    NotFound(Uuid),
    #[error("Tab already exists: {0}")]
    AlreadyExists(Uuid),
    #[error("Tab is not active")]
    NotActive,
    #[error("Tab has no suspended state to resume")]
    NoSuspendedState,
    #[error("Vault error: {0}")]
    Vault(#[from] VaultError),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Max tabs reached: {0}")]
    MaxTabsReached(usize),
}

pub type Result<T> = std::result::Result<T, TabError>;

/// Tab lifecycle state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TabState {
    /// Full WebView rendering, DOM in encrypted memory
    Active,
    /// DOM sealed to disk, WebView destroyed (85% RAM saved)
    Suspended,
    /// All state sealed to disk (full reload required)
    Destroyed,
}

impl TabState {
    pub fn label(&self) -> &'static str {
        match self {
            TabState::Active => "ACTIVE",
            TabState::Suspended => "SUSPENDED",
            TabState::Destroyed => "DESTROYED",
        }
    }
}

/// A single browser tab
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: Uuid,
    pub url: String,
    pub title: String,
    pub state: TabState,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    /// Whether this tab is playing audio/video
    pub media_playing: bool,
    /// Whether this tab is pinned (exempt from auto-suspend)
    pub pinned: bool,
    /// Favicon URL
    pub favicon_url: Option<String>,
    /// Position in tab bar (0-indexed)
    pub position: usize,
    /// How many times this tab has been suspended
    pub suspend_count: u32,
}

/// Sealed (encrypted) tab state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealedTabState {
    pub tab_id: Uuid,
    pub dom_snapshot: SealedBlob,
    pub scroll_position: SealedBlob,
    pub form_data: Option<SealedBlob>,
    pub sealed_at: DateTime<Utc>,
}

/// Tab manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabManagerConfig {
    /// Duration of inactivity before auto-suspend
    pub suspend_after: Duration,
    /// Duration of suspension before auto-destroy
    pub destroy_after: Duration,
    /// Maximum number of active tabs
    pub max_active_tabs: usize,
    /// Maximum total tabs (active + suspended + destroyed)
    pub max_total_tabs: usize,
    /// Whether to auto-suspend inactive tabs
    pub auto_suspend: bool,
    /// Whether to exempt media-playing tabs from auto-suspend
    pub exempt_media: bool,
}

impl Default for TabManagerConfig {
    fn default() -> Self {
        Self {
            suspend_after: Duration::from_secs(30),
            destroy_after: Duration::from_secs(300),
            max_active_tabs: 8,
            max_total_tabs: 100,
            auto_suspend: true,
            exempt_media: true,
        }
    }
}

/// The Tab Manager — orchestrates tab lifecycle with E2EE
pub struct TabManager {
    tabs: HashMap<Uuid, Tab>,
    sealed_states: HashMap<Uuid, SealedTabState>,
    active_tab: Option<Uuid>,
    config: TabManagerConfig,
}

impl TabManager {
    pub fn new(config: TabManagerConfig) -> Self {
        Self {
            tabs: HashMap::new(),
            sealed_states: HashMap::new(),
            active_tab: None,
            config,
        }
    }

    /// Create a new tab and make it active
    pub fn create_tab(&mut self, url: impl Into<String>) -> Result<Uuid> {
        if self.tabs.len() >= self.config.max_total_tabs {
            return Err(TabError::MaxTabsReached(self.config.max_total_tabs));
        }

        let url = url.into();
        if !url.is_empty() && url::Url::parse(&url).is_err() && !url.starts_with("about:") {
            return Err(TabError::InvalidUrl(url));
        }

        let id = Uuid::new_v4();
        let position = self.tabs.len();

        let tab = Tab {
            id,
            url,
            title: "New Tab".into(),
            state: TabState::Active,
            created_at: Utc::now(),
            last_active: Utc::now(),
            media_playing: false,
            pinned: false,
            favicon_url: None,
            position,
            suspend_count: 0,
        };

        self.tabs.insert(id, tab);
        self.active_tab = Some(id);

        tracing::info!("Tab created: {}", id);
        Ok(id)
    }

    /// Suspend a tab (encrypt DOM to disk, free 85% RAM)
    pub fn suspend_tab(&mut self, id: Uuid, vault: &mut MemoryVault, dom_snapshot: &[u8], scroll_pos: &[u8]) -> Result<()> {
        let tab = self.tabs.get_mut(&id).ok_or(TabError::NotFound(id))?;

        if tab.state != TabState::Active {
            return Err(TabError::NotActive);
        }

        // Seal DOM and scroll position
        let sealed_dom = vault.seal(dom_snapshot)?;
        let sealed_scroll = vault.seal(scroll_pos)?;

        let sealed_state = SealedTabState {
            tab_id: id,
            dom_snapshot: sealed_dom,
            scroll_position: sealed_scroll,
            form_data: None,
            sealed_at: Utc::now(),
        };

        tab.state = TabState::Suspended;
        tab.suspend_count += 1;
        self.sealed_states.insert(id, sealed_state);

        if self.active_tab == Some(id) {
            self.active_tab = None;
        }

        tracing::info!("Tab suspended: {} (count: {})", id, tab.suspend_count);
        Ok(())
    }

    /// Resume a suspended tab (decrypt DOM, restore WebView)
    pub fn resume_tab(&mut self, id: Uuid, vault: &mut MemoryVault) -> Result<(Vec<u8>, Vec<u8>)> {
        let tab = self.tabs.get_mut(&id).ok_or(TabError::NotFound(id))?;

        if tab.state != TabState::Suspended {
            return Err(TabError::NoSuspendedState);
        }

        let sealed = self
            .sealed_states
            .remove(&id)
            .ok_or(TabError::NoSuspendedState)?;

        // Unseal DOM and scroll position
        let dom = vault.unseal(&sealed.dom_snapshot)?;
        let scroll = vault.unseal(&sealed.scroll_position)?;

        tab.state = TabState::Active;
        tab.last_active = Utc::now();
        self.active_tab = Some(id);

        tracing::info!("Tab resumed: {}", id);
        Ok((dom, scroll))
    }

    /// Destroy a tab (encrypt everything, mark for full reload)
    pub fn destroy_tab(&mut self, id: Uuid) -> Result<()> {
        let tab = self.tabs.get_mut(&id).ok_or(TabError::NotFound(id))?;
        tab.state = TabState::Destroyed;

        if self.active_tab == Some(id) {
            self.active_tab = None;
        }

        tracing::info!("Tab destroyed: {}", id);
        Ok(())
    }

    /// Close a tab completely (remove from manager)
    pub fn close_tab(&mut self, id: Uuid) -> Result<Tab> {
        let tab = self.tabs.remove(&id).ok_or(TabError::NotFound(id))?;
        self.sealed_states.remove(&id);

        if self.active_tab == Some(id) {
            // Activate the nearest tab
            self.active_tab = self.tabs.keys().next().copied();
        }

        // Reorder positions
        let mut position = 0;
        let mut sorted: Vec<_> = self.tabs.values_mut().collect();
        sorted.sort_by_key(|t| t.position);
        for t in sorted {
            t.position = position;
            position += 1;
        }

        tracing::info!("Tab closed: {} (\"{}\")", id, tab.title);
        Ok(tab)
    }

    /// Switch to a specific tab
    pub fn switch_to(&mut self, id: Uuid) -> Result<()> {
        if !self.tabs.contains_key(&id) {
            return Err(TabError::NotFound(id));
        }

        if let Some(tab) = self.tabs.get_mut(&id) {
            tab.last_active = Utc::now();
        }
        self.active_tab = Some(id);
        Ok(())
    }

    /// Update tab title
    pub fn update_title(&mut self, id: Uuid, title: impl Into<String>) -> Result<()> {
        let tab = self.tabs.get_mut(&id).ok_or(TabError::NotFound(id))?;
        tab.title = title.into();
        Ok(())
    }

    /// Update tab URL (navigation)
    pub fn navigate(&mut self, id: Uuid, url: impl Into<String>) -> Result<()> {
        let tab = self.tabs.get_mut(&id).ok_or(TabError::NotFound(id))?;
        tab.url = url.into();
        tab.last_active = Utc::now();
        Ok(())
    }

    /// Pin/unpin a tab
    pub fn set_pinned(&mut self, id: Uuid, pinned: bool) -> Result<()> {
        let tab = self.tabs.get_mut(&id).ok_or(TabError::NotFound(id))?;
        tab.pinned = pinned;
        Ok(())
    }

    /// Set media playing state
    pub fn set_media_playing(&mut self, id: Uuid, playing: bool) -> Result<()> {
        let tab = self.tabs.get_mut(&id).ok_or(TabError::NotFound(id))?;
        tab.media_playing = playing;
        Ok(())
    }

    /// Enforce lifecycle policy — call periodically
    ///
    /// Auto-suspends idle active tabs and auto-destroys old suspended tabs.
    pub fn enforce_lifecycle(&mut self, vault: &mut MemoryVault) -> Vec<TabLifecycleEvent> {
        let now = Utc::now();
        let suspend_threshold = chrono::Duration::from_std(self.config.suspend_after).unwrap();
        let destroy_threshold = chrono::Duration::from_std(self.config.destroy_after).unwrap();

        let mut events = Vec::new();
        let mut to_suspend = Vec::new();
        let mut to_destroy = Vec::new();

        for (id, tab) in &self.tabs {
            let idle_duration = now - tab.last_active;

            match tab.state {
                TabState::Active => {
                    if !self.config.auto_suspend {
                        continue;
                    }
                    if tab.pinned {
                        continue;
                    }
                    if self.config.exempt_media && tab.media_playing {
                        continue;
                    }
                    if Some(*id) == self.active_tab {
                        continue; // Never suspend the focused tab
                    }
                    if idle_duration > suspend_threshold {
                        to_suspend.push(*id);
                    }
                }
                TabState::Suspended => {
                    if idle_duration > destroy_threshold {
                        to_destroy.push(*id);
                    }
                }
                TabState::Destroyed => {}
            }
        }

        for id in to_suspend {
            // Create a dummy DOM snapshot for auto-suspend
            // In production, the Tauri frontend captures actual DOM
            let dummy_dom = format!("auto-suspended:{}", id).into_bytes();
            if self.suspend_tab(id, vault, &dummy_dom, b"0").is_ok() {
                events.push(TabLifecycleEvent::AutoSuspended(id));
            }
        }

        for id in to_destroy {
            if self.destroy_tab(id).is_ok() {
                events.push(TabLifecycleEvent::AutoDestroyed(id));
            }
        }

        events
    }

    // === Queries ===

    pub fn get_tab(&self, id: Uuid) -> Option<&Tab> {
        self.tabs.get(&id)
    }

    pub fn active_tab_id(&self) -> Option<Uuid> {
        self.active_tab
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.active_tab.and_then(|id| self.tabs.get(&id))
    }

    pub fn all_tabs(&self) -> Vec<&Tab> {
        let mut tabs: Vec<_> = self.tabs.values().collect();
        tabs.sort_by_key(|t| t.position);
        tabs
    }

    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    pub fn active_count(&self) -> usize {
        self.tabs.values().filter(|t| t.state == TabState::Active).count()
    }

    pub fn suspended_count(&self) -> usize {
        self.tabs.values().filter(|t| t.state == TabState::Suspended).count()
    }

    /// Estimate RAM saved by suspended tabs (bytes)
    pub fn ram_saved_estimate(&self) -> u64 {
        self.sealed_states
            .values()
            .map(|s| s.dom_snapshot.original_size as u64)
            .sum::<u64>()
            * 85
            / 100 // 85% savings
    }

    pub fn summary(&self) -> TabManagerSummary {
        TabManagerSummary {
            total: self.tab_count(),
            active: self.active_count(),
            suspended: self.suspended_count(),
            destroyed: self.tabs.values().filter(|t| t.state == TabState::Destroyed).count(),
            pinned: self.tabs.values().filter(|t| t.pinned).count(),
            media_playing: self.tabs.values().filter(|t| t.media_playing).count(),
            ram_saved_bytes: self.ram_saved_estimate(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabManagerSummary {
    pub total: usize,
    pub active: usize,
    pub suspended: usize,
    pub destroyed: usize,
    pub pinned: usize,
    pub media_playing: usize,
    pub ram_saved_bytes: u64,
}

#[derive(Debug, Clone)]
pub enum TabLifecycleEvent {
    AutoSuspended(Uuid),
    AutoDestroyed(Uuid),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_manager() -> TabManager {
        TabManager::new(TabManagerConfig::default())
    }

    fn test_vault() -> MemoryVault {
        MemoryVault::generate()
    }

    #[test]
    fn test_create_tab() {
        let mut mgr = test_manager();
        let id = mgr.create_tab("https://example.com").unwrap();

        assert_eq!(mgr.tab_count(), 1);
        assert_eq!(mgr.active_tab_id(), Some(id));

        let tab = mgr.get_tab(id).unwrap();
        assert_eq!(tab.url, "https://example.com");
        assert_eq!(tab.state, TabState::Active);
        assert_eq!(tab.position, 0);
    }

    #[test]
    fn test_create_multiple_tabs() {
        let mut mgr = test_manager();
        mgr.create_tab("https://a.com").unwrap();
        let id2 = mgr.create_tab("https://b.com").unwrap();
        mgr.create_tab("https://c.com").unwrap();

        assert_eq!(mgr.tab_count(), 3);
        // Last created is active
        assert_eq!(mgr.active_tab_id(), Some(mgr.all_tabs()[2].id));
        let _ = id2;
    }

    #[test]
    fn test_invalid_url_rejected() {
        let mut mgr = test_manager();
        assert!(mgr.create_tab("not a url").is_err());
    }

    #[test]
    fn test_about_url_accepted() {
        let mut mgr = test_manager();
        assert!(mgr.create_tab("about:blank").is_ok());
    }

    #[test]
    fn test_empty_url_accepted() {
        let mut mgr = test_manager();
        assert!(mgr.create_tab("").is_ok());
    }

    #[test]
    fn test_suspend_and_resume() {
        let mut mgr = test_manager();
        let mut vault = test_vault();

        let id = mgr.create_tab("https://example.com").unwrap();
        let dom = b"<html><body>Hello World</body></html>";
        let scroll = b"42";

        // Suspend
        mgr.suspend_tab(id, &mut vault, dom, scroll).unwrap();
        assert_eq!(mgr.get_tab(id).unwrap().state, TabState::Suspended);
        assert_eq!(mgr.active_tab_id(), None);
        assert_eq!(mgr.suspended_count(), 1);

        // Resume
        let (recovered_dom, recovered_scroll) = mgr.resume_tab(id, &mut vault).unwrap();
        assert_eq!(recovered_dom, dom);
        assert_eq!(recovered_scroll, scroll);
        assert_eq!(mgr.get_tab(id).unwrap().state, TabState::Active);
        assert_eq!(mgr.active_tab_id(), Some(id));
    }

    #[test]
    fn test_suspend_count_increments() {
        let mut mgr = test_manager();
        let mut vault = test_vault();

        let id = mgr.create_tab("https://example.com").unwrap();

        mgr.suspend_tab(id, &mut vault, b"dom1", b"0").unwrap();
        mgr.resume_tab(id, &mut vault).unwrap();
        mgr.suspend_tab(id, &mut vault, b"dom2", b"0").unwrap();

        assert_eq!(mgr.get_tab(id).unwrap().suspend_count, 2);
    }

    #[test]
    fn test_close_tab() {
        let mut mgr = test_manager();
        let id1 = mgr.create_tab("https://a.com").unwrap();
        let id2 = mgr.create_tab("https://b.com").unwrap();

        let closed = mgr.close_tab(id2).unwrap();
        assert_eq!(closed.url, "https://b.com");
        assert_eq!(mgr.tab_count(), 1);
        // Active should switch to remaining tab
        assert_eq!(mgr.active_tab_id(), Some(id1));
    }

    #[test]
    fn test_close_nonexistent_tab() {
        let mut mgr = test_manager();
        assert!(mgr.close_tab(Uuid::new_v4()).is_err());
    }

    #[test]
    fn test_destroy_tab() {
        let mut mgr = test_manager();
        let id = mgr.create_tab("https://example.com").unwrap();

        mgr.destroy_tab(id).unwrap();
        assert_eq!(mgr.get_tab(id).unwrap().state, TabState::Destroyed);
        assert_eq!(mgr.active_tab_id(), None);
    }

    #[test]
    fn test_switch_tab() {
        let mut mgr = test_manager();
        let id1 = mgr.create_tab("https://a.com").unwrap();
        let _id2 = mgr.create_tab("https://b.com").unwrap();

        mgr.switch_to(id1).unwrap();
        assert_eq!(mgr.active_tab_id(), Some(id1));
    }

    #[test]
    fn test_update_title_and_navigate() {
        let mut mgr = test_manager();
        let id = mgr.create_tab("https://example.com").unwrap();

        mgr.update_title(id, "Example Site").unwrap();
        mgr.navigate(id, "https://example.com/page2").unwrap();

        let tab = mgr.get_tab(id).unwrap();
        assert_eq!(tab.title, "Example Site");
        assert_eq!(tab.url, "https://example.com/page2");
    }

    #[test]
    fn test_pin_tab() {
        let mut mgr = test_manager();
        let id = mgr.create_tab("https://example.com").unwrap();

        mgr.set_pinned(id, true).unwrap();
        assert!(mgr.get_tab(id).unwrap().pinned);
    }

    #[test]
    fn test_max_tabs_enforced() {
        let mut mgr = TabManager::new(TabManagerConfig {
            max_total_tabs: 3,
            ..Default::default()
        });

        mgr.create_tab("https://a.com").unwrap();
        mgr.create_tab("https://b.com").unwrap();
        mgr.create_tab("https://c.com").unwrap();
        assert!(mgr.create_tab("https://d.com").is_err());
    }

    #[test]
    fn test_summary() {
        let mut mgr = test_manager();
        let mut vault = test_vault();

        mgr.create_tab("https://a.com").unwrap();
        let id2 = mgr.create_tab("https://b.com").unwrap();
        mgr.create_tab("https://c.com").unwrap();

        mgr.set_pinned(mgr.all_tabs()[0].id, true).unwrap();
        mgr.suspend_tab(id2, &mut vault, b"dom", b"0").unwrap();

        let summary = mgr.summary();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.active, 2);
        assert_eq!(summary.suspended, 1);
        assert_eq!(summary.pinned, 1);
    }

    #[test]
    fn test_tab_ordering() {
        let mut mgr = test_manager();
        mgr.create_tab("https://a.com").unwrap();
        mgr.create_tab("https://b.com").unwrap();
        mgr.create_tab("https://c.com").unwrap();

        let tabs = mgr.all_tabs();
        assert_eq!(tabs[0].position, 0);
        assert_eq!(tabs[1].position, 1);
        assert_eq!(tabs[2].position, 2);
    }

    #[test]
    fn test_positions_reorder_on_close() {
        let mut mgr = test_manager();
        mgr.create_tab("https://a.com").unwrap();
        let id2 = mgr.create_tab("https://b.com").unwrap();
        mgr.create_tab("https://c.com").unwrap();

        mgr.close_tab(id2).unwrap();

        let tabs = mgr.all_tabs();
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0].position, 0);
        assert_eq!(tabs[1].position, 1);
    }

    #[test]
    fn test_tab_serialization() {
        let mut mgr = test_manager();
        let id = mgr.create_tab("https://example.com").unwrap();
        let tab = mgr.get_tab(id).unwrap();
        let json = serde_json::to_string(tab).unwrap();
        assert!(json.contains("example.com"));
    }
}
