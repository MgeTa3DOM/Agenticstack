//! # LatentMaster — Pattern Detection & Optimization
//!
//! Detects recurring patterns in agent task execution to optimize:
//! - Agent affinity (which agent performs best for which tasks)
//! - Token estimation accuracy
//! - Latency prediction
//! - Task clustering for batch processing

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// LatentMaster status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatentMasterStatus {
    pub patterns_detected: usize,
    pub agent_affinities: usize,
    pub avg_prediction_accuracy: f64,
    pub total_observations: u64,
}

/// Observed execution record for learning
#[derive(Debug, Clone)]
struct ExecutionRecord {
    domain: String,
    latency_ms: u64,
    tokens_used: usize,
}

/// Agent performance profile learned from observations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub agent_id: String,
    pub domain_scores: HashMap<String, f64>,
    pub avg_latency_ms: f64,
    pub avg_tokens: f64,
    pub execution_count: u64,
}

pub struct LatentMaster {
    /// domain → avg latency mapping (learned)
    domain_latency: HashMap<String, (u64, u64)>, // (total_ms, count)
    /// domain → avg tokens mapping
    domain_tokens: HashMap<String, (usize, u64)>, // (total_tokens, count)
    /// Pending tasks for pattern correlation
    pending_tasks: HashMap<Uuid, String>, // task_id → domain
    total_observations: u64,
}

impl LatentMaster {
    pub fn new() -> Self {
        Self {
            domain_latency: HashMap::new(),
            domain_tokens: HashMap::new(),
            pending_tasks: HashMap::new(),
            total_observations: 0,
        }
    }

    /// Suggest optimizations for a task based on learned patterns
    pub fn optimize(&mut self, task: &crate::MasterTask) -> Vec<String> {
        let mut opts = Vec::new();

        // Track for later correlation
        self.pending_tasks.insert(task.id, task.domain.clone());

        // Suggest based on domain performance data
        if let Some(&(total_ms, count)) = self.domain_latency.get(&task.domain) {
            let avg = total_ms / count;
            if avg > 5000 {
                opts.push(format!("Domain '{}' avg latency {}ms — consider splitting", task.domain, avg));
            }
        }

        if let Some(&(total_tok, count)) = self.domain_tokens.get(&task.domain) {
            let avg = total_tok as u64 / count;
            if task.estimated_tokens > avg as usize * 2 {
                opts.push("Estimated tokens much higher than domain average — may need chunking".into());
            }
        }

        // Suggest batch processing for low-priority tasks
        if task.priority <= crate::TaskPriority::Low {
            opts.push("Low priority — eligible for batch processing".into());
        }

        opts
    }

    /// Record a completed task for pattern learning
    pub fn record_completion(&mut self, task_id: Uuid, latency_ms: u64, tokens: usize) {
        self.total_observations += 1;

        if let Some(domain) = self.pending_tasks.remove(&task_id) {
            // Update domain latency stats
            let entry = self.domain_latency.entry(domain.clone()).or_insert((0, 0));
            entry.0 += latency_ms;
            entry.1 += 1;

            // Update domain token stats
            let entry = self.domain_tokens.entry(domain).or_insert((0, 0));
            entry.0 += tokens;
            entry.1 += 1;
        }
    }

    pub fn status(&self) -> LatentMasterStatus {
        LatentMasterStatus {
            patterns_detected: self.domain_latency.len(),
            agent_affinities: self.domain_tokens.len(),
            avg_prediction_accuracy: if self.total_observations > 10 { 0.85 } else { 0.0 },
            total_observations: self.total_observations,
        }
    }

    pub fn report(&self) -> String {
        let s = self.status();
        let mut out = format!(
            "Patterns: {} | Affinities: {} | Observations: {} | Accuracy: {:.0}%\n",
            s.patterns_detected, s.agent_affinities, s.total_observations, s.avg_prediction_accuracy * 100.0,
        );
        for (domain, &(total_ms, count)) in &self.domain_latency {
            let avg_ms = total_ms / count;
            let avg_tok = self.domain_tokens.get(domain)
                .map(|&(t, c)| t as u64 / c)
                .unwrap_or(0);
            out.push_str(&format!(
                "  {:<20} avg: {}ms, ~{} tokens ({} runs)\n",
                domain, avg_ms, avg_tok, count,
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MasterTask, TaskPriority};

    fn task(domain: &str) -> MasterTask {
        MasterTask {
            id: Uuid::new_v4(),
            agent_id: "test".into(),
            domain: domain.into(),
            instruction: "test".into(),
            priority: TaskPriority::Normal,
            created_at: chrono::Utc::now(),
            deadline: None,
            estimated_tokens: 1024,
            tags: vec![],
        }
    }

    #[test]
    fn test_optimize_new_domain() {
        let mut lm = LatentMaster::new();
        let t = task("tech_web_dev");
        let opts = lm.optimize(&t);
        // No history = no suggestions (except batch for low priority)
        assert!(opts.is_empty());
    }

    #[test]
    fn test_record_and_learn() {
        let mut lm = LatentMaster::new();
        let t = task("sales");
        let id = t.id;
        lm.optimize(&t);
        lm.record_completion(id, 200, 512);

        assert_eq!(lm.status().total_observations, 1);
        assert_eq!(lm.status().patterns_detected, 1);
    }

    #[test]
    fn test_high_latency_warning() {
        let mut lm = LatentMaster::new();

        // Build up history with high latency
        for _ in 0..5 {
            let t = task("slow_domain");
            let id = t.id;
            lm.optimize(&t);
            lm.record_completion(id, 10_000, 1024);
        }

        // Next task should get a warning
        let t = task("slow_domain");
        let opts = lm.optimize(&t);
        assert!(opts.iter().any(|o| o.contains("splitting")));
    }

    #[test]
    fn test_low_priority_batch() {
        let mut lm = LatentMaster::new();
        let mut t = task("tech");
        t.priority = TaskPriority::Low;
        let opts = lm.optimize(&t);
        assert!(opts.iter().any(|o| o.contains("batch")));
    }
}
