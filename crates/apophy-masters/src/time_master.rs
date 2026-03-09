//! # TimeMaster — Temporal Orchestration
//!
//! Manages scheduling, rate limiting, priority queuing, and cooldown periods.
//! Ensures the 3000-agent fleet doesn't overwhelm inference backends.

use crate::{MasterTask, TaskPriority};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// TimeMaster configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeMasterConfig {
    /// Maximum tasks per second across all agents
    pub max_tasks_per_second: u32,
    /// Cooldown between tasks for the same agent (ms)
    pub agent_cooldown_ms: u64,
    /// Maximum queue depth before rejecting
    pub max_queue_depth: usize,
    /// Priority boost for overdue tasks (ms threshold)
    pub overdue_threshold_ms: u64,
}

impl Default for TimeMasterConfig {
    fn default() -> Self {
        Self {
            max_tasks_per_second: 100,
            agent_cooldown_ms: 500,
            max_queue_depth: 10_000,
            overdue_threshold_ms: 30_000,
        }
    }
}

/// TimeMaster evaluation result
#[derive(Debug, Clone)]
pub struct TimeDecision {
    pub defer: bool,
    pub reason: String,
    pub retry_after_ms: u64,
    pub effective_priority: TaskPriority,
}

/// TimeMaster status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeMasterStatus {
    pub queue_depth: usize,
    pub tasks_per_second: f64,
    pub total_scheduled: u64,
    pub total_deferred: u64,
    pub avg_wait_ms: f64,
}

pub struct TimeMaster {
    config: TimeMasterConfig,
    queue: VecDeque<uuid::Uuid>,
    scheduled_count: u64,
    deferred_count: u64,
    completions_in_window: Vec<std::time::Instant>,
    total_wait_ms: u64,
    wait_count: u64,
}

impl TimeMaster {
    pub fn new() -> Self {
        Self::with_config(TimeMasterConfig::default())
    }

    pub fn with_config(config: TimeMasterConfig) -> Self {
        Self {
            config,
            queue: VecDeque::new(),
            scheduled_count: 0,
            deferred_count: 0,
            completions_in_window: Vec::new(),
            total_wait_ms: 0,
            wait_count: 0,
        }
    }

    /// Evaluate whether a task should be scheduled now or deferred
    pub fn evaluate(&mut self, task: &MasterTask) -> TimeDecision {
        // Clean old completions (keep last second)
        let cutoff = std::time::Instant::now() - std::time::Duration::from_secs(1);
        self.completions_in_window.retain(|t| *t > cutoff);

        // Check rate limit
        if self.completions_in_window.len() >= self.config.max_tasks_per_second as usize {
            self.deferred_count += 1;
            return TimeDecision {
                defer: true,
                reason: "Rate limit reached".into(),
                retry_after_ms: 1000 / self.config.max_tasks_per_second as u64,
                effective_priority: task.priority,
            };
        }

        // Check queue depth
        if self.queue.len() >= self.config.max_queue_depth {
            // Only accept critical tasks when queue is full
            if task.priority < TaskPriority::Critical {
                self.deferred_count += 1;
                return TimeDecision {
                    defer: true,
                    reason: "Queue full — only critical tasks accepted".into(),
                    retry_after_ms: 5000,
                    effective_priority: task.priority,
                };
            }
        }

        // Priority boost for overdue tasks
        let mut effective_priority = task.priority;
        if let Some(deadline) = task.deadline {
            let now = chrono::Utc::now();
            if deadline < now {
                effective_priority = TaskPriority::Critical;
            } else {
                let remaining = (deadline - now).num_milliseconds() as u64;
                if remaining < self.config.overdue_threshold_ms {
                    effective_priority = std::cmp::max(effective_priority, TaskPriority::High);
                }
            }
        }

        // Schedule it
        self.queue.push_back(task.id);
        self.scheduled_count += 1;

        TimeDecision {
            defer: false,
            reason: "Scheduled".into(),
            retry_after_ms: 0,
            effective_priority,
        }
    }

    /// Record a completion (for rate tracking)
    pub fn record_completion(&mut self) {
        self.completions_in_window.push(std::time::Instant::now());
        self.queue.pop_front();
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    pub fn status(&self) -> TimeMasterStatus {
        let cutoff = std::time::Instant::now() - std::time::Duration::from_secs(1);
        let recent = self.completions_in_window.iter().filter(|t| **t > cutoff).count();

        TimeMasterStatus {
            queue_depth: self.queue.len(),
            tasks_per_second: recent as f64,
            total_scheduled: self.scheduled_count,
            total_deferred: self.deferred_count,
            avg_wait_ms: if self.wait_count > 0 {
                self.total_wait_ms as f64 / self.wait_count as f64
            } else {
                0.0
            },
        }
    }

    pub fn report(&self) -> String {
        let s = self.status();
        format!(
            "Queue: {} | Rate: {:.1}/s | Scheduled: {} | Deferred: {} | AvgWait: {:.0}ms\n",
            s.queue_depth, s.tasks_per_second, s.total_scheduled, s.total_deferred, s.avg_wait_ms,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(priority: TaskPriority) -> MasterTask {
        MasterTask {
            id: uuid::Uuid::new_v4(),
            agent_id: "test".into(),
            domain: "tech".into(),
            instruction: "test".into(),
            priority,
            created_at: chrono::Utc::now(),
            deadline: None,
            estimated_tokens: 100,
            tags: vec![],
        }
    }

    #[test]
    fn test_schedule_normal() {
        let mut tm = TimeMaster::new();
        let d = tm.evaluate(&task(TaskPriority::Normal));
        assert!(!d.defer);
        assert_eq!(tm.queue_len(), 1);
    }

    #[test]
    fn test_queue_full_rejects_low() {
        let config = TimeMasterConfig {
            max_queue_depth: 2,
            ..Default::default()
        };
        let mut tm = TimeMaster::with_config(config);
        tm.evaluate(&task(TaskPriority::Normal));
        tm.evaluate(&task(TaskPriority::Normal));
        let d = tm.evaluate(&task(TaskPriority::Low));
        assert!(d.defer);
    }

    #[test]
    fn test_queue_full_accepts_critical() {
        let config = TimeMasterConfig {
            max_queue_depth: 2,
            ..Default::default()
        };
        let mut tm = TimeMaster::with_config(config);
        tm.evaluate(&task(TaskPriority::Normal));
        tm.evaluate(&task(TaskPriority::Normal));
        let d = tm.evaluate(&task(TaskPriority::Critical));
        assert!(!d.defer);
    }

    #[test]
    fn test_completion_tracking() {
        let mut tm = TimeMaster::new();
        tm.evaluate(&task(TaskPriority::Normal));
        assert_eq!(tm.queue_len(), 1);
        tm.record_completion();
        assert_eq!(tm.queue_len(), 0);
    }
}
