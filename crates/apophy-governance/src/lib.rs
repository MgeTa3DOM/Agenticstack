//! # Apophy Governance — Infrastructure Mode for Sovereign AI
//!
//! "90% des entreprises perdent ce pari. Pas nous."
//!
//! This crate implements the four levels of the Infrastructure Mode skill tree:
//!
//! - **Level 1 Conditioning** (intent spec, context engineering, constraints)
//! - **Level 2 Authority** (verification, provenance, permission envelopes)
//! - **Level 3 Workflows** (pipeline decomposition, failure taxonomy, observability)
//! - **Level 4 Compounding** (eval harness, feedback loops, drift governance)
//!
//! ## Why This Exists
//!
//! Tool Mode: send prompt → get output → hope for the best → retry on failure.
//! Infrastructure Mode: design flow → verify output → maintain authority → compound gains.
//!
//! The difference: 30-50% less token waste, exponentially less risk, 10x leverage.

pub mod observe;
pub mod verify;
pub mod permit;
pub mod taxonomy;
pub mod eval;

pub use observe::{LlmTrace, TraceLog, TraceSummary};
pub use verify::{Verification, VerificationGate, VerificationResult};
pub use permit::{Permission, PermissionEnvelope, PermitDecision};
pub use taxonomy::{FailureMode, FailureTaxonomy, Diagnosis};
pub use eval::{EvalHarness, EvalResult, GoldenExample, Scorecard};
