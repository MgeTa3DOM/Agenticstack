//! # AutoDev — Self-Developing, Self-Deploying, Self-Maintaining Pipeline
//!
//! The autonomous development pipeline that keeps the sovereign stack alive.
//! No human intervention needed for routine maintenance, testing, documentation,
//! and deployment. The neutral fiber ensures stability through all changes.
//!
//! ## Pipeline Stages
//!
//! ```text
//! ┌─────────┐   ┌──────┐   ┌────────┐   ┌────────┐   ┌──────────┐
//! │  Watch   │──▶│Build │──▶│  Test  │──▶│ Deploy │──▶│ Monitor  │
//! │ (git)    │   │(cargo)│   │(cargo) │   │(docker)│   │(health)  │
//! └─────────┘   └──────┘   └────────┘   └────────┘   └──────────┘
//!       │                                                  │
//!       │         ┌──────────┐   ┌──────────┐             │
//!       └────────▶│ AutoDoc  │──▶│FineTune  │◀────────────┘
//!                 │(prime.js)│   │(UV/Unsloth)│
//!                 └──────────┘   └──────────┘
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

// =============================================================================
// PIPELINE DEFINITION
// =============================================================================

/// A single pipeline run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRun {
    pub id: Uuid,
    pub trigger: PipelineTrigger,
    pub stages: Vec<StageResult>,
    pub status: PipelineStatus,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: u64,
    /// Hash of the commit/state that triggered this run
    pub source_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineTrigger {
    /// Git push/commit
    GitPush { branch: String, commit: String },
    /// Scheduled (cron-like)
    Scheduled { interval: String },
    /// Health check failure
    HealthFailure { service: String, error: String },
    /// Manual trigger
    Manual { user: String },
    /// AZR self-play generated new training data
    DatasetUpdate { dataset: String, entries: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineStatus {
    Pending,
    Running,
    Success,
    Failed,
    Recovered,
    Skipped,
}

/// Result of a single pipeline stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub stage: PipelineStage,
    pub status: StageStatus,
    pub output: String,
    pub duration_ms: u64,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineStage {
    /// Check source code
    Lint,
    /// Compile the workspace
    Build,
    /// Run all tests
    Test,
    /// Generate documentation
    Doc,
    /// Deploy to target
    Deploy,
    /// Run health checks
    Health,
    /// Collect training data
    DataCollect,
    /// Fine-tune model
    FineTune,
    /// Auto-recover from failure
    Recover,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
}

impl std::fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lint => write!(f, "lint"),
            Self::Build => write!(f, "build"),
            Self::Test => write!(f, "test"),
            Self::Doc => write!(f, "doc"),
            Self::Deploy => write!(f, "deploy"),
            Self::Health => write!(f, "health"),
            Self::DataCollect => write!(f, "data-collect"),
            Self::FineTune => write!(f, "fine-tune"),
            Self::Recover => write!(f, "recover"),
        }
    }
}

// =============================================================================
// AUTO-DEV ENGINE
// =============================================================================

/// The AutoDev engine manages the autonomous pipeline
pub struct AutoDevEngine {
    pub config: AutoDevEngineConfig,
    pub history: Vec<PipelineRun>,
    pub current_run: Option<PipelineRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoDevEngineConfig {
    /// Enable the engine
    pub enabled: bool,
    /// Stages to run (in order)
    pub stages: Vec<PipelineStage>,
    /// Stop on first failure (vs. continue)
    pub fail_fast: bool,
    /// Auto-recover on failure
    pub auto_recover: bool,
    /// Maximum history to keep
    pub max_history: usize,
    /// Build command
    pub build_cmd: String,
    /// Test command
    pub test_cmd: String,
    /// Lint command
    pub lint_cmd: String,
    /// Doc generation command
    pub doc_cmd: String,
    /// Deploy command
    pub deploy_cmd: String,
    /// Health check URL
    pub health_url: String,
    /// Fine-tune script path
    pub finetune_script: String,
}

impl Default for AutoDevEngineConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            stages: vec![
                PipelineStage::Lint,
                PipelineStage::Build,
                PipelineStage::Test,
                PipelineStage::Doc,
                PipelineStage::Deploy,
                PipelineStage::Health,
            ],
            fail_fast: true,
            auto_recover: true,
            max_history: 100,
            build_cmd: "cargo build --workspace --release".to_string(),
            test_cmd: "cargo test --workspace".to_string(),
            lint_cmd: "cargo clippy --workspace -- -D warnings".to_string(),
            doc_cmd: "node prime.js --doctor".to_string(),
            deploy_cmd: "docker compose up -d --build".to_string(),
            health_url: "http://localhost:8080/health".to_string(),
            finetune_script: "tools/finetune/trainer.py".to_string(),
        }
    }
}

impl AutoDevEngine {
    pub fn new(config: AutoDevEngineConfig) -> Self {
        Self {
            config,
            history: Vec::new(),
            current_run: None,
        }
    }

    /// Start a new pipeline run
    pub fn start_run(&mut self, trigger: PipelineTrigger) -> &PipelineRun {
        let source_hash = match &trigger {
            PipelineTrigger::GitPush { commit, .. } => commit.clone(),
            _ => {
                let mut hasher = Sha256::new();
                hasher.update(format!("{:?}{}", trigger, Utc::now()).as_bytes());
                format!("{:x}", hasher.finalize())[..12].to_string()
            }
        };

        let run = PipelineRun {
            id: Uuid::new_v4(),
            trigger,
            stages: Vec::new(),
            status: PipelineStatus::Running,
            started_at: Utc::now(),
            finished_at: None,
            duration_ms: 0,
            source_hash,
        };

        self.current_run = Some(run);
        self.current_run.as_ref().unwrap()
    }

    /// Record a stage result
    pub fn record_stage(&mut self, stage: PipelineStage, status: StageStatus, output: &str, duration_ms: u64) {
        if let Some(ref mut run) = self.current_run {
            run.stages.push(StageResult {
                stage,
                status,
                output: output.to_string(),
                duration_ms,
                started_at: Utc::now(),
            });

            // If fail_fast and stage failed, mark run as failed
            if self.config.fail_fast && status == StageStatus::Failed {
                run.status = PipelineStatus::Failed;
            }
        }
    }

    /// Complete the current run
    pub fn complete_run(&mut self, success: bool) {
        if let Some(mut run) = self.current_run.take() {
            run.finished_at = Some(Utc::now());
            run.duration_ms = (Utc::now() - run.started_at).num_milliseconds().max(0) as u64;
            run.status = if success {
                PipelineStatus::Success
            } else {
                PipelineStatus::Failed
            };

            self.history.push(run);

            // Trim history
            while self.history.len() > self.config.max_history {
                self.history.remove(0);
            }
        }
    }

    /// Get pipeline statistics
    pub fn stats(&self) -> PipelineStats {
        let total = self.history.len();
        let successes = self.history.iter().filter(|r| r.status == PipelineStatus::Success).count();
        let failures = self.history.iter().filter(|r| r.status == PipelineStatus::Failed).count();
        let recovered = self.history.iter().filter(|r| r.status == PipelineStatus::Recovered).count();

        let avg_duration = if total > 0 {
            self.history.iter().map(|r| r.duration_ms).sum::<u64>() / total as u64
        } else {
            0
        };

        let success_rate = if total > 0 {
            successes as f64 / total as f64
        } else {
            1.0
        };

        PipelineStats {
            total_runs: total,
            successes,
            failures,
            recovered,
            avg_duration_ms: avg_duration,
            success_rate,
            is_running: self.current_run.is_some(),
            last_run: self.history.last().map(|r| r.started_at),
        }
    }

    /// Get the commands for each stage
    pub fn stage_command(&self, stage: PipelineStage) -> &str {
        match stage {
            PipelineStage::Build => &self.config.build_cmd,
            PipelineStage::Test => &self.config.test_cmd,
            PipelineStage::Lint => &self.config.lint_cmd,
            PipelineStage::Doc => &self.config.doc_cmd,
            PipelineStage::Deploy => &self.config.deploy_cmd,
            PipelineStage::Health => &self.config.health_url,
            PipelineStage::FineTune => &self.config.finetune_script,
            PipelineStage::DataCollect => "cargo run -p apophy-sovereign -- self-play",
            PipelineStage::Recover => "docker compose restart",
        }
    }

    /// Generate a pipeline run report
    pub fn report(&self) -> String {
        let stats = self.stats();
        let mut report = format!(
            "=== AutoDev Pipeline Report ===\n\
             Total Runs:    {}\n\
             Success Rate:  {:.0}%\n\
             Avg Duration:  {}ms\n\
             Currently:     {}\n",
            stats.total_runs,
            stats.success_rate * 100.0,
            stats.avg_duration_ms,
            if stats.is_running { "RUNNING" } else { "IDLE" },
        );

        if let Some(last) = self.history.last() {
            report.push_str(&format!(
                "\nLast Run: {} ({:?})\n  Trigger: {:?}\n  Status:  {:?}\n  Stages:  {}\n",
                last.id,
                last.started_at,
                last.trigger,
                last.status,
                last.stages.len(),
            ));

            for stage in &last.stages {
                report.push_str(&format!(
                    "    {} : {:?} ({}ms)\n",
                    stage.stage, stage.status, stage.duration_ms,
                ));
            }
        }

        report
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStats {
    pub total_runs: usize,
    pub successes: usize,
    pub failures: usize,
    pub recovered: usize,
    pub avg_duration_ms: u64,
    pub success_rate: f64,
    pub is_running: bool,
    pub last_run: Option<DateTime<Utc>>,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autodev_engine_creation() {
        let engine = AutoDevEngine::new(AutoDevEngineConfig::default());
        assert!(engine.config.enabled);
        assert_eq!(engine.config.stages.len(), 6);
        assert!(engine.history.is_empty());
    }

    #[test]
    fn test_pipeline_run_lifecycle() {
        let mut engine = AutoDevEngine::new(AutoDevEngineConfig::default());

        // Start a run
        let trigger = PipelineTrigger::Manual { user: "test".to_string() };
        engine.start_run(trigger);
        assert!(engine.current_run.is_some());

        // Record stages
        engine.record_stage(PipelineStage::Lint, StageStatus::Success, "OK", 100);
        engine.record_stage(PipelineStage::Build, StageStatus::Success, "Built", 5000);
        engine.record_stage(PipelineStage::Test, StageStatus::Success, "509 passed", 3000);

        // Complete
        engine.complete_run(true);
        assert!(engine.current_run.is_none());
        assert_eq!(engine.history.len(), 1);
        assert_eq!(engine.history[0].status, PipelineStatus::Success);
        assert_eq!(engine.history[0].stages.len(), 3);
    }

    #[test]
    fn test_pipeline_fail_fast() {
        let mut engine = AutoDevEngine::new(AutoDevEngineConfig {
            fail_fast: true,
            ..Default::default()
        });

        engine.start_run(PipelineTrigger::Manual { user: "test".to_string() });
        engine.record_stage(PipelineStage::Build, StageStatus::Failed, "compile error", 1000);

        // Run should be marked failed
        assert_eq!(engine.current_run.as_ref().unwrap().status, PipelineStatus::Failed);

        engine.complete_run(false);
        assert_eq!(engine.history[0].status, PipelineStatus::Failed);
    }

    #[test]
    fn test_pipeline_stats() {
        let mut engine = AutoDevEngine::new(AutoDevEngineConfig::default());

        // Run 1: success
        engine.start_run(PipelineTrigger::Manual { user: "a".to_string() });
        engine.record_stage(PipelineStage::Build, StageStatus::Success, "ok", 100);
        engine.complete_run(true);

        // Run 2: failure
        engine.start_run(PipelineTrigger::Manual { user: "b".to_string() });
        engine.record_stage(PipelineStage::Build, StageStatus::Failed, "err", 50);
        engine.complete_run(false);

        let stats = engine.stats();
        assert_eq!(stats.total_runs, 2);
        assert_eq!(stats.successes, 1);
        assert_eq!(stats.failures, 1);
        assert!((stats.success_rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_pipeline_history_trim() {
        let mut engine = AutoDevEngine::new(AutoDevEngineConfig {
            max_history: 3,
            ..Default::default()
        });

        for i in 0..5 {
            engine.start_run(PipelineTrigger::Manual { user: format!("u{}", i) });
            engine.complete_run(true);
        }

        assert_eq!(engine.history.len(), 3);
    }

    #[test]
    fn test_pipeline_report() {
        let mut engine = AutoDevEngine::new(AutoDevEngineConfig::default());
        engine.start_run(PipelineTrigger::GitPush {
            branch: "main".to_string(),
            commit: "abc1234".to_string(),
        });
        engine.record_stage(PipelineStage::Build, StageStatus::Success, "ok", 100);
        engine.complete_run(true);

        let report = engine.report();
        assert!(report.contains("AutoDev Pipeline Report"));
        assert!(report.contains("Success Rate"));
    }

    #[test]
    fn test_stage_commands() {
        let engine = AutoDevEngine::new(AutoDevEngineConfig::default());
        assert!(engine.stage_command(PipelineStage::Build).contains("cargo build"));
        assert!(engine.stage_command(PipelineStage::Test).contains("cargo test"));
        assert!(engine.stage_command(PipelineStage::Doc).contains("prime.js"));
    }

    #[test]
    fn test_git_push_trigger() {
        let mut engine = AutoDevEngine::new(AutoDevEngineConfig::default());
        engine.start_run(PipelineTrigger::GitPush {
            branch: "main".to_string(),
            commit: "deadbeef".to_string(),
        });

        let run = engine.current_run.as_ref().unwrap();
        assert_eq!(run.source_hash, "deadbeef");
    }

    #[test]
    fn test_dataset_update_trigger() {
        let mut engine = AutoDevEngine::new(AutoDevEngineConfig::default());
        engine.start_run(PipelineTrigger::DatasetUpdate {
            dataset: "math-v1".to_string(),
            entries: 100,
        });

        assert!(engine.current_run.is_some());
    }
}
