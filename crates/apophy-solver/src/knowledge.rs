//! Knowledge Base — Institutional memory for problem patterns
//!
//! Captures problem-solution pairs to accelerate future diagnosis.
//! Enables pattern matching against historical resolutions.

use crate::{Diagnosis, DiagnosticCategory, Problem, Resolution};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: Uuid,
    pub problem_title: String,
    pub problem_domain: String,
    pub root_cause_category: String,
    pub root_cause: String,
    pub solution_titles: Vec<String>,
    pub tags: Vec<String>,
    pub effectiveness: f64,
    pub times_referenced: u64,
    pub created_at: DateTime<Utc>,
}

pub struct KnowledgeBase {
    pub entries: Vec<KnowledgeEntry>,
}

impl KnowledgeBase {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn capture(&mut self, problem: &Problem, diagnosis: &Diagnosis, resolution: &Resolution) {
        let solution_titles: Vec<String> = resolution.solutions.iter()
            .map(|s| s.title.clone())
            .collect();

        let avg_effectiveness = if resolution.solutions.is_empty() {
            0.0
        } else {
            resolution.solutions.iter().map(|s| s.effectiveness_score).sum::<f64>()
                / resolution.solutions.len() as f64
        };

        let entry = KnowledgeEntry {
            id: Uuid::new_v4(),
            problem_title: problem.title.clone(),
            problem_domain: problem.domain.to_string(),
            root_cause_category: format!("{}", diagnosis.category),
            root_cause: diagnosis.root_cause.clone(),
            solution_titles,
            tags: problem.tags.clone(),
            effectiveness: avg_effectiveness,
            times_referenced: 0,
            created_at: Utc::now(),
        };

        tracing::debug!("Knowledge captured: {} → {}", entry.problem_title, entry.root_cause);
        self.entries.push(entry);
    }

    pub fn search(&mut self, query: &str) -> Vec<&KnowledgeEntry> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<&mut KnowledgeEntry> = self.entries.iter_mut()
            .filter(|e| {
                e.problem_title.to_lowercase().contains(&query_lower)
                    || e.root_cause.to_lowercase().contains(&query_lower)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
                    || e.problem_domain.to_lowercase().contains(&query_lower)
            })
            .collect();

        for entry in &mut results {
            entry.times_referenced += 1;
        }

        // Convert back to immutable refs
        results.into_iter().map(|e| &*e).collect()
    }

    pub fn search_by_category(&self, category: &DiagnosticCategory) -> Vec<&KnowledgeEntry> {
        let cat_str = format!("{}", category);
        self.entries.iter()
            .filter(|e| e.root_cause_category == cat_str)
            .collect()
    }

    pub fn top_patterns(&self, limit: usize) -> Vec<&KnowledgeEntry> {
        let mut sorted: Vec<&KnowledgeEntry> = self.entries.iter().collect();
        sorted.sort_by(|a, b| b.times_referenced.cmp(&a.times_referenced));
        sorted.into_iter().take(limit).collect()
    }

    pub fn stats(&self) -> KnowledgeStats {
        let total = self.entries.len();
        let avg_effectiveness = if total == 0 {
            0.0
        } else {
            self.entries.iter().map(|e| e.effectiveness).sum::<f64>() / total as f64
        };

        let mut domain_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for entry in &self.entries {
            *domain_counts.entry(entry.problem_domain.clone()).or_default() += 1;
        }

        KnowledgeStats {
            total_entries: total,
            avg_effectiveness,
            domain_distribution: domain_counts,
        }
    }
}

impl Default for KnowledgeBase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeStats {
    pub total_entries: usize,
    pub avg_effectiveness: f64,
    pub domain_distribution: std::collections::HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use crate::{Domain, EnterpriseSolver, Severity};

    #[test]
    fn test_knowledge_capture() {
        let mut solver = EnterpriseSolver::new();
        solver.solve("Test issue", "Something broke", Domain::Infrastructure, Severity::High);
        assert_eq!(solver.knowledge_base.entries.len(), 1);
    }

    #[test]
    fn test_knowledge_search() {
        let mut solver = EnterpriseSolver::new();
        solver.solve("Database timeout", "DB connection pool exhausted", Domain::Infrastructure, Severity::High);
        solver.solve("Login slow", "Auth service latency", Domain::Performance, Severity::Medium);
        let results = solver.knowledge_base.search("database");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_knowledge_stats() {
        let mut solver = EnterpriseSolver::new();
        solver.solve("A", "Desc", Domain::Security, Severity::Critical);
        solver.solve("B", "Desc", Domain::Security, Severity::High);
        let stats = solver.knowledge_base.stats();
        assert_eq!(stats.total_entries, 2);
        assert!(stats.avg_effectiveness > 0.0);
    }
}
