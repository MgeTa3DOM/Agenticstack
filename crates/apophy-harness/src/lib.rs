//! # Apophy Harness — Initializer/Worker Agent Pattern
//!
//! **"The moat is not the model — it's the harness and domain memory."**
//!
//! This crate implements the Initializer/Worker pattern discovered by Anthropic
//! for building agents that actually work in production:
//!
//! - **Initializer**: Transforms a goal into a machine-readable feature backlog
//!   with testable pass/fail criteria. Runs once (or infrequently).
//!
//! - **Worker**: A pure function that reads externalized memory, picks ONE failing
//!   item, works on it, tests it, updates state, and exits. Runs repeatedly.
//!
//! The agent is stateless; the memory is persistent. This inversion is essential.
//!
//! ## Domain Memory Schema (4 pillars)
//!
//! | Component | Purpose |
//! |---|---|
//! | **Goals & Requirements** | Machine-readable backlog with pass/fail status |
//! | **State Tracking** | Current situation: what's passing, failing, broken |
//! | **Scaffolding Rules** | How we operate: test framework, validation, constraints |
//! | **Progress Log** | Machine-readable run history the agent reads to reason |
//!
//! ## Integration
//!
//! - Backed by `apophy-memory` (MemoryPalace) for persistence
//! - Uses `apophy-graph` (TransparentGraph) for workflow DAGs
//! - Wired into `apophy-sovereign` via CLI and API

pub mod domain;
pub mod initializer;
pub mod worker;
pub mod harness;

pub use domain::*;
pub use initializer::*;
pub use worker::*;
pub use harness::*;
