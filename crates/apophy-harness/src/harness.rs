//! # Harness — The complete Initializer/Worker orchestration layer
//!
//! Ties everything together: bootstraps domain memory via Initializer,
//! then runs Workers in a loop until the project is complete.
//!
//! Also provides persistence: serialize/deserialize domain memory to JSON
//! so the harness survives process restarts.

use crate::domain::*;
use crate::initializer::Initializer;
use crate::worker::{FeatureExecutor, TestRunner, Worker, WorkerResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HarnessError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Harness stopped: {0}")]
    Stopped(String),
}

pub type Result<T> = std::result::Result<T, HarnessError>;

/// Configuration for the Harness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessConfig {
    /// Where to persist domain memory
    pub memory_path: PathBuf,
    /// Whether to load existing memory on startup
    pub resume: bool,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            memory_path: PathBuf::from("/opt/apophy/domain_memory.json"),
            resume: true,
        }
    }
}

/// The Harness — orchestrates Initializer + Worker with persistence
pub struct Harness {
    config: HarnessConfig,
}

impl Harness {
    pub fn new(config: HarnessConfig) -> Self {
        Self { config }
    }

    /// Load domain memory from disk (if exists and resume=true)
    pub fn load_memory(&self) -> Result<Option<DomainMemory>> {
        if !self.config.resume || !self.config.memory_path.exists() {
            return Ok(None);
        }

        let data = std::fs::read_to_string(&self.config.memory_path)?;
        let memory: DomainMemory = serde_json::from_str(&data)?;
        Ok(Some(memory))
    }

    /// Save domain memory to disk (atomic write)
    pub fn save_memory(&self, memory: &DomainMemory) -> Result<()> {
        if let Some(parent) = self.config.memory_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(memory)?;

        // Atomic write: write to temp then rename
        let tmp_path = self.config.memory_path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &json)?;
        std::fs::rename(&tmp_path, &self.config.memory_path)?;

        Ok(())
    }

    /// Initialize or resume domain memory
    pub fn initialize_or_resume(&self, fresh_memory: DomainMemory) -> Result<DomainMemory> {
        match self.load_memory()? {
            Some(existing) => {
                tracing::info!(
                    "Resumed domain memory: {}/{} features passed, {} runs",
                    existing.backlog.passed(),
                    existing.backlog.total(),
                    existing.progress.total_runs()
                );
                Ok(existing)
            }
            None => {
                tracing::info!(
                    "Fresh domain memory: {} features to complete",
                    fresh_memory.backlog.total()
                );
                self.save_memory(&fresh_memory)?;
                Ok(fresh_memory)
            }
        }
    }

    /// Run ONE worker cycle, persist, return result
    pub fn run_once<E: FeatureExecutor, T: TestRunner>(
        &self,
        memory: &mut DomainMemory,
        executor: E,
        test_runner: T,
    ) -> Result<WorkerResult> {
        let worker = Worker::new(executor, test_runner);
        let result = worker.run(memory);

        // Always persist after each run
        self.save_memory(memory)?;

        tracing::info!(
            "Worker run complete: {} | {}/{} passed",
            result.outcome.label(),
            memory.backlog.passed(),
            memory.backlog.total()
        );

        Ok(result)
    }

    /// Run workers until project is complete or stopped
    pub fn run_until_done<E: FeatureExecutor, T: TestRunner>(
        &self,
        memory: &mut DomainMemory,
        executor: E,
        test_runner: T,
    ) -> Result<Vec<WorkerResult>> {
        let worker = Worker::new(executor, test_runner);
        let mut results = Vec::new();

        loop {
            if memory.should_stop() {
                break;
            }

            let result = worker.run(memory);
            self.save_memory(memory)?;

            let is_done = result.outcome == RunOutcome::NothingToDo;
            results.push(result);

            if is_done {
                break;
            }
        }

        Ok(results)
    }

    /// Get a human-readable status report
    pub fn status(&self) -> Result<String> {
        match self.load_memory()? {
            Some(memory) => Ok(memory.summarize()),
            None => Ok("No domain memory found. Run initializer first.".to_string()),
        }
    }

    /// Create the sovereign infrastructure template and persist it
    pub fn bootstrap_sovereign(&self) -> Result<DomainMemory> {
        let memory = Initializer::sovereign_infrastructure();
        self.save_memory(&memory)?;
        Ok(memory)
    }

    /// Get the memory path
    pub fn memory_path(&self) -> &Path {
        &self.config.memory_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worker::{FeatureExecutor, TestRunner};
    use tempfile::TempDir;

    struct AlwaysSucceeds;
    impl FeatureExecutor for AlwaysSucceeds {
        fn execute(&self, f: &Feature, _: &[String]) -> std::result::Result<Vec<String>, String> {
            Ok(vec![format!("done: {}", f.name)])
        }
    }

    struct AllTestsPass;
    impl TestRunner for AllTestsPass {
        fn run_tests(&self) -> TestSnapshot {
            TestSnapshot::new(5, 5, 0)
        }
    }

    fn test_harness() -> (Harness, TempDir) {
        let dir = TempDir::new().unwrap();
        let config = HarnessConfig {
            memory_path: dir.path().join("memory.json"),
            resume: true,
        };
        (Harness::new(config), dir)
    }

    fn simple_memory() -> DomainMemory {
        let mut backlog = FeatureBacklog::new("test");
        backlog.add_feature(Feature::new("A", 1));
        backlog.add_feature(Feature::new("B", 2));
        DomainMemory::new(backlog, ScaffoldingRules::default())
    }

    #[test]
    fn test_save_and_load() {
        let (harness, _dir) = test_harness();
        let memory = simple_memory();

        harness.save_memory(&memory).unwrap();
        let loaded = harness.load_memory().unwrap().unwrap();

        assert_eq!(loaded.backlog.project_name, "test");
        assert_eq!(loaded.backlog.total(), 2);
    }

    #[test]
    fn test_load_nonexistent() {
        let (harness, _dir) = test_harness();
        let loaded = harness.load_memory().unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_initialize_or_resume_fresh() {
        let (harness, _dir) = test_harness();
        let memory = harness.initialize_or_resume(simple_memory()).unwrap();
        assert_eq!(memory.backlog.total(), 2);
        // File should now exist
        assert!(harness.memory_path().exists());
    }

    #[test]
    fn test_initialize_or_resume_existing() {
        let (harness, _dir) = test_harness();

        // Save initial state
        let mut memory = simple_memory();
        memory.backlog.features[0].mark_passed();
        memory.update_state();
        harness.save_memory(&memory).unwrap();

        // Resume should pick up existing state
        let resumed = harness.initialize_or_resume(simple_memory()).unwrap();
        assert_eq!(resumed.backlog.passed(), 1); // Preserved!
    }

    #[test]
    fn test_run_once() {
        let (harness, _dir) = test_harness();
        let mut memory = simple_memory();

        let result = harness.run_once(&mut memory, AlwaysSucceeds, AllTestsPass).unwrap();
        assert_eq!(result.outcome, RunOutcome::FeaturePassed);
        assert_eq!(memory.backlog.passed(), 1);

        // Memory should be persisted
        let loaded = harness.load_memory().unwrap().unwrap();
        assert_eq!(loaded.backlog.passed(), 1);
    }

    #[test]
    fn test_run_until_done() {
        let (harness, _dir) = test_harness();
        let mut memory = simple_memory();

        let results = harness.run_until_done(&mut memory, AlwaysSucceeds, AllTestsPass).unwrap();
        assert!(results.len() >= 2);
        assert!(memory.backlog.is_complete());

        // Final state persisted
        let loaded = harness.load_memory().unwrap().unwrap();
        assert!(loaded.backlog.is_complete());
    }

    #[test]
    fn test_status_no_memory() {
        let (harness, _dir) = test_harness();
        let status = harness.status().unwrap();
        assert!(status.contains("No domain memory found"));
    }

    #[test]
    fn test_status_with_memory() {
        let (harness, _dir) = test_harness();
        harness.save_memory(&simple_memory()).unwrap();
        let status = harness.status().unwrap();
        assert!(status.contains("DOMAIN MEMORY"));
    }

    #[test]
    fn test_bootstrap_sovereign() {
        let (harness, _dir) = test_harness();
        let memory = harness.bootstrap_sovereign().unwrap();

        assert_eq!(memory.backlog.project_name, "Apophy Sovereign Infrastructure");
        assert_eq!(memory.backlog.total(), 8);
        assert!(harness.memory_path().exists());
    }

    #[test]
    fn test_persistence_across_runs() {
        let (harness, _dir) = test_harness();
        let mut memory = simple_memory();

        // Run 1
        harness.run_once(&mut memory, AlwaysSucceeds, AllTestsPass).unwrap();
        assert_eq!(memory.progress.total_runs(), 1);

        // Simulate restart: load from disk
        let mut resumed = harness.load_memory().unwrap().unwrap();
        assert_eq!(resumed.progress.total_runs(), 1);
        assert_eq!(resumed.backlog.passed(), 1);

        // Run 2 on resumed state
        harness.run_once(&mut resumed, AlwaysSucceeds, AllTestsPass).unwrap();
        assert_eq!(resumed.progress.total_runs(), 2);
        assert_eq!(resumed.backlog.passed(), 2);
    }
}
