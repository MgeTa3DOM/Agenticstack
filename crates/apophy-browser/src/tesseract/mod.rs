//! # Tesseract Tachionique — Temporal Hash Architecture
//!
//! "Aussi simple que complique car c'est complique de faire simple."
//!
//! A tesseract is a 4D hypercube: three spatial dimensions + time.
//! Here, time IS the hash chain. Each state folds into the next.
//! Past validates present. Present validates future. The cycle is the proof.
//!
//! ## The Bitcoin Insight
//!
//! The real value of Bitcoin is the regular hash since genesis.
//! Nobody can cheat. No more stealing, no more thieves.
//! We apply this to browser state: an immutable personal ledger.
//!
//! ## Mandala Fractal Holographique
//!
//! Each Merkle root contains the fingerprint of ALL states.
//! Break the chain — each fragment still proves its authenticity.
//! Self-similar at every scale. The part contains the whole.

pub mod state;
pub mod timeline;
pub mod merkle;
pub mod history;

pub use state::*;
pub use timeline::*;
pub use merkle::*;
pub use history::*;
