//! # Apophy Masters — Triple Orchestration Layer
//!
//! Three sovereign Masters that form the control plane for the 3000-agent fleet:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                  TOUR DE CONTRÔLE                       │
//! └───────────┬──────────────┬──────────────┬───────────────┘
//!             │              │              │
//!      ┌──────┴──────┐ ┌────┴────┐ ┌───────┴───────┐
//!      │ TIME MASTER │ │  SPACE  │ │    LATENT     │
//!      │ Temporal    │ │ MASTER  │ │    MASTER     │
//!      │ scheduling  │ │ Distrib │ │   Patterns    │
//!      └──────┬──────┘ └────┬────┘ └───────┬───────┘
//!             └──────────────┼──────────────┘
//!                            ▼
//!                   3000 AGENT FLEET
//! ```
//!
//! - **TimeMaster**: Temporal orchestration — scheduling, rate limiting, cooldowns
//! - **SpaceMaster**: Spatial distribution — load balancing across CPU cores, memory zones
//! - **LatentMaster**: Latent pattern detection — recurring tasks, agent affinity, optimization

pub mod latent_master;
pub mod space_master;
pub mod time_master;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A task dispatched through the Masters pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterTask {
    pub id: Uuid,
    pub agent_id: String,
    pub domain: String,
    pub instruction: String,
    pub priority: TaskPriority,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
    pub estimated_tokens: usize,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Critical = 4,
    High = 3,
    Normal = 2,
    Low = 1,
    Background = 0,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Unified status from all three Masters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MastersPanelStatus {
    pub time_master: time_master::TimeMasterStatus,
    pub space_master: space_master::SpaceMasterStatus,
    pub latent_master: latent_master::LatentMasterStatus,
    pub total_queued: usize,
    pub total_running: usize,
    pub total_completed: u64,
    pub uptime_secs: u64,
}

/// The unified Masters control plane
pub struct MastersPanel {
    pub time: time_master::TimeMaster,
    pub space: space_master::SpaceMaster,
    pub latent: latent_master::LatentMaster,
    start_time: std::time::Instant,
    completed_count: u64,
}

impl MastersPanel {
    pub fn new(cpu_cores: usize, memory_mb: usize) -> Self {
        Self {
            time: time_master::TimeMaster::new(),
            space: space_master::SpaceMaster::new(cpu_cores, memory_mb),
            latent: latent_master::LatentMaster::new(),
            start_time: std::time::Instant::now(),
            completed_count: 0,
        }
    }

    /// Submit a task through the full Masters pipeline:
    /// TimeMaster (schedule) → SpaceMaster (allocate) → LatentMaster (optimize)
    pub fn submit(&mut self, task: MasterTask) -> MasterDecision {
        // Step 1: TimeMaster — check scheduling constraints
        let time_decision = self.time.evaluate(&task);
        if time_decision.defer {
            return MasterDecision {
                task_id: task.id,
                action: DecisionAction::Defer {
                    reason: time_decision.reason.clone(),
                    retry_after_ms: time_decision.retry_after_ms,
                },
                assigned_zone: None,
                optimizations: vec![],
            };
        }

        // Step 2: SpaceMaster — find best execution zone
        let zone = self.space.allocate(&task);

        // Step 3: LatentMaster — apply pattern-based optimizations
        let optimizations = self.latent.optimize(&task);

        MasterDecision {
            task_id: task.id,
            action: DecisionAction::Execute,
            assigned_zone: Some(zone),
            optimizations,
        }
    }

    /// Record task completion for learning
    pub fn complete(&mut self, task_id: Uuid, zone_id: usize, latency_ms: u64, tokens: usize) {
        self.completed_count += 1;
        self.space.release(zone_id);
        self.latent.record_completion(task_id, latency_ms, tokens);
        self.time.record_completion();
    }

    /// Get unified status
    pub fn status(&self) -> MastersPanelStatus {
        MastersPanelStatus {
            time_master: self.time.status(),
            space_master: self.space.status(),
            latent_master: self.latent.status(),
            total_queued: self.time.queue_len(),
            total_running: self.space.active_count(),
            total_completed: self.completed_count,
            uptime_secs: self.start_time.elapsed().as_secs(),
        }
    }

    /// Generate text report
    pub fn report(&self) -> String {
        let s = self.status();
        format!(
            "=== MASTERS PANEL ===\n\
             Uptime:    {}s\n\
             Queued:    {}\n\
             Running:   {}\n\
             Completed: {}\n\
             \n--- TimeMaster ---\n{}\
             \n--- SpaceMaster ---\n{}\
             \n--- LatentMaster ---\n{}",
            s.uptime_secs,
            s.total_queued,
            s.total_running,
            s.total_completed,
            self.time.report(),
            self.space.report(),
            self.latent.report(),
        )
    }
}

/// Decision returned by the Masters pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterDecision {
    pub task_id: Uuid,
    pub action: DecisionAction,
    pub assigned_zone: Option<space_master::ExecutionZone>,
    pub optimizations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionAction {
    Execute,
    Defer { reason: String, retry_after_ms: u64 },
    Reject { reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_task() -> MasterTask {
        MasterTask {
            id: Uuid::new_v4(),
            agent_id: "agent-001".into(),
            domain: "tech_web_dev".into(),
            instruction: "Build a landing page".into(),
            priority: TaskPriority::Normal,
            created_at: chrono::Utc::now(),
            deadline: None,
            estimated_tokens: 2048,
            tags: vec!["web".into(), "frontend".into()],
        }
    }

    #[test]
    fn test_masters_panel_submit() {
        let mut panel = MastersPanel::new(24, 96_000);
        let task = sample_task();
        let decision = panel.submit(task);
        assert!(matches!(decision.action, DecisionAction::Execute));
        assert!(decision.assigned_zone.is_some());
    }

    #[test]
    fn test_masters_panel_lifecycle() {
        let mut panel = MastersPanel::new(24, 96_000);
        let task = sample_task();
        let task_id = task.id;
        let decision = panel.submit(task);
        let zone_id = decision.assigned_zone.unwrap().id;

        panel.complete(task_id, zone_id, 150, 1024);
        assert_eq!(panel.status().total_completed, 1);
    }

    #[test]
    fn test_masters_report() {
        let panel = MastersPanel::new(24, 96_000);
        let report = panel.report();
        assert!(report.contains("MASTERS PANEL"));
        assert!(report.contains("TimeMaster"));
        assert!(report.contains("SpaceMaster"));
        assert!(report.contains("LatentMaster"));
    }
}
