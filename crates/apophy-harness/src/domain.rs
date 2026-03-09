//! Domain Memory Schema — The 4 pillars of agent memory
//!
//! These are the externalized, persistent artifacts that make agents work.
//! Without them, agents are "an infinite sequence of disconnected interns."

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

// =============================================================================
// PILLAR 1: GOALS & REQUIREMENTS
// =============================================================================

/// A single testable feature/goal with pass/fail status.
/// The atomic unit of the backlog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    /// Unique identifier
    pub id: Uuid,
    /// Human-readable name (e.g. "User authentication via SSH keys")
    pub name: String,
    /// Machine-readable description of what "done" means
    pub acceptance_criteria: Vec<String>,
    /// Current status
    pub status: FeatureStatus,
    /// Priority (lower = higher priority)
    pub priority: u32,
    /// Which worker run last touched this
    pub last_worker_run: Option<Uuid>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Dependencies: other feature IDs that must pass first
    pub depends_on: Vec<Uuid>,
    /// How many times a worker has attempted this
    pub attempt_count: u32,
    /// Last error if status is Failed
    pub last_error: Option<String>,
    /// Timestamp of last status change
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FeatureStatus {
    /// Not yet attempted
    Pending,
    /// Worker is currently working on it
    InProgress,
    /// All acceptance criteria pass
    Passed,
    /// Attempted but tests fail
    Failed,
    /// Explicitly skipped (blocked or deferred)
    Skipped,
    /// Was passing, now broken by another change
    Regressed,
}

impl FeatureStatus {
    pub fn is_actionable(&self) -> bool {
        matches!(self, FeatureStatus::Pending | FeatureStatus::Failed | FeatureStatus::Regressed)
    }

    pub fn label(&self) -> &'static str {
        match self {
            FeatureStatus::Pending => "PENDING",
            FeatureStatus::InProgress => "IN_PROGRESS",
            FeatureStatus::Passed => "PASSED",
            FeatureStatus::Failed => "FAILED",
            FeatureStatus::Skipped => "SKIPPED",
            FeatureStatus::Regressed => "REGRESSED",
        }
    }
}

impl Feature {
    pub fn new(name: impl Into<String>, priority: u32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            acceptance_criteria: Vec::new(),
            status: FeatureStatus::Pending,
            priority,
            last_worker_run: None,
            tags: Vec::new(),
            depends_on: Vec::new(),
            attempt_count: 0,
            last_error: None,
            updated_at: Utc::now(),
        }
    }

    pub fn with_criteria(mut self, criteria: Vec<String>) -> Self {
        self.acceptance_criteria = criteria;
        self
    }

    pub fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.into_iter().map(String::from).collect();
        self
    }

    pub fn with_dependency(mut self, dep: Uuid) -> Self {
        self.depends_on.push(dep);
        self
    }

    pub fn mark_in_progress(&mut self, worker_run_id: Uuid) {
        self.status = FeatureStatus::InProgress;
        self.last_worker_run = Some(worker_run_id);
        self.attempt_count += 1;
        self.updated_at = Utc::now();
    }

    pub fn mark_passed(&mut self) {
        self.status = FeatureStatus::Passed;
        self.last_error = None;
        self.updated_at = Utc::now();
    }

    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = FeatureStatus::Failed;
        self.last_error = Some(error.into());
        self.updated_at = Utc::now();
    }

    pub fn mark_regressed(&mut self, reason: impl Into<String>) {
        self.status = FeatureStatus::Regressed;
        self.last_error = Some(reason.into());
        self.updated_at = Utc::now();
    }
}

/// The complete feature backlog — single source of truth for what needs doing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureBacklog {
    pub id: Uuid,
    pub project_name: String,
    pub created_at: DateTime<Utc>,
    pub features: Vec<Feature>,
}

impl FeatureBacklog {
    pub fn new(project_name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_name: project_name.into(),
            created_at: Utc::now(),
            features: Vec::new(),
        }
    }

    pub fn add_feature(&mut self, feature: Feature) {
        self.features.push(feature);
    }

    /// Get the next actionable feature (respecting priority and dependencies)
    pub fn next_actionable(&self) -> Option<&Feature> {
        let passed_ids: Vec<Uuid> = self.features
            .iter()
            .filter(|f| f.status == FeatureStatus::Passed)
            .map(|f| f.id)
            .collect();

        self.features
            .iter()
            .filter(|f| f.status.is_actionable())
            .filter(|f| f.depends_on.iter().all(|dep| passed_ids.contains(dep)))
            .min_by_key(|f| (f.priority, f.attempt_count))
    }

    pub fn total(&self) -> usize {
        self.features.len()
    }

    pub fn passed(&self) -> usize {
        self.features.iter().filter(|f| f.status == FeatureStatus::Passed).count()
    }

    pub fn failed(&self) -> usize {
        self.features.iter().filter(|f| f.status == FeatureStatus::Failed).count()
    }

    pub fn pending(&self) -> usize {
        self.features.iter().filter(|f| f.status == FeatureStatus::Pending).count()
    }

    pub fn regressed(&self) -> usize {
        self.features.iter().filter(|f| f.status == FeatureStatus::Regressed).count()
    }

    pub fn progress_percent(&self) -> f64 {
        if self.features.is_empty() {
            return 0.0;
        }
        (self.passed() as f64 / self.total() as f64) * 100.0
    }

    pub fn is_complete(&self) -> bool {
        !self.features.is_empty() && self.features.iter().all(|f| f.status == FeatureStatus::Passed)
    }
}

// =============================================================================
// PILLAR 2: STATE TRACKING
// =============================================================================

/// Current system state — what's passing, failing, broken, tried
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub snapshot_id: Uuid,
    pub timestamp: DateTime<Utc>,
    /// Backlog summary stats
    pub backlog_total: usize,
    pub backlog_passed: usize,
    pub backlog_failed: usize,
    pub backlog_pending: usize,
    pub backlog_regressed: usize,
    /// Test results from last run
    pub tests_total: u32,
    pub tests_passed: u32,
    pub tests_failed: u32,
    /// Current blockers
    pub blockers: Vec<String>,
    /// What's been tried and didn't work (prevent loops)
    pub failed_approaches: Vec<FailedApproach>,
    /// Hash for integrity verification
    pub state_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedApproach {
    pub feature_id: Uuid,
    pub description: String,
    pub error: String,
    pub timestamp: DateTime<Utc>,
}

impl SystemState {
    pub fn from_backlog(backlog: &FeatureBacklog) -> Self {
        let mut state = Self {
            snapshot_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            backlog_total: backlog.total(),
            backlog_passed: backlog.passed(),
            backlog_failed: backlog.failed(),
            backlog_pending: backlog.pending(),
            backlog_regressed: backlog.regressed(),
            tests_total: 0,
            tests_passed: 0,
            tests_failed: 0,
            blockers: Vec::new(),
            failed_approaches: Vec::new(),
            state_hash: String::new(),
        };
        state.state_hash = state.compute_hash();
        state
    }

    pub fn record_test_results(&mut self, total: u32, passed: u32, failed: u32) {
        self.tests_total = total;
        self.tests_passed = passed;
        self.tests_failed = failed;
        self.state_hash = self.compute_hash();
    }

    pub fn add_blocker(&mut self, blocker: impl Into<String>) {
        self.blockers.push(blocker.into());
        self.state_hash = self.compute_hash();
    }

    pub fn record_failed_approach(&mut self, feature_id: Uuid, desc: impl Into<String>, error: impl Into<String>) {
        self.failed_approaches.push(FailedApproach {
            feature_id,
            description: desc.into(),
            error: error.into(),
            timestamp: Utc::now(),
        });
        self.state_hash = self.compute_hash();
    }

    pub fn has_blockers(&self) -> bool {
        !self.blockers.is_empty()
    }

    fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.snapshot_id.as_bytes());
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(self.backlog_total.to_le_bytes());
        hasher.update(self.backlog_passed.to_le_bytes());
        hasher.update(self.tests_passed.to_le_bytes());
        format!("{:x}", hasher.finalize())
    }
}

// =============================================================================
// PILLAR 3: SCAFFOLDING RULES
// =============================================================================

/// How we operate — the rules the worker must follow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaffoldingRules {
    /// How to validate work (test commands)
    pub test_commands: Vec<TestCommand>,
    /// Maximum attempts per feature before escalation
    pub max_attempts_per_feature: u32,
    /// Maximum total worker runs before stopping
    pub max_total_runs: u32,
    /// Whether to stop on first regression
    pub stop_on_regression: bool,
    /// Required success rate before marking project complete (0.0 to 1.0)
    pub required_pass_rate: f64,
    /// Bootup ritual steps (read in order before acting)
    pub bootup_ritual: Vec<String>,
    /// Constraints the worker must respect
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCommand {
    pub name: String,
    pub command: String,
    pub timeout_seconds: u32,
    pub required: bool,
}

impl Default for ScaffoldingRules {
    fn default() -> Self {
        Self {
            test_commands: vec![
                TestCommand {
                    name: "cargo test".into(),
                    command: "cargo test".into(),
                    timeout_seconds: 300,
                    required: true,
                },
                TestCommand {
                    name: "cargo clippy".into(),
                    command: "cargo clippy -- -D warnings".into(),
                    timeout_seconds: 120,
                    required: false,
                },
            ],
            max_attempts_per_feature: 3,
            max_total_runs: 50,
            stop_on_regression: false,
            required_pass_rate: 1.0,
            bootup_ritual: vec![
                "Read feature backlog from domain memory".into(),
                "Read progress log from last N runs".into(),
                "Run test suite to verify current state".into(),
                "Compare test results with backlog status (detect regressions)".into(),
                "Select ONE actionable feature (priority + dependency order)".into(),
                "Check failed_approaches to avoid repeating mistakes".into(),
            ],
            constraints: vec![
                "ONE feature per run — no partial progress".into(),
                "All tests must pass before marking feature as passed".into(),
                "If any previously-passing feature regresses, stop and fix it first".into(),
                "Leave campsite cleaner: update all state before exiting".into(),
                "Never skip the bootup ritual".into(),
            ],
        }
    }
}

impl ScaffoldingRules {
    /// Sovereign mode: local-only, no external calls, strict validation
    pub fn sovereign() -> Self {
        let mut rules = Self::default();
        rules.constraints.push("No external API calls — all inference local".into());
        rules.constraints.push("All data stays on-premise".into());
        rules.constraints.push("Egress blocked for AI APIs".into());
        rules.stop_on_regression = true;
        rules
    }
}

// =============================================================================
// PILLAR 4: PROGRESS LOG
// =============================================================================

/// A single worker run record — machine-readable history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRunLog {
    pub run_id: Uuid,
    pub run_number: u32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub target_feature: Option<Uuid>,
    pub target_feature_name: Option<String>,
    pub outcome: RunOutcome,
    pub tests_before: TestSnapshot,
    pub tests_after: TestSnapshot,
    pub changes_made: Vec<String>,
    pub regressions_detected: Vec<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RunOutcome {
    /// Feature was completed and passes
    FeaturePassed,
    /// Feature was attempted but still fails
    FeatureFailed,
    /// A regression was detected and fixed
    RegressionFixed,
    /// A regression was detected but not fixed
    RegressionUnfixed,
    /// No actionable features (all done or all blocked)
    NothingToDo,
    /// Worker hit max attempts for this feature
    MaxAttemptsReached,
    /// Worker was interrupted or errored
    Aborted,
}

impl RunOutcome {
    pub fn label(&self) -> &'static str {
        match self {
            RunOutcome::FeaturePassed => "FEATURE_PASSED",
            RunOutcome::FeatureFailed => "FEATURE_FAILED",
            RunOutcome::RegressionFixed => "REGRESSION_FIXED",
            RunOutcome::RegressionUnfixed => "REGRESSION_UNFIXED",
            RunOutcome::NothingToDo => "NOTHING_TO_DO",
            RunOutcome::MaxAttemptsReached => "MAX_ATTEMPTS",
            RunOutcome::Aborted => "ABORTED",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSnapshot {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub timestamp: DateTime<Utc>,
}

impl TestSnapshot {
    pub fn new(total: u32, passed: u32, failed: u32) -> Self {
        Self {
            total,
            passed,
            failed,
            timestamp: Utc::now(),
        }
    }

    pub fn all_passing(&self) -> bool {
        self.failed == 0 && self.total > 0
    }

    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 { return 0.0; }
        self.passed as f64 / self.total as f64
    }
}

/// The complete progress log — run history for agent reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressLog {
    pub project_id: Uuid,
    pub runs: Vec<WorkerRunLog>,
}

impl ProgressLog {
    pub fn new(project_id: Uuid) -> Self {
        Self {
            project_id,
            runs: Vec::new(),
        }
    }

    pub fn add_run(&mut self, run: WorkerRunLog) {
        self.runs.push(run);
    }

    pub fn last_n_runs(&self, n: usize) -> &[WorkerRunLog] {
        let start = self.runs.len().saturating_sub(n);
        &self.runs[start..]
    }

    pub fn total_runs(&self) -> u32 {
        self.runs.len() as u32
    }

    pub fn successful_runs(&self) -> usize {
        self.runs.iter().filter(|r| r.outcome == RunOutcome::FeaturePassed).count()
    }

    pub fn failed_runs(&self) -> usize {
        self.runs.iter().filter(|r| r.outcome == RunOutcome::FeatureFailed).count()
    }

    pub fn regression_count(&self) -> usize {
        self.runs.iter()
            .flat_map(|r| &r.regressions_detected)
            .count()
    }

    /// Check if we're in a loop (same feature failing repeatedly)
    pub fn detect_loop(&self, feature_id: Uuid, threshold: u32) -> bool {
        let recent_failures: u32 = self.last_n_runs(10)
            .iter()
            .filter(|r| r.target_feature == Some(feature_id) && r.outcome == RunOutcome::FeatureFailed)
            .count() as u32;
        recent_failures >= threshold
    }

    /// Get previously failed approaches for a feature (avoid repeating)
    pub fn failed_approaches_for(&self, feature_id: Uuid) -> Vec<String> {
        self.runs.iter()
            .filter(|r| r.target_feature == Some(feature_id) && r.outcome == RunOutcome::FeatureFailed)
            .flat_map(|r| r.changes_made.clone())
            .collect()
    }
}

// =============================================================================
// COMPOSITE: DOMAIN MEMORY
// =============================================================================

/// The complete externalized domain memory — everything the agent needs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainMemory {
    pub backlog: FeatureBacklog,
    pub state: SystemState,
    pub rules: ScaffoldingRules,
    pub progress: ProgressLog,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub memory_hash: String,
}

impl DomainMemory {
    pub fn new(backlog: FeatureBacklog, rules: ScaffoldingRules) -> Self {
        let state = SystemState::from_backlog(&backlog);
        let progress = ProgressLog::new(backlog.id);
        let mut mem = Self {
            backlog,
            state,
            rules,
            progress,
            created_at: Utc::now(),
            last_updated: Utc::now(),
            memory_hash: String::new(),
        };
        mem.memory_hash = mem.compute_hash();
        mem
    }

    pub fn update_state(&mut self) {
        self.state = SystemState::from_backlog(&self.backlog);
        self.last_updated = Utc::now();
        self.memory_hash = self.compute_hash();
    }

    pub fn is_complete(&self) -> bool {
        self.backlog.is_complete()
    }

    pub fn should_stop(&self) -> bool {
        let max_runs = self.rules.max_total_runs;
        let total_runs = self.progress.total_runs();

        if total_runs >= max_runs {
            return true;
        }

        if self.backlog.is_complete() {
            return true;
        }

        // All features either passed, skipped, or exceeded max attempts
        self.backlog.next_actionable().is_none()
    }

    pub fn summarize(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "=== DOMAIN MEMORY: {} ===",
            self.backlog.project_name
        ));
        lines.push(format!(
            "Progress: {}/{} features passed ({:.0}%)",
            self.backlog.passed(),
            self.backlog.total(),
            self.backlog.progress_percent()
        ));
        lines.push(format!(
            "Status: {} pending, {} failed, {} regressed",
            self.backlog.pending(),
            self.backlog.failed(),
            self.backlog.regressed()
        ));
        lines.push(format!(
            "Runs: {} total ({} success, {} failed)",
            self.progress.total_runs(),
            self.progress.successful_runs(),
            self.progress.failed_runs()
        ));

        if let Some(next) = self.backlog.next_actionable() {
            lines.push(format!("Next: [P{}] {} ({})", next.priority, next.name, next.status.label()));
        } else if self.backlog.is_complete() {
            lines.push("ALL FEATURES PASSED".to_string());
        } else {
            lines.push("BLOCKED: no actionable features".to_string());
        }

        lines.push(format!("Integrity: {}", &self.memory_hash[..16]));
        lines.push("=== END DOMAIN MEMORY ===".to_string());

        lines.join("\n")
    }

    fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.backlog.id.as_bytes());
        hasher.update(self.backlog.total().to_le_bytes());
        hasher.update(self.backlog.passed().to_le_bytes());
        hasher.update(self.progress.total_runs().to_le_bytes());
        hasher.update(self.last_updated.to_rfc3339().as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_backlog() -> FeatureBacklog {
        let mut backlog = FeatureBacklog::new("test-project");
        backlog.add_feature(
            Feature::new("User auth via SSH keys", 1)
                .with_criteria(vec!["SSH key login works".into(), "Password login rejected".into()])
                .with_tags(vec!["auth", "security"]),
        );
        backlog.add_feature(
            Feature::new("API rate limiting", 2)
                .with_criteria(vec!["429 after 100 req/min".into()]),
        );
        backlog.add_feature(
            Feature::new("Sovereign inference", 3)
                .with_criteria(vec!["No external API calls".into(), "Local model responds".into()])
                .with_tags(vec!["ai", "sovereign"]),
        );
        backlog
    }

    #[test]
    fn test_feature_lifecycle() {
        let mut f = Feature::new("test feature", 1);
        assert_eq!(f.status, FeatureStatus::Pending);
        assert!(f.status.is_actionable());

        f.mark_in_progress(Uuid::new_v4());
        assert_eq!(f.status, FeatureStatus::InProgress);
        assert_eq!(f.attempt_count, 1);

        f.mark_passed();
        assert_eq!(f.status, FeatureStatus::Passed);
        assert!(!f.status.is_actionable());

        f.mark_regressed("broke by other change");
        assert_eq!(f.status, FeatureStatus::Regressed);
        assert!(f.status.is_actionable());
    }

    #[test]
    fn test_backlog_next_actionable() {
        let backlog = sample_backlog();
        let next = backlog.next_actionable().unwrap();
        assert_eq!(next.priority, 1); // Highest priority first
    }

    #[test]
    fn test_backlog_respects_dependencies() {
        let mut backlog = FeatureBacklog::new("dep-test");
        let f1 = Feature::new("Base feature", 1);
        let f1_id = f1.id;
        backlog.add_feature(f1);
        backlog.add_feature(
            Feature::new("Dependent feature", 2).with_dependency(f1_id),
        );

        // Next should be f1 (f2 depends on f1)
        let next = backlog.next_actionable().unwrap();
        assert_eq!(next.name, "Base feature");

        // Mark f1 passed
        backlog.features[0].mark_passed();
        let next = backlog.next_actionable().unwrap();
        assert_eq!(next.name, "Dependent feature");
    }

    #[test]
    fn test_backlog_progress() {
        let mut backlog = sample_backlog();
        assert_eq!(backlog.progress_percent(), 0.0);

        backlog.features[0].mark_passed();
        let pct = backlog.progress_percent();
        assert!((pct - 33.33).abs() < 1.0);

        for f in &mut backlog.features {
            f.mark_passed();
        }
        assert!(backlog.is_complete());
        assert_eq!(backlog.progress_percent(), 100.0);
    }

    #[test]
    fn test_system_state_from_backlog() {
        let backlog = sample_backlog();
        let state = SystemState::from_backlog(&backlog);
        assert_eq!(state.backlog_total, 3);
        assert_eq!(state.backlog_pending, 3);
        assert_eq!(state.backlog_passed, 0);
        assert!(!state.state_hash.is_empty());
    }

    #[test]
    fn test_scaffolding_rules_default() {
        let rules = ScaffoldingRules::default();
        assert_eq!(rules.max_attempts_per_feature, 3);
        assert_eq!(rules.bootup_ritual.len(), 6);
        assert_eq!(rules.constraints.len(), 5);
    }

    #[test]
    fn test_scaffolding_rules_sovereign() {
        let rules = ScaffoldingRules::sovereign();
        assert!(rules.stop_on_regression);
        assert!(rules.constraints.iter().any(|c| c.contains("external API")));
    }

    #[test]
    fn test_progress_log_loop_detection() {
        let mut log = ProgressLog::new(Uuid::new_v4());
        let feature_id = Uuid::new_v4();

        for i in 0..5 {
            log.add_run(WorkerRunLog {
                run_id: Uuid::new_v4(),
                run_number: i,
                started_at: Utc::now(),
                completed_at: Some(Utc::now()),
                target_feature: Some(feature_id),
                target_feature_name: Some("stuck feature".into()),
                outcome: RunOutcome::FeatureFailed,
                tests_before: TestSnapshot::new(10, 8, 2),
                tests_after: TestSnapshot::new(10, 8, 2),
                changes_made: vec![format!("attempt {}", i)],
                regressions_detected: vec![],
                notes: String::new(),
            });
        }

        assert!(log.detect_loop(feature_id, 3));
        assert!(!log.detect_loop(Uuid::new_v4(), 3));
    }

    #[test]
    fn test_domain_memory_lifecycle() {
        let backlog = sample_backlog();
        let rules = ScaffoldingRules::sovereign();
        let mut memory = DomainMemory::new(backlog, rules);

        assert!(!memory.is_complete());
        assert!(!memory.should_stop());

        // Pass all features
        for f in &mut memory.backlog.features {
            f.mark_passed();
        }
        memory.update_state();

        assert!(memory.is_complete());
        assert!(memory.should_stop());
    }

    #[test]
    fn test_domain_memory_summarize() {
        let backlog = sample_backlog();
        let rules = ScaffoldingRules::default();
        let memory = DomainMemory::new(backlog, rules);
        let summary = memory.summarize();

        assert!(summary.contains("DOMAIN MEMORY"));
        assert!(summary.contains("test-project"));
        assert!(summary.contains("0/3"));
        assert!(summary.contains("Integrity"));
    }

    #[test]
    fn test_domain_memory_hash_changes_on_update() {
        let backlog = sample_backlog();
        let rules = ScaffoldingRules::default();
        let mut memory = DomainMemory::new(backlog, rules);
        let hash1 = memory.memory_hash.clone();

        memory.backlog.features[0].mark_passed();
        memory.update_state();
        let hash2 = memory.memory_hash.clone();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_test_snapshot() {
        let snap = TestSnapshot::new(100, 95, 5);
        assert!(!snap.all_passing());
        assert!((snap.pass_rate() - 0.95).abs() < 0.01);

        let perfect = TestSnapshot::new(100, 100, 0);
        assert!(perfect.all_passing());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let backlog = sample_backlog();
        let rules = ScaffoldingRules::sovereign();
        let memory = DomainMemory::new(backlog, rules);

        let json = serde_json::to_string(&memory).unwrap();
        let restored: DomainMemory = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.backlog.project_name, "test-project");
        assert_eq!(restored.backlog.total(), 3);
        assert_eq!(restored.memory_hash, memory.memory_hash);
    }
}
