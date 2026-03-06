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
pub mod blueprint;
pub mod toolshed;
pub mod sandbox;

// Explicit re-exports to avoid ambiguous glob conflicts (domain::Result vs harness::Result)
pub use domain::{
    DomainMemory, Feature, FeatureBacklog, FeatureStatus, FailedApproach,
    ScaffoldingRules, SystemState, TestCommand, TestSnapshot, WorkerRunLog,
    RunOutcome, ProgressLog,
};
pub use initializer::{FeatureSpec, Initializer};
pub use worker::{FeatureExecutor, TestRunner, Worker, WorkerResult};
pub use harness::{Harness, HarnessConfig};
pub use blueprint::{
    Blueprint, BlueprintContext, BlueprintResult, BlueprintRunner, BlueprintStep,
    StepKind, StepOutcome, AgentBackend, AgentResponse,
};
pub use toolshed::{
    Toolshed, ToolDefinition, ToolSelection, ToolHandler, ToolInvocation,
    PermissionLevel, ToolshedSummary,
};
pub use sandbox::{
    Sandbox, SandboxConfig, SandboxManager, SandboxManagerSummary,
    SandboxState, ResourceUsage,
};
