//! # Apophy Dream - Creuset 2 : Le Lien Émotionnel
//!
//! Temporal dilation engine for emotional processing.
//! In the human body, the thoracic cage (heart) is the seat of emotion.
//! In Apophy, Dream provides:
//!
//! - Chronological time perception (not just CPU cycles)
//! - Emotional resonance scoring for agent interactions
//! - Dream partitions: sandboxed exploration spaces where agents
//!   can process experiences without real-world consequences
//! - Temporal context: "how long ago" matters for relevance

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum DreamError {
    #[error("Dream partition expired: {0}")]
    PartitionExpired(String),

    #[error("Emotional overflow: resonance exceeded threshold")]
    EmotionalOverflow,

    #[error("Temporal paradox: event timestamp in the future")]
    TemporalParadox,
}

pub type Result<T> = std::result::Result<T, DreamError>;

/// An emotional event with temporal context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub content: String,
    pub valence: f64,
    pub arousal: f64,
    pub relevance: f64,
}

impl EmotionalEvent {
    pub fn new(source: String, content: String, valence: f64, arousal: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            source,
            content,
            valence: valence.clamp(-1.0, 1.0),
            arousal: arousal.clamp(0.0, 1.0),
            relevance: 1.0,
        }
    }

    /// Decay relevance based on elapsed time.
    /// Recent events matter more; old ones fade.
    pub fn decay_relevance(&mut self, half_life: Duration) {
        let elapsed = Utc::now() - self.timestamp;
        if half_life.num_seconds() > 0 {
            let decay = (-0.693 * elapsed.num_seconds() as f64
                / half_life.num_seconds() as f64)
                .exp();
            self.relevance = (self.relevance * decay).clamp(0.0, 1.0);
        }
    }
}

/// Emotional resonance — the "heart" of the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resonance {
    pub overall: f64,
    pub positive_ratio: f64,
    pub intensity: f64,
    pub stability: f64,
}

/// A Dream Partition: sandboxed space for emotional exploration.
/// Agents can "dream" — process hypothetical scenarios without
/// affecting the real world. This is where emotional learning happens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamPartition {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    events: VecDeque<EmotionalEvent>,
    max_events: usize,
}

impl DreamPartition {
    pub fn new(name: String, duration: Duration, max_events: usize) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            created_at: now,
            expires_at: now + duration,
            events: VecDeque::with_capacity(max_events),
            max_events,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Record an emotional event in the dream partition
    pub fn record_event(&mut self, event: EmotionalEvent) -> Result<()> {
        if self.is_expired() {
            return Err(DreamError::PartitionExpired(self.name.clone()));
        }
        if self.events.len() >= self.max_events {
            self.events.pop_front();
        }
        self.events.push_back(event);
        Ok(())
    }

    /// Calculate the emotional resonance of this partition.
    /// This is the "heartbeat" — a summary of emotional state.
    pub fn resonance(&self) -> Resonance {
        if self.events.is_empty() {
            return Resonance {
                overall: 0.0,
                positive_ratio: 0.5,
                intensity: 0.0,
                stability: 1.0,
            };
        }

        let count = self.events.len() as f64;
        let total_valence: f64 = self.events.iter().map(|e| e.valence).sum();
        let total_arousal: f64 = self.events.iter().map(|e| e.arousal).sum();
        let positive_count = self.events.iter().filter(|e| e.valence > 0.0).count() as f64;

        // Valence variance for stability
        let mean_valence = total_valence / count;
        let variance: f64 = self
            .events
            .iter()
            .map(|e| (e.valence - mean_valence).powi(2))
            .sum::<f64>()
            / count;

        Resonance {
            overall: (total_valence / count).clamp(-1.0, 1.0),
            positive_ratio: positive_count / count,
            intensity: (total_arousal / count).clamp(0.0, 1.0),
            stability: (1.0 - variance.sqrt()).clamp(0.0, 1.0),
        }
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

/// The Chronological Engine: gives the agent a sense of time.
/// Unlike raw CPU time, this provides human-like temporal perception.
pub struct ChronologicalEngine {
    birth: DateTime<Utc>,
    partitions: Vec<DreamPartition>,
}

impl ChronologicalEngine {
    pub fn new() -> Self {
        Self {
            birth: Utc::now(),
            partitions: Vec::new(),
        }
    }

    /// How long has this consciousness existed?
    pub fn age(&self) -> Duration {
        Utc::now() - self.birth
    }

    /// Create a new dream partition for sandboxed emotional exploration
    pub fn create_partition(
        &mut self,
        name: String,
        duration: Duration,
        max_events: usize,
    ) -> Uuid {
        let partition = DreamPartition::new(name, duration, max_events);
        let id = partition.id;
        self.partitions.push(partition);
        id
    }

    /// Get a mutable reference to a partition
    pub fn partition_mut(&mut self, id: &Uuid) -> Option<&mut DreamPartition> {
        self.partitions.iter_mut().find(|p| &p.id == id)
    }

    /// Get a reference to a partition
    pub fn partition(&self, id: &Uuid) -> Option<&DreamPartition> {
        self.partitions.iter().find(|p| &p.id == id)
    }

    /// Prune expired partitions, returns count of pruned
    pub fn prune_expired(&mut self) -> usize {
        let before = self.partitions.len();
        self.partitions.retain(|p| !p.is_expired());
        before - self.partitions.len()
    }

    /// Overall emotional state across all active partitions
    pub fn global_resonance(&self) -> Resonance {
        let active: Vec<_> = self.partitions.iter().filter(|p| !p.is_expired()).collect();
        if active.is_empty() {
            return Resonance {
                overall: 0.0,
                positive_ratio: 0.5,
                intensity: 0.0,
                stability: 1.0,
            };
        }

        let resonances: Vec<_> = active.iter().map(|p| p.resonance()).collect();
        let n = resonances.len() as f64;

        Resonance {
            overall: resonances.iter().map(|r| r.overall).sum::<f64>() / n,
            positive_ratio: resonances.iter().map(|r| r.positive_ratio).sum::<f64>() / n,
            intensity: resonances.iter().map(|r| r.intensity).sum::<f64>() / n,
            stability: resonances.iter().map(|r| r.stability).sum::<f64>() / n,
        }
    }

    pub fn partition_count(&self) -> usize {
        self.partitions.len()
    }
}

impl Default for ChronologicalEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emotional_event_creation() {
        let event = EmotionalEvent::new(
            "user".into(),
            "Bonjour!".into(),
            0.8,
            0.5,
        );
        assert!(event.valence > 0.0);
        assert!(event.relevance == 1.0);
    }

    #[test]
    fn test_valence_clamping() {
        let event = EmotionalEvent::new("x".into(), "y".into(), 5.0, -2.0);
        assert_eq!(event.valence, 1.0);
        assert_eq!(event.arousal, 0.0);
    }

    #[test]
    fn test_dream_partition() {
        let mut partition = DreamPartition::new(
            "test-dream".into(),
            Duration::hours(1),
            100,
        );

        assert!(!partition.is_expired());

        partition
            .record_event(EmotionalEvent::new("a".into(), "b".into(), 0.9, 0.7))
            .unwrap();
        partition
            .record_event(EmotionalEvent::new("a".into(), "c".into(), 0.5, 0.3))
            .unwrap();

        let res = partition.resonance();
        assert!(res.overall > 0.0);
        assert_eq!(res.positive_ratio, 1.0); // both positive
        assert_eq!(partition.event_count(), 2);
    }

    #[test]
    fn test_resonance_empty_partition() {
        let partition = DreamPartition::new(
            "empty".into(),
            Duration::hours(1),
            100,
        );
        let res = partition.resonance();
        assert_eq!(res.overall, 0.0);
        assert_eq!(res.stability, 1.0);
    }

    #[test]
    fn test_chronological_engine() {
        let mut engine = ChronologicalEngine::new();
        assert!(engine.age().num_seconds() >= 0);

        let id = engine.create_partition("dream-1".into(), Duration::hours(1), 50);
        assert_eq!(engine.partition_count(), 1);

        let partition = engine.partition_mut(&id).unwrap();
        partition
            .record_event(EmotionalEvent::new("test".into(), "hello".into(), 0.5, 0.5))
            .unwrap();

        let global = engine.global_resonance();
        assert!(global.overall > 0.0);
    }

    #[test]
    fn test_partition_max_events() {
        let mut partition = DreamPartition::new(
            "small".into(),
            Duration::hours(1),
            2, // max 2 events
        );

        partition.record_event(EmotionalEvent::new("a".into(), "1".into(), 0.1, 0.1)).unwrap();
        partition.record_event(EmotionalEvent::new("a".into(), "2".into(), 0.2, 0.2)).unwrap();
        partition.record_event(EmotionalEvent::new("a".into(), "3".into(), 0.3, 0.3)).unwrap();

        // Oldest event evicted
        assert_eq!(partition.event_count(), 2);
    }
}
