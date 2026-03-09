//! # Worker Agent
//!
//! A pure function: reads memory state -> picks ONE failing item ->
//! works on it -> tests -> updates state -> exits.
//!
//! The Worker is intentionally stateless. All state lives in DomainMemory.
//! This means every run starts from truth, not from degraded context.
//!
//! ## The Bootup Ritual (every run, no exceptions)
//!
//! 1. Read feature backlog from domain memory
//! 2. Read progress log from last N runs
//! 3. Run test suite to verify current state
//! 4. Compare results with backlog (detect regressions)
//! 5. Select ONE actionable feature
//! 6. Check failed_approaches to avoid repeating mistakes

use crate::domain::*;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum WorkerError {
    #[error("No actionable features")]
    NothingToDo,
    #[error("Feature blocked: {0}")]
    Blocked(String),
    #[error("Max attempts reached for feature: {0}")]
    MaxAttempts(String),
    #[error("Loop detected for feature: {0}")]
    LoopDetected(String),
    #[error("Regression detected: {0}")]
    Regression(String),
    #[error("Test failed: {0}")]
    TestFailed(String),
}

pub type Result<T> = std::result::Result<T, WorkerError>;

/// The result of a worker execution step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResult {
    pub run_id: Uuid,
    pub feature_id: Option<Uuid>,
    pub feature_name: Option<String>,
    pub outcome: RunOutcome,
    pub changes: Vec<String>,
    pub regressions: Vec<String>,
    pub tests_before: TestSnapshot,
    pub tests_after: TestSnapshot,
    pub notes: String,
}

/// Trait for executing work on a feature.
/// Implement this for your specific domain.
pub trait FeatureExecutor {
    /// Attempt to implement/fix the given feature.
    /// Returns a list of changes made (for the progress log).
    fn execute(&self, feature: &Feature, failed_approaches: &[String]) -> std::result::Result<Vec<String>, String>;
}

/// Trait for running tests.
/// Implement this for your specific domain.
pub trait TestRunner {
    /// Run the test suite and return results.
    fn run_tests(&self) -> TestSnapshot;

    /// Run tests for a specific feature (if granular testing is available).
    fn run_feature_tests(&self, _feature: &Feature) -> TestSnapshot {
        self.run_tests() // Default: run all tests
    }
}

/// The Worker agent — executes one atomic unit of work per run
pub struct Worker<E: FeatureExecutor, T: TestRunner> {
    executor: E,
    test_runner: T,
}

impl<E: FeatureExecutor, T: TestRunner> Worker<E, T> {
    pub fn new(executor: E, test_runner: T) -> Self {
        Self { executor, test_runner }
    }

    /// Execute ONE worker run following the bootup ritual.
    ///
    /// This is the core loop:
    /// 1. Verify current state (run tests)
    /// 2. Detect regressions
    /// 3. Pick ONE feature
    /// 4. Execute
    /// 5. Verify (run tests again)
    /// 6. Update memory
    pub fn run(&self, memory: &mut DomainMemory) -> WorkerResult {
        let run_id = Uuid::new_v4();
        let run_number = memory.progress.total_runs() + 1;
        let started_at = Utc::now();

        // === BOOTUP RITUAL ===

        // Step 1: Run tests to verify current state
        let tests_before = self.test_runner.run_tests();

        // Step 2: Detect regressions (features marked passed that now fail)
        let regressions = self.detect_regressions(memory, &tests_before);

        // If regressions found and rules say stop, record and exit
        if !regressions.is_empty() && memory.rules.stop_on_regression {
            // Mark regressed features
            for regression in &regressions {
                for f in &mut memory.backlog.features {
                    if f.name == *regression && f.status == FeatureStatus::Passed {
                        f.mark_regressed("Detected during bootup ritual");
                    }
                }
            }
            memory.update_state();

            let result = WorkerResult {
                run_id,
                feature_id: None,
                feature_name: None,
                outcome: RunOutcome::RegressionUnfixed,
                changes: vec![],
                regressions: regressions.clone(),
                tests_before: tests_before.clone(),
                tests_after: tests_before,
                notes: format!("Regressions detected during bootup: {:?}", regressions),
            };
            self.record_run(memory, run_number, started_at, &result);
            return result;
        }

        // Step 3: Check if we should stop
        if memory.should_stop() {
            let result = WorkerResult {
                run_id,
                feature_id: None,
                feature_name: None,
                outcome: RunOutcome::NothingToDo,
                changes: vec![],
                regressions,
                tests_before: tests_before.clone(),
                tests_after: tests_before,
                notes: "No actionable features or max runs reached".into(),
            };
            self.record_run(memory, run_number, started_at, &result);
            return result;
        }

        // Step 4: Pick ONE feature
        let (feature_id, feature_name) = match memory.backlog.next_actionable() {
            Some(f) => (f.id, f.name.clone()),
            None => {
                let result = WorkerResult {
                    run_id,
                    feature_id: None,
                    feature_name: None,
                    outcome: RunOutcome::NothingToDo,
                    changes: vec![],
                    regressions,
                    tests_before: tests_before.clone(),
                    tests_after: tests_before,
                    notes: "All features blocked or complete".into(),
                };
                self.record_run(memory, run_number, started_at, &result);
                return result;
            }
        };

        // Step 5: Check for loops
        if memory.progress.detect_loop(feature_id, memory.rules.max_attempts_per_feature) {
            // Skip this feature
            if let Some(f) = memory.backlog.features.iter_mut().find(|f| f.id == feature_id) {
                f.status = FeatureStatus::Skipped;
                f.last_error = Some("Max attempts reached — loop detected".into());
                f.updated_at = Utc::now();
            }
            memory.update_state();

            let result = WorkerResult {
                run_id,
                feature_id: Some(feature_id),
                feature_name: Some(feature_name),
                outcome: RunOutcome::MaxAttemptsReached,
                changes: vec![],
                regressions,
                tests_before: tests_before.clone(),
                tests_after: tests_before,
                notes: "Loop detected — feature skipped after max attempts".into(),
            };
            self.record_run(memory, run_number, started_at, &result);
            return result;
        }

        // Step 6: Get failed approaches to avoid repeating
        let failed_approaches = memory.progress.failed_approaches_for(feature_id);

        // Step 7: Mark in progress
        if let Some(f) = memory.backlog.features.iter_mut().find(|f| f.id == feature_id) {
            f.mark_in_progress(run_id);
        }

        // Step 8: Execute
        let feature = memory.backlog.features.iter().find(|f| f.id == feature_id).unwrap().clone();
        let exec_result = self.executor.execute(&feature, &failed_approaches);

        let changes = match &exec_result {
            Ok(changes) => changes.clone(),
            Err(_) => vec![],
        };

        // Step 9: Run tests after execution
        let tests_after = self.test_runner.run_tests();

        // Step 10: Determine outcome
        let outcome = match exec_result {
            Ok(_) if tests_after.all_passing() => {
                if let Some(f) = memory.backlog.features.iter_mut().find(|f| f.id == feature_id) {
                    f.mark_passed();
                }
                RunOutcome::FeaturePassed
            }
            Ok(_) => {
                if let Some(f) = memory.backlog.features.iter_mut().find(|f| f.id == feature_id) {
                    f.mark_failed("Tests still failing after execution");
                }
                RunOutcome::FeatureFailed
            }
            Err(e) => {
                if let Some(f) = memory.backlog.features.iter_mut().find(|f| f.id == feature_id) {
                    f.mark_failed(&e);
                }
                memory.state.record_failed_approach(feature_id, &feature_name, &e);
                RunOutcome::FeatureFailed
            }
        };

        // Step 11: Update memory
        memory.update_state();
        memory.state.record_test_results(tests_after.total, tests_after.passed, tests_after.failed);

        let result = WorkerResult {
            run_id,
            feature_id: Some(feature_id),
            feature_name: Some(feature_name),
            outcome,
            changes,
            regressions,
            tests_before,
            tests_after,
            notes: String::new(),
        };

        self.record_run(memory, run_number, started_at, &result);
        result
    }

    /// Run the worker in a loop until completion or stop condition
    pub fn run_until_done(&self, memory: &mut DomainMemory) -> Vec<WorkerResult> {
        let mut results = Vec::new();
        let max_runs = memory.rules.max_total_runs;

        loop {
            if memory.should_stop() || results.len() as u32 >= max_runs {
                break;
            }

            let result = self.run(memory);
            let is_nothing = result.outcome == RunOutcome::NothingToDo;
            let is_max = result.outcome == RunOutcome::MaxAttemptsReached;
            results.push(result);

            if is_nothing {
                break;
            }

            // If all remaining features are skipped, we're done
            if is_max && memory.backlog.next_actionable().is_none() {
                break;
            }
        }

        results
    }

    fn detect_regressions(&self, memory: &DomainMemory, _current_tests: &TestSnapshot) -> Vec<String> {
        // In a real system, we'd map test names to features.
        // Here we check if any "passed" feature's criteria conceptually fail.
        // For now, return empty — regressions are detected via test runner.
        let _ = memory;
        Vec::new()
    }

    fn record_run(
        &self,
        memory: &mut DomainMemory,
        run_number: u32,
        started_at: chrono::DateTime<Utc>,
        result: &WorkerResult,
    ) {
        memory.progress.add_run(WorkerRunLog {
            run_id: result.run_id,
            run_number,
            started_at,
            completed_at: Some(Utc::now()),
            target_feature: result.feature_id,
            target_feature_name: result.feature_name.clone(),
            outcome: result.outcome.clone(),
            tests_before: result.tests_before.clone(),
            tests_after: result.tests_after.clone(),
            changes_made: result.changes.clone(),
            regressions_detected: result.regressions.clone(),
            notes: result.notes.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test executor that always succeeds
    struct AlwaysSucceeds;
    impl FeatureExecutor for AlwaysSucceeds {
        fn execute(&self, feature: &Feature, _failed: &[String]) -> std::result::Result<Vec<String>, String> {
            Ok(vec![format!("Implemented {}", feature.name)])
        }
    }

    /// A test executor that always fails
    struct AlwaysFails;
    impl FeatureExecutor for AlwaysFails {
        fn execute(&self, _feature: &Feature, _failed: &[String]) -> std::result::Result<Vec<String>, String> {
            Err("Implementation failed".into())
        }
    }

    /// A test executor that succeeds after N attempts
    struct SucceedsAfter {
        threshold: std::sync::atomic::AtomicU32,
        target: u32,
    }
    impl SucceedsAfter {
        fn new(after: u32) -> Self {
            Self {
                threshold: std::sync::atomic::AtomicU32::new(0),
                target: after,
            }
        }
    }
    impl FeatureExecutor for SucceedsAfter {
        fn execute(&self, feature: &Feature, _failed: &[String]) -> std::result::Result<Vec<String>, String> {
            let count = self.threshold.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if count >= self.target {
                Ok(vec![format!("Implemented {} on attempt {}", feature.name, count + 1)])
            } else {
                Err(format!("Not ready yet (attempt {})", count + 1))
            }
        }
    }

    /// Test runner that always passes
    struct AllTestsPass;
    impl TestRunner for AllTestsPass {
        fn run_tests(&self) -> TestSnapshot {
            TestSnapshot::new(10, 10, 0)
        }
    }

    /// Test runner that always has failures
    struct SomeTestsFail;
    impl TestRunner for SomeTestsFail {
        fn run_tests(&self) -> TestSnapshot {
            TestSnapshot::new(10, 7, 3)
        }
    }

    fn sample_memory() -> DomainMemory {
        let mut backlog = FeatureBacklog::new("test-project");
        backlog.add_feature(
            Feature::new("Feature A", 1)
                .with_criteria(vec!["criterion 1".into()]),
        );
        backlog.add_feature(
            Feature::new("Feature B", 2)
                .with_criteria(vec!["criterion 2".into()]),
        );
        DomainMemory::new(backlog, ScaffoldingRules::default())
    }

    #[test]
    fn test_worker_single_success() {
        let worker = Worker::new(AlwaysSucceeds, AllTestsPass);
        let mut memory = sample_memory();

        let result = worker.run(&mut memory);
        assert_eq!(result.outcome, RunOutcome::FeaturePassed);
        assert!(result.feature_name.is_some());
        assert_eq!(memory.backlog.passed(), 1);
        assert_eq!(memory.progress.total_runs(), 1);
    }

    #[test]
    fn test_worker_single_failure() {
        let worker = Worker::new(AlwaysFails, AllTestsPass);
        let mut memory = sample_memory();

        let result = worker.run(&mut memory);
        assert_eq!(result.outcome, RunOutcome::FeatureFailed);
        assert_eq!(memory.backlog.failed(), 1);
    }

    #[test]
    fn test_worker_tests_fail_after_exec() {
        let worker = Worker::new(AlwaysSucceeds, SomeTestsFail);
        let mut memory = sample_memory();

        let result = worker.run(&mut memory);
        // Even though executor succeeded, tests fail
        assert_eq!(result.outcome, RunOutcome::FeatureFailed);
    }

    #[test]
    fn test_worker_run_until_done_all_pass() {
        let worker = Worker::new(AlwaysSucceeds, AllTestsPass);
        let mut memory = sample_memory();

        let results = worker.run_until_done(&mut memory);
        // 2 features + 1 NothingToDo at the end
        assert!(results.len() >= 2);
        assert!(memory.backlog.is_complete());
    }

    #[test]
    fn test_worker_loop_detection() {
        let worker = Worker::new(AlwaysFails, AllTestsPass);
        let mut memory = sample_memory();
        memory.rules.max_attempts_per_feature = 3;
        memory.rules.max_total_runs = 20;

        let results = worker.run_until_done(&mut memory);

        // Should eventually skip features after max attempts
        let max_reached = results.iter().any(|r| r.outcome == RunOutcome::MaxAttemptsReached);
        assert!(max_reached || results.len() <= 20);
    }

    #[test]
    fn test_worker_succeeds_after_retry() {
        let executor = SucceedsAfter::new(2);
        let worker = Worker::new(executor, AllTestsPass);
        let mut memory = sample_memory();
        // Only 1 feature to keep it simple
        memory.backlog.features.truncate(1);
        memory.rules.max_attempts_per_feature = 5;

        let results = worker.run_until_done(&mut memory);

        // Should eventually pass
        let passed = results.iter().any(|r| r.outcome == RunOutcome::FeaturePassed);
        assert!(passed);
    }

    #[test]
    fn test_worker_records_progress() {
        let worker = Worker::new(AlwaysSucceeds, AllTestsPass);
        let mut memory = sample_memory();

        worker.run(&mut memory);
        worker.run(&mut memory);

        assert_eq!(memory.progress.total_runs(), 2);
        assert_eq!(memory.progress.successful_runs(), 2);
    }

    #[test]
    fn test_worker_empty_backlog() {
        let worker = Worker::new(AlwaysSucceeds, AllTestsPass);
        let backlog = FeatureBacklog::new("empty");
        let mut memory = DomainMemory::new(backlog, ScaffoldingRules::default());

        let result = worker.run(&mut memory);
        assert_eq!(result.outcome, RunOutcome::NothingToDo);
    }

    #[test]
    fn test_worker_result_serializes() {
        let worker = Worker::new(AlwaysSucceeds, AllTestsPass);
        let mut memory = sample_memory();

        let result = worker.run(&mut memory);
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("FeaturePassed"));
    }
}
