//! # Sandbox — Dev Box Isolation for Parallel Agent Execution
//!
//! Inspired by Stripe's Dev Boxes: each agent runs in an isolated sandbox
//! with its own filesystem view, resource limits, and rollback capability.
//!
//! ## Why Sandboxes
//!
//! When multiple agents work in parallel (e.g., different features in a
//! Blueprint), they can't share mutable state without conflicts. Sandboxes
//! provide:
//!
//! - **Filesystem isolation**: Each agent sees a snapshot, not the live tree
//! - **Resource limits**: CPU time, memory, disk, network (sovereign = no net)
//! - **Rollback**: If an agent fails, its sandbox is discarded. Zero damage.
//! - **Merge**: If an agent succeeds, its changes are merged into the main tree
//!
//! ## Sovereign Twist
//!
//! No cloud VMs. Sandboxes are lightweight process-level isolation using
//! temporary directories and resource tracking. For true isolation, back
//! with Linux namespaces (cgroups + unshare).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

// =============================================================================
// SANDBOX CONFIG — resource limits and isolation rules
// =============================================================================

/// Configuration for a sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Maximum execution time in seconds
    pub max_duration_secs: u64,
    /// Maximum memory in bytes (0 = unlimited)
    pub max_memory_bytes: u64,
    /// Maximum disk usage in bytes (0 = unlimited)
    pub max_disk_bytes: u64,
    /// Whether network access is allowed
    pub allow_network: bool,
    /// Whether the sandbox can execute shell commands
    pub allow_exec: bool,
    /// Base directory for sandbox working directories
    pub base_dir: PathBuf,
    /// Environment variables to pass into the sandbox
    pub env_vars: HashMap<String, String>,
    /// Files/directories to copy into the sandbox (source → dest relative path)
    pub initial_files: Vec<(PathBuf, PathBuf)>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_duration_secs: 300,      // 5 minutes
            max_memory_bytes: 512 * 1024 * 1024, // 512 MB
            max_disk_bytes: 1024 * 1024 * 1024,  // 1 GB
            allow_network: false,        // Sovereign = no network by default
            allow_exec: true,
            base_dir: PathBuf::from("/tmp/apophy-sandboxes"),
            env_vars: HashMap::new(),
            initial_files: Vec::new(),
        }
    }
}

impl SandboxConfig {
    /// Sovereign mode: no network, strict limits
    pub fn sovereign() -> Self {
        let mut config = Self::default();
        config.allow_network = false;
        config.max_duration_secs = 120;
        config.env_vars.insert("APOPHY_SOVEREIGN".into(), "true".into());
        config.env_vars.insert("APOPHY_NO_EGRESS".into(), "true".into());
        config
    }

    /// Permissive mode: for testing
    pub fn permissive() -> Self {
        let mut config = Self::default();
        config.allow_network = true;
        config.max_duration_secs = 600;
        config.max_memory_bytes = 0; // unlimited
        config.max_disk_bytes = 0;   // unlimited
        config
    }
}

// =============================================================================
// SANDBOX STATE — lifecycle tracking
// =============================================================================

/// Sandbox lifecycle state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SandboxState {
    /// Created but not yet started
    Ready,
    /// Currently running an agent
    Running,
    /// Agent completed successfully
    Completed,
    /// Agent failed
    Failed,
    /// Changes merged into main tree
    Merged,
    /// Sandbox discarded (failed or no longer needed)
    Discarded,
}

impl SandboxState {
    pub fn label(&self) -> &'static str {
        match self {
            SandboxState::Ready => "READY",
            SandboxState::Running => "RUNNING",
            SandboxState::Completed => "COMPLETED",
            SandboxState::Failed => "FAILED",
            SandboxState::Merged => "MERGED",
            SandboxState::Discarded => "DISCARDED",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, SandboxState::Merged | SandboxState::Discarded)
    }
}

// =============================================================================
// SANDBOX — isolated execution environment
// =============================================================================

/// A single sandbox instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sandbox {
    /// Unique sandbox ID
    pub id: Uuid,
    /// Human-readable name (e.g., "feature-auth-impl")
    pub name: String,
    /// Working directory for this sandbox
    pub work_dir: PathBuf,
    /// Current state
    pub state: SandboxState,
    /// Configuration
    pub config: SandboxConfig,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Started at (when agent began executing)
    pub started_at: Option<DateTime<Utc>>,
    /// Completed at
    pub completed_at: Option<DateTime<Utc>>,
    /// Files created or modified by the agent (relative paths)
    pub modified_files: Vec<PathBuf>,
    /// Resource usage tracked during execution
    pub resource_usage: ResourceUsage,
    /// Exit message (success or error)
    pub exit_message: Option<String>,
    /// Agent that ran in this sandbox
    pub agent_id: Option<String>,
}

/// Resource usage tracking
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceUsage {
    /// Elapsed wall-clock time in ms
    pub elapsed_ms: u64,
    /// Peak memory usage in bytes
    pub peak_memory_bytes: u64,
    /// Total disk bytes written
    pub disk_bytes_written: u64,
    /// Number of files created
    pub files_created: u32,
    /// Number of files modified
    pub files_modified: u32,
    /// Number of shell commands executed
    pub commands_executed: u32,
}

impl Sandbox {
    pub fn new(name: impl Into<String>, config: SandboxConfig) -> Self {
        let id = Uuid::new_v4();
        let work_dir = config.base_dir.join(id.to_string());

        Self {
            id,
            name: name.into(),
            work_dir,
            state: SandboxState::Ready,
            config,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            modified_files: Vec::new(),
            resource_usage: ResourceUsage::default(),
            exit_message: None,
            agent_id: None,
        }
    }

    /// Mark as running
    pub fn start(&mut self, agent_id: impl Into<String>) {
        self.state = SandboxState::Running;
        self.started_at = Some(Utc::now());
        self.agent_id = Some(agent_id.into());
    }

    /// Mark as completed successfully
    pub fn complete(&mut self, message: impl Into<String>) {
        self.state = SandboxState::Completed;
        self.completed_at = Some(Utc::now());
        self.exit_message = Some(message.into());
        self.update_elapsed();
    }

    /// Mark as failed
    pub fn fail(&mut self, error: impl Into<String>) {
        self.state = SandboxState::Failed;
        self.completed_at = Some(Utc::now());
        self.exit_message = Some(error.into());
        self.update_elapsed();
    }

    /// Mark as merged
    pub fn mark_merged(&mut self) {
        self.state = SandboxState::Merged;
    }

    /// Mark as discarded
    pub fn discard(&mut self) {
        self.state = SandboxState::Discarded;
    }

    /// Record a file modification
    pub fn record_file_modified(&mut self, path: impl Into<PathBuf>) {
        self.modified_files.push(path.into());
        self.resource_usage.files_modified += 1;
    }

    /// Record a file creation
    pub fn record_file_created(&mut self, path: impl Into<PathBuf>) {
        self.modified_files.push(path.into());
        self.resource_usage.files_created += 1;
    }

    /// Record a command execution
    pub fn record_command(&mut self) {
        self.resource_usage.commands_executed += 1;
    }

    /// Check if resource limits are exceeded
    pub fn check_limits(&self) -> Result<(), String> {
        if self.config.max_duration_secs > 0 {
            if let Some(started) = self.started_at {
                let elapsed = (Utc::now() - started).num_seconds() as u64;
                if elapsed > self.config.max_duration_secs {
                    return Err(format!(
                        "Duration limit exceeded: {}s > {}s",
                        elapsed, self.config.max_duration_secs
                    ));
                }
            }
        }

        if self.config.max_memory_bytes > 0 && self.resource_usage.peak_memory_bytes > self.config.max_memory_bytes {
            return Err(format!(
                "Memory limit exceeded: {} > {}",
                self.resource_usage.peak_memory_bytes, self.config.max_memory_bytes
            ));
        }

        if self.config.max_disk_bytes > 0 && self.resource_usage.disk_bytes_written > self.config.max_disk_bytes {
            return Err(format!(
                "Disk limit exceeded: {} > {}",
                self.resource_usage.disk_bytes_written, self.config.max_disk_bytes
            ));
        }

        Ok(())
    }

    /// Get the sandbox's working directory
    pub fn work_dir(&self) -> &Path {
        &self.work_dir
    }

    /// Elapsed time in ms (or 0 if not started)
    pub fn elapsed_ms(&self) -> u64 {
        match (self.started_at, self.completed_at) {
            (Some(s), Some(c)) => (c - s).num_milliseconds() as u64,
            (Some(s), None) => (Utc::now() - s).num_milliseconds() as u64,
            _ => 0,
        }
    }

    fn update_elapsed(&mut self) {
        self.resource_usage.elapsed_ms = self.elapsed_ms();
    }
}

// =============================================================================
// SANDBOX MANAGER — manages multiple sandboxes for parallel execution
// =============================================================================

/// Manages a pool of sandboxes for parallel agent execution
#[derive(Debug)]
pub struct SandboxManager {
    /// All sandboxes
    sandboxes: Vec<Sandbox>,
    /// Default config for new sandboxes
    default_config: SandboxConfig,
    /// Maximum concurrent sandboxes
    pub max_concurrent: usize,
}

impl SandboxManager {
    pub fn new(default_config: SandboxConfig) -> Self {
        Self {
            sandboxes: Vec::new(),
            default_config,
            max_concurrent: 4,
        }
    }

    /// Create a new sandbox
    pub fn create(&mut self, name: impl Into<String>) -> Result<Uuid, String> {
        let running = self.sandboxes.iter().filter(|s| s.state == SandboxState::Running).count();
        if running >= self.max_concurrent {
            return Err(format!("Max concurrent sandboxes reached: {}", self.max_concurrent));
        }

        let sandbox = Sandbox::new(name, self.default_config.clone());
        let id = sandbox.id;
        self.sandboxes.push(sandbox);
        Ok(id)
    }

    /// Create a sandbox with custom config
    pub fn create_with_config(&mut self, name: impl Into<String>, config: SandboxConfig) -> Result<Uuid, String> {
        let running = self.sandboxes.iter().filter(|s| s.state == SandboxState::Running).count();
        if running >= self.max_concurrent {
            return Err(format!("Max concurrent sandboxes reached: {}", self.max_concurrent));
        }

        let sandbox = Sandbox::new(name, config);
        let id = sandbox.id;
        self.sandboxes.push(sandbox);
        Ok(id)
    }

    /// Get a sandbox by ID
    pub fn get(&self, id: Uuid) -> Option<&Sandbox> {
        self.sandboxes.iter().find(|s| s.id == id)
    }

    /// Get a mutable sandbox by ID
    pub fn get_mut(&mut self, id: Uuid) -> Option<&mut Sandbox> {
        self.sandboxes.iter_mut().find(|s| s.id == id)
    }

    /// Start a sandbox
    pub fn start(&mut self, id: Uuid, agent_id: impl Into<String>) -> Result<(), String> {
        let sandbox = self.get_mut(id).ok_or("Sandbox not found")?;
        if sandbox.state != SandboxState::Ready {
            return Err(format!("Sandbox is not in Ready state: {}", sandbox.state.label()));
        }
        sandbox.start(agent_id);
        Ok(())
    }

    /// Complete a sandbox
    pub fn complete(&mut self, id: Uuid, message: impl Into<String>) -> Result<(), String> {
        let sandbox = self.get_mut(id).ok_or("Sandbox not found")?;
        if sandbox.state != SandboxState::Running {
            return Err(format!("Sandbox is not Running: {}", sandbox.state.label()));
        }
        sandbox.complete(message);
        Ok(())
    }

    /// Fail a sandbox
    pub fn fail(&mut self, id: Uuid, error: impl Into<String>) -> Result<(), String> {
        let sandbox = self.get_mut(id).ok_or("Sandbox not found")?;
        if sandbox.state != SandboxState::Running {
            return Err(format!("Sandbox is not Running: {}", sandbox.state.label()));
        }
        sandbox.fail(error);
        Ok(())
    }

    /// Discard a sandbox (cleanup)
    pub fn discard(&mut self, id: Uuid) -> Result<(), String> {
        let sandbox = self.get_mut(id).ok_or("Sandbox not found")?;
        sandbox.discard();
        Ok(())
    }

    /// Get all active (non-terminal) sandboxes
    pub fn active(&self) -> Vec<&Sandbox> {
        self.sandboxes.iter().filter(|s| !s.state.is_terminal()).collect()
    }

    /// Get all running sandboxes
    pub fn running(&self) -> Vec<&Sandbox> {
        self.sandboxes.iter().filter(|s| s.state == SandboxState::Running).collect()
    }

    /// Get all completed sandboxes (ready for merge)
    pub fn completed(&self) -> Vec<&Sandbox> {
        self.sandboxes.iter().filter(|s| s.state == SandboxState::Completed).collect()
    }

    /// Total sandboxes
    pub fn total(&self) -> usize {
        self.sandboxes.len()
    }

    /// Summary for display
    pub fn summary(&self) -> SandboxManagerSummary {
        SandboxManagerSummary {
            total: self.sandboxes.len(),
            ready: self.sandboxes.iter().filter(|s| s.state == SandboxState::Ready).count(),
            running: self.sandboxes.iter().filter(|s| s.state == SandboxState::Running).count(),
            completed: self.sandboxes.iter().filter(|s| s.state == SandboxState::Completed).count(),
            failed: self.sandboxes.iter().filter(|s| s.state == SandboxState::Failed).count(),
            merged: self.sandboxes.iter().filter(|s| s.state == SandboxState::Merged).count(),
            discarded: self.sandboxes.iter().filter(|s| s.state == SandboxState::Discarded).count(),
            max_concurrent: self.max_concurrent,
        }
    }
}

/// Summary of SandboxManager state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxManagerSummary {
    pub total: usize,
    pub ready: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub merged: usize,
    pub discarded: usize,
    pub max_concurrent: usize,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_config_default() {
        let config = SandboxConfig::default();
        assert!(!config.allow_network);
        assert!(config.allow_exec);
        assert_eq!(config.max_duration_secs, 300);
    }

    #[test]
    fn test_sandbox_config_sovereign() {
        let config = SandboxConfig::sovereign();
        assert!(!config.allow_network);
        assert_eq!(config.max_duration_secs, 120);
        assert_eq!(config.env_vars.get("APOPHY_SOVEREIGN"), Some(&"true".to_string()));
    }

    #[test]
    fn test_sandbox_config_permissive() {
        let config = SandboxConfig::permissive();
        assert!(config.allow_network);
        assert_eq!(config.max_memory_bytes, 0);
    }

    #[test]
    fn test_sandbox_lifecycle() {
        let mut sb = Sandbox::new("test-sandbox", SandboxConfig::default());
        assert_eq!(sb.state, SandboxState::Ready);
        assert!(sb.started_at.is_none());

        sb.start("agent-1");
        assert_eq!(sb.state, SandboxState::Running);
        assert!(sb.started_at.is_some());
        assert_eq!(sb.agent_id, Some("agent-1".into()));

        sb.complete("All tests passed");
        assert_eq!(sb.state, SandboxState::Completed);
        assert!(sb.completed_at.is_some());
        assert_eq!(sb.exit_message, Some("All tests passed".into()));

        sb.mark_merged();
        assert_eq!(sb.state, SandboxState::Merged);
        assert!(sb.state.is_terminal());
    }

    #[test]
    fn test_sandbox_failure() {
        let mut sb = Sandbox::new("fail-sandbox", SandboxConfig::default());
        sb.start("agent-2");
        sb.fail("Compilation error");

        assert_eq!(sb.state, SandboxState::Failed);
        assert!(sb.exit_message.unwrap().contains("Compilation"));
    }

    #[test]
    fn test_sandbox_discard() {
        let mut sb = Sandbox::new("discard", SandboxConfig::default());
        sb.discard();
        assert_eq!(sb.state, SandboxState::Discarded);
        assert!(sb.state.is_terminal());
    }

    #[test]
    fn test_sandbox_file_tracking() {
        let mut sb = Sandbox::new("track", SandboxConfig::default());
        sb.record_file_created("src/new.rs");
        sb.record_file_modified("src/lib.rs");

        assert_eq!(sb.modified_files.len(), 2);
        assert_eq!(sb.resource_usage.files_created, 1);
        assert_eq!(sb.resource_usage.files_modified, 1);
    }

    #[test]
    fn test_sandbox_command_tracking() {
        let mut sb = Sandbox::new("cmds", SandboxConfig::default());
        sb.record_command();
        sb.record_command();
        sb.record_command();

        assert_eq!(sb.resource_usage.commands_executed, 3);
    }

    #[test]
    fn test_sandbox_resource_limits_ok() {
        let sb = Sandbox::new("limits-ok", SandboxConfig::default());
        assert!(sb.check_limits().is_ok());
    }

    #[test]
    fn test_sandbox_memory_limit_exceeded() {
        let mut sb = Sandbox::new("mem-limit", SandboxConfig::default());
        sb.resource_usage.peak_memory_bytes = 1024 * 1024 * 1024; // 1GB > 512MB limit
        assert!(sb.check_limits().is_err());
    }

    #[test]
    fn test_sandbox_disk_limit_exceeded() {
        let mut sb = Sandbox::new("disk-limit", SandboxConfig::default());
        sb.resource_usage.disk_bytes_written = 2 * 1024 * 1024 * 1024; // 2GB > 1GB limit
        assert!(sb.check_limits().is_err());
    }

    #[test]
    fn test_sandbox_work_dir() {
        let config = SandboxConfig::default();
        let sb = Sandbox::new("workdir-test", config);
        assert!(sb.work_dir().starts_with("/tmp/apophy-sandboxes"));
    }

    #[test]
    fn test_sandbox_manager_create() {
        let mut mgr = SandboxManager::new(SandboxConfig::sovereign());
        let id = mgr.create("sandbox-1").unwrap();

        assert_eq!(mgr.total(), 1);
        assert!(mgr.get(id).is_some());
        assert_eq!(mgr.get(id).unwrap().state, SandboxState::Ready);
    }

    #[test]
    fn test_sandbox_manager_lifecycle() {
        let mut mgr = SandboxManager::new(SandboxConfig::default());
        let id = mgr.create("test").unwrap();

        mgr.start(id, "worker-agent").unwrap();
        assert_eq!(mgr.running().len(), 1);

        mgr.complete(id, "Done").unwrap();
        assert_eq!(mgr.completed().len(), 1);
        assert_eq!(mgr.running().len(), 0);
    }

    #[test]
    fn test_sandbox_manager_max_concurrent() {
        let mut mgr = SandboxManager::new(SandboxConfig::default());
        mgr.max_concurrent = 2;

        let id1 = mgr.create("sb-1").unwrap();
        let id2 = mgr.create("sb-2").unwrap();

        mgr.start(id1, "a1").unwrap();
        mgr.start(id2, "a2").unwrap();

        // 2 running = max_concurrent reached. Creating a new sandbox should fail.
        let result = mgr.create("sb-3");
        assert!(result.is_err());

        // Complete one, now we can create again
        mgr.complete(id1, "done").unwrap();
        let _id3 = mgr.create("sb-3").unwrap();
        assert_eq!(mgr.total(), 3);
    }

    #[test]
    fn test_sandbox_manager_fail_and_discard() {
        let mut mgr = SandboxManager::new(SandboxConfig::default());
        let id = mgr.create("fail-test").unwrap();

        mgr.start(id, "agent").unwrap();
        mgr.fail(id, "Crash").unwrap();

        assert_eq!(mgr.summary().failed, 1);

        mgr.discard(id).unwrap();
        assert_eq!(mgr.summary().discarded, 1);
    }

    #[test]
    fn test_sandbox_manager_summary() {
        let mut mgr = SandboxManager::new(SandboxConfig::default());
        mgr.max_concurrent = 10;

        let id1 = mgr.create("sb-1").unwrap();
        let id2 = mgr.create("sb-2").unwrap();
        let _id3 = mgr.create("sb-3").unwrap();

        mgr.start(id1, "a").unwrap();
        mgr.complete(id1, "ok").unwrap();
        mgr.start(id2, "b").unwrap();

        let summary = mgr.summary();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.completed, 1);
        assert_eq!(summary.running, 1);
        assert_eq!(summary.ready, 1);
    }

    #[test]
    fn test_sandbox_manager_active() {
        let mut mgr = SandboxManager::new(SandboxConfig::default());
        mgr.max_concurrent = 10;

        let id1 = mgr.create("sb-1").unwrap();
        let id2 = mgr.create("sb-2").unwrap();

        mgr.start(id1, "a").unwrap();
        mgr.complete(id1, "done").unwrap();
        mgr.get_mut(id1).unwrap().mark_merged();

        // id1 is merged (terminal), id2 is ready (non-terminal)
        assert_eq!(mgr.active().len(), 1);
        assert_eq!(mgr.active()[0].id, id2);
    }

    #[test]
    fn test_sandbox_manager_custom_config() {
        let mut mgr = SandboxManager::new(SandboxConfig::default());
        let custom = SandboxConfig::permissive();
        let id = mgr.create_with_config("custom", custom).unwrap();

        let sb = mgr.get(id).unwrap();
        assert!(sb.config.allow_network); // Custom config applied
    }

    #[test]
    fn test_sandbox_state_labels() {
        assert_eq!(SandboxState::Ready.label(), "READY");
        assert_eq!(SandboxState::Running.label(), "RUNNING");
        assert_eq!(SandboxState::Completed.label(), "COMPLETED");
        assert_eq!(SandboxState::Failed.label(), "FAILED");
        assert_eq!(SandboxState::Merged.label(), "MERGED");
        assert_eq!(SandboxState::Discarded.label(), "DISCARDED");
    }

    #[test]
    fn test_sandbox_state_terminal() {
        assert!(!SandboxState::Ready.is_terminal());
        assert!(!SandboxState::Running.is_terminal());
        assert!(!SandboxState::Completed.is_terminal());
        assert!(!SandboxState::Failed.is_terminal());
        assert!(SandboxState::Merged.is_terminal());
        assert!(SandboxState::Discarded.is_terminal());
    }

    #[test]
    fn test_sandbox_elapsed_not_started() {
        let sb = Sandbox::new("not-started", SandboxConfig::default());
        assert_eq!(sb.elapsed_ms(), 0);
    }
}
