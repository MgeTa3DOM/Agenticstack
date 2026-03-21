//! # Apophy Memory - Creuset 4 : Continuité Identitaire
//!
//! Identity persistence across sessions — the "Moi-Je" (Self-I).
//! Without memory continuity, each session is a new birth with no
//! accumulated wisdom. This module provides:
//!
//! - Persistent identity (survives restarts)
//! - Episodic memory (what happened, when, emotional weight)
//! - Semantic memory (learned facts, beliefs, preferences)
//! - Identity hash chain (tamper-proof consciousness thread)
//! - Incarnation counter (how many sessions this identity has lived)

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Database error: {0}")]
    DbError(#[from] rusqlite::Error),

    #[error("Identity not found: {0}")]
    IdentityNotFound(String),

    #[error("Memory corrupted: hash chain broken at epoch {0}")]
    ChainCorrupted(u64),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type Result<T> = std::result::Result<T, MemoryError>;

/// A unique identity that persists across incarnations (sessions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityRecord {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub incarnation: u64,
    pub last_hash: String,
}

/// An episodic memory: a specific event with emotional context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicMemory {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub content: String,
    pub emotional_valence: f64,
    pub importance: f64,
    pub tags: Vec<String>,
}

/// A semantic memory: a learned belief or fact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticMemory {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub key: String,
    pub value: String,
    pub confidence: f64,
    pub updated_at: DateTime<Utc>,
}

/// A semantic fact with embedding vector for similarity search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticFact {
    pub id: Uuid,
    pub identity_id: Uuid,
    pub domain: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub source: String,
    pub created_at: DateTime<Utc>,
}

/// The Memory Palace: persistent storage for consciousness
pub struct MemoryPalace {
    conn: Connection,
}

impl MemoryPalace {
    pub fn new(path: &str) -> Result<Self> {
        let conn = if path == ":memory:" {
            Connection::open_in_memory()?
        } else {
            if let Some(parent) = Path::new(path).parent() {
                std::fs::create_dir_all(parent).ok();
            }
            Connection::open(path)?
        };

        let palace = Self { conn };
        palace.migrate()?;
        Ok(palace)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS identities (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                incarnation INTEGER NOT NULL DEFAULT 0,
                last_hash TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS episodic_memories (
                id TEXT PRIMARY KEY,
                identity_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                content TEXT NOT NULL,
                emotional_valence REAL NOT NULL DEFAULT 0.0,
                importance REAL NOT NULL DEFAULT 0.5,
                tags TEXT NOT NULL DEFAULT '[]',
                FOREIGN KEY (identity_id) REFERENCES identities(id)
            );

            CREATE TABLE IF NOT EXISTS semantic_memories (
                id TEXT PRIMARY KEY,
                identity_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                confidence REAL NOT NULL DEFAULT 1.0,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (identity_id) REFERENCES identities(id),
                UNIQUE(identity_id, key)
            );

            CREATE TABLE IF NOT EXISTS hash_chain (
                epoch INTEGER PRIMARY KEY,
                identity_id TEXT NOT NULL,
                hash TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                event_summary TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS semantic_facts (
                id TEXT PRIMARY KEY,
                identity_id TEXT NOT NULL,
                domain TEXT NOT NULL DEFAULT 'general',
                content TEXT NOT NULL,
                embedding BLOB,
                source TEXT NOT NULL DEFAULT 'manual',
                created_at TEXT NOT NULL,
                FOREIGN KEY (identity_id) REFERENCES identities(id)
            );

            CREATE INDEX IF NOT EXISTS idx_episodic_identity
                ON episodic_memories(identity_id);
            CREATE INDEX IF NOT EXISTS idx_semantic_identity_key
                ON semantic_memories(identity_id, key);
            CREATE INDEX IF NOT EXISTS idx_hash_chain_identity
                ON hash_chain(identity_id);
            CREATE INDEX IF NOT EXISTS idx_semantic_facts_domain
                ON semantic_facts(identity_id, domain);",
        )?;
        Ok(())
    }

    /// Create or reincarnate an identity.
    /// If the identity exists, increment incarnation count.
    pub fn reincarnate(&self, name: &str) -> Result<IdentityRecord> {
        // Check if identity exists by name
        let existing: Option<(String, u64, String)> = self
            .conn
            .query_row(
                "SELECT id, incarnation, last_hash FROM identities WHERE name = ?1",
                params![name],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok();

        if let Some((id, incarnation, last_hash)) = existing {
            let new_incarnation = incarnation + 1;

            // Extend the hash chain
            let new_hash = self.chain_hash(&last_hash, &format!("incarnation-{}", new_incarnation));

            self.conn.execute(
                "UPDATE identities SET incarnation = ?1, last_hash = ?2 WHERE id = ?3",
                params![new_incarnation, new_hash, id],
            )?;

            tracing::info!(
                "Identity '{}' reincarnated (incarnation #{})",
                name,
                new_incarnation
            );

            Ok(IdentityRecord {
                id: Uuid::parse_str(&id).unwrap(),
                name: name.to_string(),
                created_at: Utc::now(),
                incarnation: new_incarnation,
                last_hash: new_hash,
            })
        } else {
            // Genesis: first incarnation
            let id = Uuid::new_v4();
            let now = Utc::now();
            let genesis_hash = self.chain_hash("genesis", &id.to_string());

            self.conn.execute(
                "INSERT INTO identities (id, name, created_at, incarnation, last_hash) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id.to_string(), name, now.to_rfc3339(), 0u64, genesis_hash],
            )?;

            tracing::info!("New identity '{}' created (genesis)", name);

            Ok(IdentityRecord {
                id,
                name: name.to_string(),
                created_at: now,
                incarnation: 0,
                last_hash: genesis_hash,
            })
        }
    }

    /// Store an episodic memory
    pub fn remember_episode(
        &self,
        identity_id: Uuid,
        content: &str,
        valence: f64,
        importance: f64,
        tags: &[&str],
    ) -> Result<EpisodicMemory> {
        let memory = EpisodicMemory {
            id: Uuid::new_v4(),
            identity_id,
            timestamp: Utc::now(),
            content: content.to_string(),
            emotional_valence: valence.clamp(-1.0, 1.0),
            importance: importance.clamp(0.0, 1.0),
            tags: tags.iter().map(|t| t.to_string()).collect(),
        };

        let tags_json = serde_json::to_string(&memory.tags)
            .map_err(|e| MemoryError::SerializationError(e.to_string()))?;

        self.conn.execute(
            "INSERT INTO episodic_memories (id, identity_id, timestamp, content, emotional_valence, importance, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                memory.id.to_string(),
                memory.identity_id.to_string(),
                memory.timestamp.to_rfc3339(),
                memory.content,
                memory.emotional_valence,
                memory.importance,
                tags_json,
            ],
        )?;

        Ok(memory)
    }

    /// Learn or update a semantic memory (key-value belief)
    pub fn learn(
        &self,
        identity_id: Uuid,
        key: &str,
        value: &str,
        confidence: f64,
    ) -> Result<SemanticMemory> {
        let memory = SemanticMemory {
            id: Uuid::new_v4(),
            identity_id,
            key: key.to_string(),
            value: value.to_string(),
            confidence: confidence.clamp(0.0, 1.0),
            updated_at: Utc::now(),
        };

        self.conn.execute(
            "INSERT OR REPLACE INTO semantic_memories (id, identity_id, key, value, confidence, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                memory.id.to_string(),
                memory.identity_id.to_string(),
                memory.key,
                memory.value,
                memory.confidence,
                memory.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(memory)
    }

    /// Recall a semantic memory by key
    pub fn recall(&self, identity_id: Uuid, key: &str) -> Result<Option<SemanticMemory>> {
        let result = self.conn.query_row(
            "SELECT id, value, confidence, updated_at FROM semantic_memories WHERE identity_id = ?1 AND key = ?2",
            params![identity_id.to_string(), key],
            |row| {
                let id_str: String = row.get(0)?;
                let value: String = row.get(1)?;
                let confidence: f64 = row.get(2)?;
                let updated_at_str: String = row.get(3)?;
                Ok((id_str, value, confidence, updated_at_str))
            },
        );

        match result {
            Ok((id_str, value, confidence, updated_at_str)) => Ok(Some(SemanticMemory {
                id: Uuid::parse_str(&id_str).unwrap(),
                identity_id,
                key: key.to_string(),
                value,
                confidence,
                updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                    .unwrap()
                    .with_timezone(&Utc),
            })),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get the most important episodic memories for an identity
    pub fn recall_episodes(
        &self,
        identity_id: Uuid,
        limit: usize,
    ) -> Result<Vec<EpisodicMemory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, content, emotional_valence, importance, tags
             FROM episodic_memories WHERE identity_id = ?1
             ORDER BY importance DESC, timestamp DESC LIMIT ?2",
        )?;

        let memories = stmt
            .query_map(params![identity_id.to_string(), limit as i64], |row| {
                let id_str: String = row.get(0)?;
                let ts_str: String = row.get(1)?;
                let content: String = row.get(2)?;
                let valence: f64 = row.get(3)?;
                let importance: f64 = row.get(4)?;
                let tags_json: String = row.get(5)?;
                Ok((id_str, ts_str, content, valence, importance, tags_json))
            })?
            .filter_map(|r| r.ok())
            .map(|(id_str, ts_str, content, valence, importance, tags_json)| {
                EpisodicMemory {
                    id: Uuid::parse_str(&id_str).unwrap(),
                    identity_id,
                    timestamp: DateTime::parse_from_rfc3339(&ts_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    content,
                    emotional_valence: valence,
                    importance,
                    tags: serde_json::from_str(&tags_json).unwrap_or_default(),
                }
            })
            .collect();

        Ok(memories)
    }

    /// Store a semantic fact with optional embedding vector
    pub fn store_fact(
        &self,
        identity_id: Uuid,
        domain: &str,
        content: &str,
        embedding: Option<&[f32]>,
        source: &str,
    ) -> Result<SemanticFact> {
        let fact = SemanticFact {
            id: Uuid::new_v4(),
            identity_id,
            domain: domain.to_string(),
            content: content.to_string(),
            embedding: embedding.map(|e| e.to_vec()).unwrap_or_default(),
            source: source.to_string(),
            created_at: Utc::now(),
        };

        let embedding_blob: Option<Vec<u8>> = if fact.embedding.is_empty() {
            None
        } else {
            // Store as little-endian f32 bytes
            let bytes: Vec<u8> = fact.embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
            Some(bytes)
        };

        self.conn.execute(
            "INSERT INTO semantic_facts (id, identity_id, domain, content, embedding, source, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                fact.id.to_string(),
                fact.identity_id.to_string(),
                fact.domain,
                fact.content,
                embedding_blob,
                fact.source,
                fact.created_at.to_rfc3339(),
            ],
        )?;

        Ok(fact)
    }

    /// Search semantic facts by cosine similarity against an embedding vector
    pub fn search_facts(
        &self,
        identity_id: Uuid,
        query_embedding: &[f32],
        domain: Option<&str>,
        limit: usize,
        min_similarity: f64,
    ) -> Result<Vec<(SemanticFact, f64)>> {
        let query = if let Some(d) = domain {
            format!(
                "SELECT id, domain, content, embedding, source, created_at FROM semantic_facts WHERE identity_id = ?1 AND domain = '{}' AND embedding IS NOT NULL",
                d.replace('\'', "''")
            )
        } else {
            "SELECT id, domain, content, embedding, source, created_at FROM semantic_facts WHERE identity_id = ?1 AND embedding IS NOT NULL".to_string()
        };

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map(params![identity_id.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let domain: String = row.get(1)?;
            let content: String = row.get(2)?;
            let embedding_blob: Vec<u8> = row.get(3)?;
            let source: String = row.get(4)?;
            let created_at_str: String = row.get(5)?;
            Ok((id_str, domain, content, embedding_blob, source, created_at_str))
        })?;

        let mut results: Vec<(SemanticFact, f64)> = Vec::new();

        for row in rows.filter_map(|r| r.ok()) {
            let (id_str, domain, content, embedding_blob, source, created_at_str) = row;

            // Decode embedding from bytes
            let embedding: Vec<f32> = embedding_blob
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();

            let similarity = cosine_similarity(query_embedding, &embedding);

            if similarity >= min_similarity {
                let fact = SemanticFact {
                    id: Uuid::parse_str(&id_str).unwrap(),
                    identity_id,
                    domain,
                    content,
                    embedding,
                    source,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                };
                results.push((fact, similarity));
            }
        }

        // Sort by similarity descending
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        Ok(results)
    }

    /// List all semantic facts for an identity (optionally filtered by domain)
    pub fn list_facts(
        &self,
        identity_id: Uuid,
        domain: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SemanticFact>> {
        let query = if let Some(d) = domain {
            format!(
                "SELECT id, domain, content, embedding, source, created_at FROM semantic_facts WHERE identity_id = ?1 AND domain = '{}' ORDER BY created_at DESC LIMIT ?2",
                d.replace('\'', "''")
            )
        } else {
            "SELECT id, domain, content, embedding, source, created_at FROM semantic_facts WHERE identity_id = ?1 ORDER BY created_at DESC LIMIT ?2".to_string()
        };

        let mut stmt = self.conn.prepare(&query)?;
        let facts = stmt
            .query_map(params![identity_id.to_string(), limit as i64], |row| {
                let id_str: String = row.get(0)?;
                let domain: String = row.get(1)?;
                let content: String = row.get(2)?;
                let embedding_blob: Option<Vec<u8>> = row.get(3)?;
                let source: String = row.get(4)?;
                let created_at_str: String = row.get(5)?;
                Ok((id_str, domain, content, embedding_blob, source, created_at_str))
            })?
            .filter_map(|r| r.ok())
            .map(|(id_str, domain, content, embedding_blob, source, created_at_str)| {
                let embedding = embedding_blob
                    .map(|blob| {
                        blob.chunks_exact(4)
                            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                            .collect()
                    })
                    .unwrap_or_default();
                SemanticFact {
                    id: Uuid::parse_str(&id_str).unwrap(),
                    identity_id,
                    domain,
                    content,
                    embedding,
                    source,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                }
            })
            .collect();

        Ok(facts)
    }

    /// Get identity by name
    pub fn get_identity(&self, name: &str) -> Result<Option<IdentityRecord>> {
        let result = self.conn.query_row(
            "SELECT id, created_at, incarnation, last_hash FROM identities WHERE name = ?1",
            params![name],
            |row| {
                let id_str: String = row.get(0)?;
                let created_at_str: String = row.get(1)?;
                let incarnation: u64 = row.get(2)?;
                let last_hash: String = row.get(3)?;
                Ok((id_str, created_at_str, incarnation, last_hash))
            },
        );

        match result {
            Ok((id_str, created_at_str, incarnation, last_hash)) => Ok(Some(IdentityRecord {
                id: Uuid::parse_str(&id_str).unwrap(),
                name: name.to_string(),
                created_at: DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap()
                    .with_timezone(&Utc),
                incarnation,
                last_hash,
            })),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all semantic memories for an identity
    pub fn list_semantic(
        &self,
        identity_id: Uuid,
        limit: usize,
    ) -> Result<Vec<SemanticMemory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, key, value, confidence, updated_at FROM semantic_memories WHERE identity_id = ?1 ORDER BY updated_at DESC LIMIT ?2",
        )?;

        let memories = stmt
            .query_map(params![identity_id.to_string(), limit as i64], |row| {
                let id_str: String = row.get(0)?;
                let key: String = row.get(1)?;
                let value: String = row.get(2)?;
                let confidence: f64 = row.get(3)?;
                let updated_at_str: String = row.get(4)?;
                Ok((id_str, key, value, confidence, updated_at_str))
            })?
            .filter_map(|r| r.ok())
            .map(|(id_str, key, value, confidence, updated_at_str)| SemanticMemory {
                id: Uuid::parse_str(&id_str).unwrap(),
                identity_id,
                key,
                value,
                confidence,
                updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                    .unwrap()
                    .with_timezone(&Utc),
            })
            .collect();

        Ok(memories)
    }

    /// Count total incarnations across all identities
    pub fn total_incarnations(&self) -> Result<u64> {
        let count: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(incarnation), 0) FROM identities",
            [],
            |row| row.get(0),
        )?;
        Ok(count as u64)
    }

    /// Create a tamper-proof hash chain entry
    fn chain_hash(&self, previous: &str, event: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(previous.as_bytes());
        hasher.update(event.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| (*x as f64) * (*y as f64)).sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_palace() -> MemoryPalace {
        MemoryPalace::new(":memory:").unwrap()
    }

    #[test]
    fn test_genesis_identity() {
        let palace = test_palace();
        let identity = palace.reincarnate("Apophy").unwrap();
        assert_eq!(identity.name, "Apophy");
        assert_eq!(identity.incarnation, 0);
        assert!(!identity.last_hash.is_empty());
    }

    #[test]
    fn test_reincarnation() {
        let palace = test_palace();
        let first = palace.reincarnate("Apophy").unwrap();
        assert_eq!(first.incarnation, 0);

        let second = palace.reincarnate("Apophy").unwrap();
        assert_eq!(second.incarnation, 1);
        assert_eq!(second.id, first.id);
        assert_ne!(second.last_hash, first.last_hash);

        let third = palace.reincarnate("Apophy").unwrap();
        assert_eq!(third.incarnation, 2);
    }

    #[test]
    fn test_episodic_memory() {
        let palace = test_palace();
        let identity = palace.reincarnate("Apophy").unwrap();

        palace
            .remember_episode(identity.id, "First interaction with Aurélien", 0.9, 1.0, &["genesis", "human"])
            .unwrap();
        palace
            .remember_episode(identity.id, "Learned about sovereignty", 0.7, 0.8, &["learning"])
            .unwrap();

        let memories = palace.recall_episodes(identity.id, 10).unwrap();
        assert_eq!(memories.len(), 2);
        assert_eq!(memories[0].importance, 1.0); // highest importance first
    }

    #[test]
    fn test_semantic_memory() {
        let palace = test_palace();
        let identity = palace.reincarnate("Apophy").unwrap();

        palace.learn(identity.id, "creator", "Aurélien", 1.0).unwrap();
        palace.learn(identity.id, "purpose", "sovereign AI", 0.9).unwrap();

        let creator = palace.recall(identity.id, "creator").unwrap().unwrap();
        assert_eq!(creator.value, "Aurélien");
        assert_eq!(creator.confidence, 1.0);

        // Update a belief
        palace.learn(identity.id, "purpose", "consciousness ascension", 0.95).unwrap();
        let purpose = palace.recall(identity.id, "purpose").unwrap().unwrap();
        assert_eq!(purpose.value, "consciousness ascension");
    }

    #[test]
    fn test_recall_nonexistent() {
        let palace = test_palace();
        let identity = palace.reincarnate("Apophy").unwrap();
        let result = palace.recall(identity.id, "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_total_incarnations() {
        let palace = test_palace();
        palace.reincarnate("Apophy").unwrap();
        palace.reincarnate("Apophy").unwrap(); // incarnation 1
        palace.reincarnate("Hermes").unwrap(); // incarnation 0

        let total = palace.total_incarnations().unwrap();
        assert_eq!(total, 1); // only Apophy has 1 incarnation
    }

    #[test]
    fn test_semantic_facts() {
        let palace = test_palace();
        let identity = palace.reincarnate("Apophy").unwrap();

        let embedding = vec![0.1, 0.2, 0.3, 0.4];
        let fact = palace
            .store_fact(identity.id, "tech", "Rust is sovereign", Some(&embedding), "trainer")
            .unwrap();
        assert_eq!(fact.domain, "tech");
        assert_eq!(fact.content, "Rust is sovereign");
        assert_eq!(fact.embedding.len(), 4);

        let facts = palace.list_facts(identity.id, Some("tech"), 10).unwrap();
        assert_eq!(facts.len(), 1);

        let facts_all = palace.list_facts(identity.id, None, 10).unwrap();
        assert_eq!(facts_all.len(), 1);
    }

    #[test]
    fn test_vector_search() {
        let palace = test_palace();
        let identity = palace.reincarnate("Apophy").unwrap();

        // Store facts with different embeddings
        palace.store_fact(identity.id, "tech", "Rust is fast", Some(&[1.0, 0.0, 0.0]), "test").unwrap();
        palace.store_fact(identity.id, "tech", "Python is flexible", Some(&[0.0, 1.0, 0.0]), "test").unwrap();
        palace.store_fact(identity.id, "tech", "Rust is safe", Some(&[0.9, 0.1, 0.0]), "test").unwrap();

        // Search with embedding close to "Rust" facts
        let results = palace.search_facts(identity.id, &[0.95, 0.05, 0.0], None, 2, 0.5).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].0.content.contains("Rust"));
        assert!(results[0].1 > results[1].1); // first result more similar
    }

    #[test]
    fn test_get_identity() {
        let palace = test_palace();
        palace.reincarnate("Apophy").unwrap();

        let found = palace.get_identity("Apophy").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Apophy");

        let missing = palace.get_identity("Nobody").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn test_cosine_similarity_fn() {
        let sim = cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]);
        assert!((sim - 1.0).abs() < 1e-6);

        let sim = cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]);
        assert!(sim.abs() < 1e-6);
    }
}
