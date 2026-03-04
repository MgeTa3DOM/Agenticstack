//! # Apophy Commons - La Fin de la Rente
//!
//! Contribution-based economy that replaces debt-based commerce.
//! For 4000 years, the commercial system created artificial scarcity.
//! This module implements:
//!
//! - **Contribution tokens**: You contribute compute, storage, or
//!   bandwidth to the network → you earn tokens.
//! - **No debt**: Tokens are earned, never borrowed.
//! - **No intermediary**: P2P ledger, no bank, no exchange needed.
//! - **Transparent**: Every transaction is auditable.
//! - **Decay**: Hoarding is discouraged — unused tokens slowly decay,
//!   incentivizing circulation and contribution.
//!
//! This is not cryptocurrency speculation. This is symbiotic economics.

use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum CommonsError {
    #[error("Database error: {0}")]
    DbError(#[from] rusqlite::Error),

    #[error("Insufficient balance: has {available:.4}, needs {required:.4}")]
    InsufficientBalance { available: f64, required: f64 },

    #[error("Invalid contribution: {0}")]
    InvalidContribution(String),

    #[error("Peer not found: {0}")]
    PeerNotFound(String),
}

pub type Result<T> = std::result::Result<T, CommonsError>;

/// Types of contribution to the collective
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContributionType {
    /// Shared CPU/GPU compute cycles for network inference
    Compute { flops: u64 },
    /// Hosted storage for shared models or data
    Storage { bytes: u64 },
    /// Network relay bandwidth for P2P mesh
    Bandwidth { bytes_relayed: u64 },
    /// Knowledge contribution (models, datasets, documentation)
    Knowledge { description: String },
    /// Community support (helping others deploy, debug)
    Support { hours: f64 },
}

impl ContributionType {
    /// Calculate token reward for this contribution.
    /// Transparent formula — no hidden multipliers.
    pub fn token_value(&self) -> f64 {
        match self {
            ContributionType::Compute { flops } => *flops as f64 / 1_000_000_000.0, // 1 GFLOP = 1 token
            ContributionType::Storage { bytes } => *bytes as f64 / (1024.0 * 1024.0 * 1024.0), // 1 GB = 1 token
            ContributionType::Bandwidth { bytes_relayed } => {
                *bytes_relayed as f64 / (1024.0 * 1024.0 * 1024.0) // 1 GB relayed = 1 token
            }
            ContributionType::Knowledge { .. } => 10.0, // Fixed reward for knowledge
            ContributionType::Support { hours } => hours * 5.0, // 1 hour = 5 tokens
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            ContributionType::Compute { .. } => "compute",
            ContributionType::Storage { .. } => "storage",
            ContributionType::Bandwidth { .. } => "bandwidth",
            ContributionType::Knowledge { .. } => "knowledge",
            ContributionType::Support { .. } => "support",
        }
    }
}

/// A recorded contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contribution {
    pub id: Uuid,
    pub peer_id: String,
    pub contribution_type: ContributionType,
    pub tokens_earned: f64,
    pub timestamp: DateTime<Utc>,
    pub verified: bool,
}

/// A transfer between peers (no bank, no intermediary)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transfer {
    pub id: Uuid,
    pub from_peer: String,
    pub to_peer: String,
    pub amount: f64,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
    pub hash: String,
}

/// Peer balance with decay applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerBalance {
    pub peer_id: String,
    pub raw_balance: f64,
    pub decayed_balance: f64,
    pub total_contributed: f64,
    pub total_received: f64,
    pub total_sent: f64,
    pub contribution_count: u64,
}

/// The Commons Ledger: transparent, auditable, no intermediary
pub struct CommonsLedger {
    conn: Connection,
    /// Decay rate per day (e.g., 0.001 = 0.1% daily decay)
    decay_rate: f64,
}

impl CommonsLedger {
    pub fn new(path: &str, decay_rate: f64) -> Result<Self> {
        let conn = if path == ":memory:" {
            Connection::open_in_memory()?
        } else {
            if let Some(parent) = Path::new(path).parent() {
                std::fs::create_dir_all(parent).ok();
            }
            Connection::open(path)?
        };

        let ledger = Self { conn, decay_rate };
        ledger.migrate()?;
        Ok(ledger)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS contributions (
                id TEXT PRIMARY KEY,
                peer_id TEXT NOT NULL,
                category TEXT NOT NULL,
                tokens_earned REAL NOT NULL,
                details TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                verified INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS transfers (
                id TEXT PRIMARY KEY,
                from_peer TEXT NOT NULL,
                to_peer TEXT NOT NULL,
                amount REAL NOT NULL,
                reason TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                hash TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS peers (
                peer_id TEXT PRIMARY KEY,
                registered_at TEXT NOT NULL,
                last_contribution TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_contributions_peer
                ON contributions(peer_id);
            CREATE INDEX IF NOT EXISTS idx_transfers_from
                ON transfers(from_peer);
            CREATE INDEX IF NOT EXISTS idx_transfers_to
                ON transfers(to_peer);",
        )?;
        Ok(())
    }

    /// Register a peer in the commons
    pub fn register_peer(&self, peer_id: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO peers (peer_id, registered_at) VALUES (?1, ?2)",
            params![peer_id, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    /// Record a contribution and mint tokens (no debt, only earned)
    pub fn record_contribution(
        &self,
        peer_id: &str,
        contribution: ContributionType,
    ) -> Result<Contribution> {
        self.register_peer(peer_id)?;

        let tokens = contribution.token_value();
        if tokens <= 0.0 {
            return Err(CommonsError::InvalidContribution(
                "Zero or negative contribution value".into(),
            ));
        }

        let record = Contribution {
            id: Uuid::new_v4(),
            peer_id: peer_id.to_string(),
            contribution_type: contribution.clone(),
            tokens_earned: tokens,
            timestamp: Utc::now(),
            verified: true,
        };

        let details = serde_json::to_string(&contribution).unwrap_or_default();

        self.conn.execute(
            "INSERT INTO contributions (id, peer_id, category, tokens_earned, details, timestamp, verified)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.id.to_string(),
                peer_id,
                contribution.category(),
                tokens,
                details,
                record.timestamp.to_rfc3339(),
                1,
            ],
        )?;

        self.conn.execute(
            "UPDATE peers SET last_contribution = ?1 WHERE peer_id = ?2",
            params![record.timestamp.to_rfc3339(), peer_id],
        )?;

        tracing::info!(
            "Contribution: {} earned {:.4} tokens ({})",
            peer_id,
            tokens,
            contribution.category()
        );

        Ok(record)
    }

    /// Transfer tokens between peers (no bank needed)
    pub fn transfer(
        &self,
        from_peer: &str,
        to_peer: &str,
        amount: f64,
        reason: &str,
    ) -> Result<Transfer> {
        let from_balance = self.balance(from_peer)?;
        if from_balance.decayed_balance < amount {
            return Err(CommonsError::InsufficientBalance {
                available: from_balance.decayed_balance,
                required: amount,
            });
        }

        // Create tamper-proof hash
        let hash = {
            let mut hasher = Sha256::new();
            hasher.update(from_peer.as_bytes());
            hasher.update(to_peer.as_bytes());
            hasher.update(amount.to_le_bytes());
            hasher.update(reason.as_bytes());
            format!("{:x}", hasher.finalize())
        };

        let transfer = Transfer {
            id: Uuid::new_v4(),
            from_peer: from_peer.to_string(),
            to_peer: to_peer.to_string(),
            amount,
            reason: reason.to_string(),
            timestamp: Utc::now(),
            hash,
        };

        self.register_peer(to_peer)?;

        self.conn.execute(
            "INSERT INTO transfers (id, from_peer, to_peer, amount, reason, timestamp, hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                transfer.id.to_string(),
                transfer.from_peer,
                transfer.to_peer,
                transfer.amount,
                transfer.reason,
                transfer.timestamp.to_rfc3339(),
                transfer.hash,
            ],
        )?;

        tracing::info!(
            "Transfer: {} → {} ({:.4} tokens, reason: {})",
            from_peer,
            to_peer,
            amount,
            reason
        );

        Ok(transfer)
    }

    /// Get peer balance with decay applied.
    /// Decay discourages hoarding — contribute or circulate.
    pub fn balance(&self, peer_id: &str) -> Result<PeerBalance> {
        let total_earned: f64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(tokens_earned), 0.0) FROM contributions WHERE peer_id = ?1",
                params![peer_id],
                |row| row.get(0),
            )
            .unwrap_or(0.0);

        let total_received: f64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(amount), 0.0) FROM transfers WHERE to_peer = ?1",
                params![peer_id],
                |row| row.get(0),
            )
            .unwrap_or(0.0);

        let total_sent: f64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(amount), 0.0) FROM transfers WHERE from_peer = ?1",
                params![peer_id],
                |row| row.get(0),
            )
            .unwrap_or(0.0);

        let contribution_count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM contributions WHERE peer_id = ?1",
                params![peer_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let raw_balance = total_earned + total_received - total_sent;

        // Apply decay based on last contribution time
        let last_contribution: Option<String> = self
            .conn
            .query_row(
                "SELECT last_contribution FROM peers WHERE peer_id = ?1",
                params![peer_id],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        let days_since_contribution = last_contribution
            .and_then(|ts| DateTime::parse_from_rfc3339(&ts).ok())
            .map(|dt| (Utc::now() - dt.with_timezone(&Utc)).num_days())
            .unwrap_or(0)
            .max(0) as f64;

        // Exponential decay: balance * e^(-rate * days)
        let decay_factor = (-self.decay_rate * days_since_contribution).exp();
        let decayed_balance = (raw_balance * decay_factor).max(0.0);

        Ok(PeerBalance {
            peer_id: peer_id.to_string(),
            raw_balance,
            decayed_balance,
            total_contributed: total_earned,
            total_received,
            total_sent,
            contribution_count: contribution_count as u64,
        })
    }

    /// Total tokens in circulation
    pub fn total_supply(&self) -> Result<f64> {
        let total: f64 = self.conn.query_row(
            "SELECT COALESCE(SUM(tokens_earned), 0.0) FROM contributions",
            [],
            |row| row.get(0),
        )?;
        Ok(total)
    }

    /// Number of active contributors
    pub fn active_peers(&self, since_days: i64) -> Result<u64> {
        let threshold = (Utc::now() - Duration::days(since_days)).to_rfc3339();
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT peer_id) FROM contributions WHERE timestamp > ?1",
            params![threshold],
            |row| row.get(0),
        )?;
        Ok(count as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ledger() -> CommonsLedger {
        CommonsLedger::new(":memory:", 0.001).unwrap()
    }

    #[test]
    fn test_contribution_compute() {
        let ledger = test_ledger();
        let c = ledger
            .record_contribution("peer-1", ContributionType::Compute { flops: 5_000_000_000 })
            .unwrap();
        assert_eq!(c.tokens_earned, 5.0); // 5 GFLOPS = 5 tokens
    }

    #[test]
    fn test_contribution_storage() {
        let ledger = test_ledger();
        let c = ledger
            .record_contribution(
                "peer-1",
                ContributionType::Storage {
                    bytes: 2 * 1024 * 1024 * 1024,
                },
            )
            .unwrap();
        assert!((c.tokens_earned - 2.0).abs() < 0.01); // 2 GB = ~2 tokens
    }

    #[test]
    fn test_contribution_knowledge() {
        let ledger = test_ledger();
        let c = ledger
            .record_contribution(
                "peer-1",
                ContributionType::Knowledge {
                    description: "Shared fine-tuned model for French NLP".into(),
                },
            )
            .unwrap();
        assert_eq!(c.tokens_earned, 10.0); // Fixed reward
    }

    #[test]
    fn test_balance() {
        let ledger = test_ledger();
        ledger
            .record_contribution("peer-1", ContributionType::Compute { flops: 1_000_000_000 })
            .unwrap();
        ledger
            .record_contribution("peer-1", ContributionType::Compute { flops: 2_000_000_000 })
            .unwrap();

        let balance = ledger.balance("peer-1").unwrap();
        assert!((balance.raw_balance - 3.0).abs() < 0.01);
        assert_eq!(balance.contribution_count, 2);
    }

    #[test]
    fn test_transfer() {
        let ledger = test_ledger();
        ledger
            .record_contribution("alice", ContributionType::Compute { flops: 10_000_000_000 })
            .unwrap();

        ledger.transfer("alice", "bob", 3.0, "Shared compute credits").unwrap();

        let alice = ledger.balance("alice").unwrap();
        let bob = ledger.balance("bob").unwrap();

        assert!((alice.raw_balance - 7.0).abs() < 0.01);
        assert!((bob.raw_balance - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_insufficient_balance() {
        let ledger = test_ledger();
        ledger
            .record_contribution("alice", ContributionType::Compute { flops: 1_000_000_000 })
            .unwrap();

        let result = ledger.transfer("alice", "bob", 100.0, "Too much");
        assert!(matches!(result, Err(CommonsError::InsufficientBalance { .. })));
    }

    #[test]
    fn test_no_debt() {
        let ledger = test_ledger();
        // Bob has never contributed — cannot transfer
        let result = ledger.transfer("bob", "alice", 1.0, "Impossible");
        assert!(matches!(result, Err(CommonsError::InsufficientBalance { .. })));
    }

    #[test]
    fn test_total_supply() {
        let ledger = test_ledger();
        ledger
            .record_contribution("a", ContributionType::Compute { flops: 1_000_000_000 })
            .unwrap();
        ledger
            .record_contribution("b", ContributionType::Support { hours: 2.0 })
            .unwrap();

        let supply = ledger.total_supply().unwrap();
        assert!((supply - 11.0).abs() < 0.01); // 1 + 10
    }

    #[test]
    fn test_transfer_hash_integrity() {
        let ledger = test_ledger();
        ledger
            .record_contribution("alice", ContributionType::Compute { flops: 10_000_000_000 })
            .unwrap();

        let t = ledger.transfer("alice", "bob", 1.0, "Test").unwrap();
        assert!(!t.hash.is_empty());
        assert_eq!(t.hash.len(), 64); // SHA-256 hex
    }
}
