//! # Apophy Browser — Le Miroir Sans Surface
//!
//! Sovereign encrypted browser core. No surface, no point of failure.
//!
//! Architecture: Tauri frontend (WebView) ←IPC→ Rust backend (this crate)
//!
//! ## Zero-Trust Principles
//!
//! 1. **Encrypt before memory**: Data is ChaCha20-Poly1305 sealed before
//!    entering RAM. The application never "sees" plaintext at rest.
//! 2. **Tab lifecycle E2EE**: Suspended tabs are encrypted to disk.
//!    Only the active tab holds decrypted DOM.
//! 3. **COM-level blocking**: Ads/trackers blocked BEFORE HTTP connection,
//!    not after (JS-level blocking is bypassable).
//! 4. **Zero-cloud sync**: Noise Protocol over mDNS. No Google, no Mozilla,
//!    no Microsoft. Your data stays on your LAN.
//! 5. **Fingerprint mitigations**: 23 techniques to prevent tracking.

pub mod vault;
pub mod tabs;
pub mod blocker;
pub mod shield;
pub mod sync;
pub mod core;

pub use vault::MemoryVault;
pub use tabs::{Tab, TabManager, TabState};
pub use blocker::{ContentBlocker, BlockRule, BlockDecision};
pub use shield::{FingerprintShield, Mitigation};
pub use sync::{LocalSync, SyncPeer, PairingState};
pub use core::BrowserCore;
