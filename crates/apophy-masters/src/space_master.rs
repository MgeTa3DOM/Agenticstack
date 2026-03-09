//! # SpaceMaster — Spatial Distribution
//!
//! Distributes agent workloads across CPU cores and memory zones.
//! Optimized for OVH VPS 24CPU/96GB/400GB SSD configuration.

use crate::MasterTask;
use serde::{Deserialize, Serialize};

/// Execution zone — a logical partition of compute resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionZone {
    pub id: usize,
    pub name: String,
    pub cpu_cores: usize,
    pub memory_mb: usize,
    pub active_tasks: usize,
    pub max_tasks: usize,
    pub zone_type: ZoneType,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ZoneType {
    /// Rust core orchestration (8 cores)
    CoreEngine,
    /// Python + uv ML agents (8 cores)
    AgentPool,
    /// Bun dashboard + WebSocket (4 cores)
    Dashboard,
    /// Browser automation + Playwright (4 cores)
    BrowserAI,
}

impl ZoneType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::CoreEngine => "Core Engine (Rust)",
            Self::AgentPool => "Agent Pool (Python/uv)",
            Self::Dashboard => "Dashboard (Bun)",
            Self::BrowserAI => "Browser AI (Playwright)",
        }
    }
}

/// SpaceMaster status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceMasterStatus {
    pub total_cores: usize,
    pub total_memory_mb: usize,
    pub zones: Vec<ExecutionZone>,
    pub active_tasks: usize,
    pub utilization_pct: f64,
}

pub struct SpaceMaster {
    zones: Vec<ExecutionZone>,
    total_cores: usize,
    total_memory_mb: usize,
}

impl SpaceMaster {
    /// Create SpaceMaster with optimal zone layout for given hardware
    pub fn new(cpu_cores: usize, memory_mb: usize) -> Self {
        // Distribute cores: 33% core, 33% agents, 17% dashboard, 17% browser
        let core_cores = (cpu_cores as f64 * 0.33).ceil() as usize;
        let agent_cores = (cpu_cores as f64 * 0.33).ceil() as usize;
        let dash_cores = (cpu_cores as f64 * 0.17).ceil() as usize;
        let browser_cores = cpu_cores.saturating_sub(core_cores + agent_cores + dash_cores);

        // Memory: 20% core, 50% agents, 15% dashboard, 15% browser
        let core_mem = memory_mb / 5;
        let agent_mem = memory_mb / 2;
        let dash_mem = (memory_mb as f64 * 0.15) as usize;
        let browser_mem = memory_mb.saturating_sub(core_mem + agent_mem + dash_mem);

        let zones = vec![
            ExecutionZone {
                id: 0,
                name: "core-engine".into(),
                cpu_cores: core_cores,
                memory_mb: core_mem,
                active_tasks: 0,
                max_tasks: core_cores * 4,
                zone_type: ZoneType::CoreEngine,
            },
            ExecutionZone {
                id: 1,
                name: "agent-pool".into(),
                cpu_cores: agent_cores,
                memory_mb: agent_mem,
                active_tasks: 0,
                max_tasks: agent_cores * 8,
                zone_type: ZoneType::AgentPool,
            },
            ExecutionZone {
                id: 2,
                name: "dashboard".into(),
                cpu_cores: dash_cores,
                memory_mb: dash_mem,
                active_tasks: 0,
                max_tasks: dash_cores * 2,
                zone_type: ZoneType::Dashboard,
            },
            ExecutionZone {
                id: 3,
                name: "browser-ai".into(),
                cpu_cores: browser_cores.max(1),
                memory_mb: browser_mem,
                active_tasks: 0,
                max_tasks: browser_cores.max(1) * 2,
                zone_type: ZoneType::BrowserAI,
            },
        ];

        Self {
            zones,
            total_cores: cpu_cores,
            total_memory_mb: memory_mb,
        }
    }

    /// Allocate a zone for a task based on its characteristics
    pub fn allocate(&mut self, task: &MasterTask) -> ExecutionZone {
        // Determine ideal zone by task tags/domain
        let preferred_zone = if task.tags.iter().any(|t| t == "browser" || t == "playwright" || t == "chrome") {
            ZoneType::BrowserAI
        } else if task.tags.iter().any(|t| t == "dashboard" || t == "ui" || t == "websocket") {
            ZoneType::Dashboard
        } else if task.estimated_tokens > 4096 || task.tags.iter().any(|t| t == "inference" || t == "ml") {
            ZoneType::AgentPool
        } else {
            ZoneType::AgentPool // Default to agent pool
        };

        // Find the best zone index: preferred first, then fallback to least busy
        let zone_idx = self.zones.iter()
            .enumerate()
            .filter(|(_, z)| z.zone_type == preferred_zone && z.active_tasks < z.max_tasks)
            .map(|(i, _)| i)
            .next()
            .or_else(|| {
                self.zones.iter()
                    .enumerate()
                    .filter(|(_, z)| z.active_tasks < z.max_tasks)
                    .min_by_key(|(_, z)| z.active_tasks)
                    .map(|(i, _)| i)
            })
            .unwrap_or_else(|| {
                // All full — find agent pool or use first zone
                self.zones.iter()
                    .position(|z| z.zone_type == ZoneType::AgentPool)
                    .unwrap_or(0)
            });

        self.zones[zone_idx].active_tasks += 1;
        self.zones[zone_idx].clone()
    }

    /// Release a zone slot after task completion
    pub fn release(&mut self, zone_id: usize) {
        if let Some(zone) = self.zones.iter_mut().find(|z| z.id == zone_id) {
            zone.active_tasks = zone.active_tasks.saturating_sub(1);
        }
    }

    pub fn active_count(&self) -> usize {
        self.zones.iter().map(|z| z.active_tasks).sum()
    }

    pub fn status(&self) -> SpaceMasterStatus {
        let active: usize = self.zones.iter().map(|z| z.active_tasks).sum();
        let max: usize = self.zones.iter().map(|z| z.max_tasks).sum();
        SpaceMasterStatus {
            total_cores: self.total_cores,
            total_memory_mb: self.total_memory_mb,
            zones: self.zones.clone(),
            active_tasks: active,
            utilization_pct: if max > 0 { active as f64 / max as f64 * 100.0 } else { 0.0 },
        }
    }

    pub fn report(&self) -> String {
        let s = self.status();
        let mut out = format!(
            "Cores: {} | Memory: {} MB | Active: {} | Util: {:.1}%\n",
            s.total_cores, s.total_memory_mb, s.active_tasks, s.utilization_pct,
        );
        for zone in &s.zones {
            out.push_str(&format!(
                "  [{:>2}] {:<16} {:>2} cores, {:>6} MB, {}/{} tasks ({})\n",
                zone.id, zone.name, zone.cpu_cores, zone.memory_mb,
                zone.active_tasks, zone.max_tasks, zone.zone_type.label(),
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TaskPriority;

    fn task_with_tags(tags: Vec<&str>) -> MasterTask {
        MasterTask {
            id: uuid::Uuid::new_v4(),
            agent_id: "test".into(),
            domain: "tech".into(),
            instruction: "test".into(),
            priority: TaskPriority::Normal,
            created_at: chrono::Utc::now(),
            deadline: None,
            estimated_tokens: 100,
            tags: tags.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn test_zone_allocation() {
        let mut sm = SpaceMaster::new(24, 96_000);
        let task = task_with_tags(vec!["web"]);
        let zone = sm.allocate(&task);
        assert_eq!(zone.zone_type, ZoneType::AgentPool);
        assert_eq!(sm.active_count(), 1);
    }

    #[test]
    fn test_browser_zone() {
        let mut sm = SpaceMaster::new(24, 96_000);
        let task = task_with_tags(vec!["browser", "playwright"]);
        let zone = sm.allocate(&task);
        assert_eq!(zone.zone_type, ZoneType::BrowserAI);
    }

    #[test]
    fn test_release() {
        let mut sm = SpaceMaster::new(24, 96_000);
        let task = task_with_tags(vec![]);
        let zone = sm.allocate(&task);
        assert_eq!(sm.active_count(), 1);
        sm.release(zone.id);
        assert_eq!(sm.active_count(), 0);
    }

    #[test]
    fn test_status() {
        let sm = SpaceMaster::new(24, 96_000);
        let s = sm.status();
        assert_eq!(s.total_cores, 24);
        assert_eq!(s.zones.len(), 4);
    }
}
