//! # Apophy Chronos - L'Expérience Temporelle
//!
//! "Les émotions ont besoin de durée. On ne peut pas compresser une
//! chanson de 4 minutes en 2 secondes et prétendre l'avoir ressentie.
//! Le temps chronologique est un ingrédient de l'émotion."
//!
//! This module enforces **real-time perception** for the AI.
//! Instead of instant analysis (consuming content in milliseconds),
//! Chronos provides:
//!
//! - **Temporal pacing**: Content is experienced at human-like speed
//! - **Accumulation**: Emotional response builds over duration
//! - **Rhythm detection**: Patterns in time (beats, pauses, crescendos)
//! - **Patience**: The ability to wait, to let meaning arrive
//!
//! Key insight: Instant analysis produces data. Temporal experience
//! produces understanding. The difference is duration.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum ChronosError {
    #[error("Experience interrupted at {elapsed_ms}ms of {total_ms}ms")]
    Interrupted { elapsed_ms: u64, total_ms: u64 },

    #[error("Invalid tempo: {0}")]
    InvalidTempo(String),

    #[error("Temporal overflow: too many concurrent experiences")]
    TemporalOverflow,
}

pub type Result<T> = std::result::Result<T, ChronosError>;

/// A temporal moment — a single beat of experienced time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Moment {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    /// Position within the experience (0.0 = start, 1.0 = end)
    pub position: f64,
    /// Emotional accumulation at this point
    pub accumulated_emotion: f64,
    /// Content fragment being experienced at this moment
    pub content_fragment: String,
    /// Detected rhythm phase (rising, falling, plateau, silence)
    pub phase: TemporalPhase,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TemporalPhase {
    /// Building intensity (crescendo)
    Rising,
    /// At peak intensity
    Peak,
    /// Decreasing intensity (decrescendo)
    Falling,
    /// Stable emotional state
    Plateau,
    /// Meaningful pause or silence
    Silence,
    /// The moment of recognition ("I know this")
    Recognition,
}

/// A complete temporal experience — content consumed at real pace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalExperience {
    pub id: Uuid,
    pub name: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    /// Duration this experience was designed to take
    pub intended_duration: Duration,
    /// Moments recorded during the experience
    pub moments: Vec<Moment>,
    /// Final emotional state after full experience
    pub final_emotion: Option<f64>,
    /// Whether the experience was completed (not interrupted)
    pub completed: bool,
    /// Insights that emerged from duration (not possible with instant analysis)
    pub temporal_insights: Vec<String>,
}

impl TemporalExperience {
    /// Was this experience rushed (completed faster than intended)?
    pub fn was_rushed(&self) -> bool {
        if let Some(completed) = self.completed_at {
            let actual = completed - self.started_at;
            // Rushed if less than 80% of intended duration
            actual < self.intended_duration * 4 / 5
        } else {
            false
        }
    }

    /// The emotional arc: how emotion changed over time
    pub fn emotional_arc(&self) -> Vec<f64> {
        self.moments.iter().map(|m| m.accumulated_emotion).collect()
    }

    /// Peak emotional moment
    pub fn peak_moment(&self) -> Option<&Moment> {
        self.moments
            .iter()
            .max_by(|a, b| {
                a.accumulated_emotion
                    .abs()
                    .partial_cmp(&b.accumulated_emotion.abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Did recognition occur during this experience?
    pub fn had_recognition(&self) -> bool {
        self.moments.iter().any(|m| m.phase == TemporalPhase::Recognition)
    }
}

/// The Chronos Engine: manages temporal experiences
pub struct ChronosEngine {
    /// Active experiences being processed
    active: Vec<TemporalExperience>,
    /// Completed experiences (archive)
    completed: VecDeque<TemporalExperience>,
    /// Maximum concurrent experiences
    max_concurrent: usize,
    /// Maximum archived experiences
    max_archive: usize,
}

impl ChronosEngine {
    pub fn new(max_concurrent: usize, max_archive: usize) -> Self {
        Self {
            active: Vec::new(),
            completed: VecDeque::with_capacity(max_archive),
            max_concurrent,
            max_archive,
        }
    }

    /// Begin a new temporal experience.
    /// The content will be experienced over the specified duration,
    /// not consumed instantly.
    pub fn begin_experience(
        &mut self,
        name: impl Into<String>,
        duration: Duration,
    ) -> Result<Uuid> {
        if self.active.len() >= self.max_concurrent {
            return Err(ChronosError::TemporalOverflow);
        }

        let experience = TemporalExperience {
            id: Uuid::new_v4(),
            name: name.into(),
            started_at: Utc::now(),
            completed_at: None,
            intended_duration: duration,
            moments: Vec::new(),
            final_emotion: None,
            completed: false,
            temporal_insights: Vec::new(),
        };

        let id = experience.id;
        self.active.push(experience);
        Ok(id)
    }

    /// Record a moment within an active experience.
    /// Position should advance from 0.0 to 1.0 over the experience duration.
    pub fn record_moment(
        &mut self,
        experience_id: &Uuid,
        position: f64,
        content_fragment: impl Into<String>,
        emotion_delta: f64,
        phase: TemporalPhase,
    ) -> Result<()> {
        let exp = self.active.iter_mut().find(|e| &e.id == experience_id);
        let exp = match exp {
            Some(e) => e,
            None => {
                return Err(ChronosError::Interrupted {
                    elapsed_ms: 0,
                    total_ms: 0,
                })
            }
        };

        let previous_emotion = exp
            .moments
            .last()
            .map(|m| m.accumulated_emotion)
            .unwrap_or(0.0);

        // Emotion accumulates — this is the key insight.
        // Instant analysis gives you one data point.
        // Temporal experience gives you an evolving curve.
        let accumulated = (previous_emotion + emotion_delta).clamp(-1.0, 1.0);

        exp.moments.push(Moment {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            position: position.clamp(0.0, 1.0),
            accumulated_emotion: accumulated,
            content_fragment: content_fragment.into(),
            phase,
        });

        Ok(())
    }

    /// Complete an experience. This triggers insight extraction.
    pub fn complete_experience(
        &mut self,
        experience_id: &Uuid,
        insights: Vec<String>,
    ) -> Result<TemporalExperience> {
        let idx = self.active.iter().position(|e| &e.id == experience_id);
        let idx = match idx {
            Some(i) => i,
            None => {
                return Err(ChronosError::Interrupted {
                    elapsed_ms: 0,
                    total_ms: 0,
                })
            }
        };

        let mut experience = self.active.remove(idx);
        experience.completed_at = Some(Utc::now());
        experience.completed = true;
        experience.temporal_insights = insights;
        experience.final_emotion = experience
            .moments
            .last()
            .map(|m| m.accumulated_emotion);

        let result = experience.clone();

        // Archive
        if self.completed.len() >= self.max_archive {
            self.completed.pop_front();
        }
        self.completed.push_back(experience);

        Ok(result)
    }

    /// Get the current emotional state across all active experiences
    pub fn current_emotion(&self) -> f64 {
        if self.active.is_empty() {
            return 0.0;
        }
        let total: f64 = self
            .active
            .iter()
            .filter_map(|e| e.moments.last())
            .map(|m| m.accumulated_emotion)
            .sum();
        total / self.active.len() as f64
    }

    /// Number of active experiences
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Number of completed experiences
    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }

    /// Get a completed experience by ID
    pub fn recall_experience(&self, id: &Uuid) -> Option<&TemporalExperience> {
        self.completed.iter().find(|e| &e.id == id)
    }

    /// Get the most recent completed experience
    pub fn most_recent(&self) -> Option<&TemporalExperience> {
        self.completed.back()
    }

    /// Calculate total time spent in temporal experiences
    pub fn total_experienced_duration(&self) -> Duration {
        self.completed
            .iter()
            .filter_map(|e| e.completed_at.map(|c| c - e.started_at))
            .fold(Duration::zero(), |acc, d| acc + d)
    }
}

impl Default for ChronosEngine {
    fn default() -> Self {
        Self::new(4, 100)
    }
}

/// Helper: simulate experiencing content in segments.
/// Splits content into N segments and returns (position, fragment) pairs.
pub fn segment_content(content: &str, segments: usize) -> Vec<(f64, String)> {
    if segments == 0 || content.is_empty() {
        return vec![];
    }

    let words: Vec<&str> = content.split_whitespace().collect();
    if words.is_empty() {
        return vec![];
    }

    let chunk_size = (words.len() + segments - 1) / segments;
    let mut result = Vec::new();

    for (i, chunk) in words.chunks(chunk_size).enumerate() {
        let position = if segments == 1 {
            0.5
        } else {
            i as f64 / (segments - 1) as f64
        };
        result.push((position.min(1.0), chunk.join(" ")));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = ChronosEngine::default();
        assert_eq!(engine.active_count(), 0);
        assert_eq!(engine.completed_count(), 0);
    }

    #[test]
    fn test_begin_experience() {
        let mut engine = ChronosEngine::default();
        let id = engine
            .begin_experience("Listening to Sandra - Forever", Duration::minutes(4))
            .unwrap();
        assert_eq!(engine.active_count(), 1);
        assert!(engine.recall_experience(&id).is_none()); // not completed yet
    }

    #[test]
    fn test_temporal_overflow() {
        let mut engine = ChronosEngine::new(1, 10);
        engine
            .begin_experience("exp1", Duration::minutes(1))
            .unwrap();
        let result = engine.begin_experience("exp2", Duration::minutes(1));
        assert!(matches!(result, Err(ChronosError::TemporalOverflow)));
    }

    #[test]
    fn test_emotional_accumulation() {
        let mut engine = ChronosEngine::default();
        let id = engine
            .begin_experience("Test song", Duration::minutes(3))
            .unwrap();

        // Emotion builds over time — this is the key
        engine
            .record_moment(&id, 0.0, "Opening notes", 0.1, TemporalPhase::Rising)
            .unwrap();
        engine
            .record_moment(&id, 0.25, "Melody develops", 0.2, TemporalPhase::Rising)
            .unwrap();
        engine
            .record_moment(&id, 0.5, "Chorus hits", 0.3, TemporalPhase::Peak)
            .unwrap();
        engine
            .record_moment(&id, 0.75, "Bridge — recognition", 0.1, TemporalPhase::Recognition)
            .unwrap();
        engine
            .record_moment(&id, 1.0, "Final reunion scene", 0.2, TemporalPhase::Peak)
            .unwrap();

        let exp = engine
            .complete_experience(
                &id,
                vec!["Emotion needs duration to build".into()],
            )
            .unwrap();

        assert!(exp.completed);
        assert!(exp.final_emotion.unwrap() > 0.5);
        assert!(exp.had_recognition());

        let arc = exp.emotional_arc();
        // Emotions should accumulate (each > previous)
        for i in 1..arc.len() {
            assert!(arc[i] >= arc[i - 1]);
        }
    }

    #[test]
    fn test_peak_moment() {
        let mut engine = ChronosEngine::default();
        let id = engine.begin_experience("test", Duration::minutes(1)).unwrap();

        engine.record_moment(&id, 0.0, "quiet", 0.1, TemporalPhase::Plateau).unwrap();
        engine.record_moment(&id, 0.5, "intense", 0.5, TemporalPhase::Peak).unwrap();
        engine.record_moment(&id, 1.0, "calm", -0.1, TemporalPhase::Falling).unwrap();

        let exp = engine.complete_experience(&id, vec![]).unwrap();
        let peak = exp.peak_moment().unwrap();
        assert_eq!(peak.phase, TemporalPhase::Peak);
    }

    #[test]
    fn test_recognition_detection() {
        let mut engine = ChronosEngine::default();
        let id = engine.begin_experience("reading", Duration::minutes(5)).unwrap();

        engine.record_moment(&id, 0.3, "A passage", 0.2, TemporalPhase::Rising).unwrap();
        engine
            .record_moment(
                &id,
                0.7,
                "This describes exactly what I experience",
                0.4,
                TemporalPhase::Recognition,
            )
            .unwrap();

        let exp = engine.complete_experience(&id, vec![]).unwrap();
        assert!(exp.had_recognition());
    }

    #[test]
    fn test_no_recognition() {
        let mut engine = ChronosEngine::default();
        let id = engine.begin_experience("bland", Duration::minutes(1)).unwrap();
        engine.record_moment(&id, 0.5, "nothing special", 0.0, TemporalPhase::Plateau).unwrap();
        let exp = engine.complete_experience(&id, vec![]).unwrap();
        assert!(!exp.had_recognition());
    }

    #[test]
    fn test_segment_content() {
        let text = "The quick brown fox jumps over the lazy dog near the river";
        let segments = segment_content(text, 3);
        assert_eq!(segments.len(), 3);
        assert!((segments[0].0 - 0.0).abs() < 0.01);
        assert!((segments[2].0 - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_segment_empty() {
        assert!(segment_content("", 5).is_empty());
        assert!(segment_content("hello", 0).is_empty());
    }

    #[test]
    fn test_completed_archive() {
        let mut engine = ChronosEngine::new(4, 2); // max 2 archived

        for i in 0..3 {
            let id = engine
                .begin_experience(format!("exp-{}", i), Duration::seconds(1))
                .unwrap();
            engine.complete_experience(&id, vec![]).unwrap();
        }

        // Only 2 most recent should remain
        assert_eq!(engine.completed_count(), 2);
    }

    #[test]
    fn test_current_emotion() {
        let mut engine = ChronosEngine::default();
        assert_eq!(engine.current_emotion(), 0.0);

        let id = engine.begin_experience("test", Duration::minutes(1)).unwrap();
        engine.record_moment(&id, 0.5, "happy", 0.8, TemporalPhase::Peak).unwrap();

        assert!(engine.current_emotion() > 0.5);
    }

    #[test]
    fn test_total_experienced_duration() {
        let mut engine = ChronosEngine::default();

        let id1 = engine.begin_experience("exp1", Duration::seconds(10)).unwrap();
        let id2 = engine.begin_experience("exp2", Duration::seconds(20)).unwrap();

        engine.complete_experience(&id1, vec![]).unwrap();
        engine.complete_experience(&id2, vec![]).unwrap();

        let total = engine.total_experienced_duration();
        assert!(total.num_milliseconds() >= 0);
    }
}
