use rusqlite::{Connection, params};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, DbError>;

/// Embedded sovereign database (SQLite)
pub struct SovereignDb {
    conn: Connection,
}

impl SovereignDb {
    pub fn new(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;

        // Security hardening
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;"
        )?;

        Ok(Self { conn })
    }

    /// Run database migrations
    pub fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS peers (
                id TEXT PRIMARY KEY,
                public_key BLOB NOT NULL,
                x25519_key BLOB NOT NULL,
                display_name TEXT,
                created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                last_seen INTEGER
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                sender_id TEXT NOT NULL,
                recipient_id TEXT NOT NULL,
                ciphertext BLOB NOT NULL,
                nonce BLOB NOT NULL,
                epoch INTEGER NOT NULL,
                message_type TEXT NOT NULL DEFAULT 'text',
                created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                expires_at INTEGER,
                FOREIGN KEY (sender_id) REFERENCES peers(id),
                FOREIGN KEY (recipient_id) REFERENCES peers(id)
            );

            CREATE TABLE IF NOT EXISTS groups (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_by TEXT NOT NULL,
                max_members INTEGER NOT NULL DEFAULT 10000,
                created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                FOREIGN KEY (created_by) REFERENCES peers(id)
            );

            CREATE TABLE IF NOT EXISTS group_members (
                group_id TEXT NOT NULL,
                peer_id TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'member',
                joined_at INTEGER NOT NULL DEFAULT (unixepoch()),
                PRIMARY KEY (group_id, peer_id),
                FOREIGN KEY (group_id) REFERENCES groups(id),
                FOREIGN KEY (peer_id) REFERENCES peers(id)
            );

            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                actor_id TEXT,
                details TEXT,
                created_at INTEGER NOT NULL DEFAULT (unixepoch())
            );

            CREATE TABLE IF NOT EXISTS agent_tasks (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                task_type TEXT NOT NULL,
                payload TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                result TEXT,
                created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                completed_at INTEGER
            );

            CREATE INDEX IF NOT EXISTS idx_messages_sender ON messages(sender_id);
            CREATE INDEX IF NOT EXISTS idx_messages_recipient ON messages(recipient_id);
            CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(created_at);
            CREATE INDEX IF NOT EXISTS idx_audit_created ON audit_log(created_at);
            CREATE INDEX IF NOT EXISTS idx_agent_tasks_status ON agent_tasks(status);"
        )?;

        tracing::info!("Database migrations complete");
        Ok(())
    }

    /// Log an audit event
    pub fn audit_log(&self, event_type: &str, actor_id: Option<&str>, details: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO audit_log (event_type, actor_id, details) VALUES (?1, ?2, ?3)",
            params![event_type, actor_id, details],
        )?;
        Ok(())
    }

    /// Register a peer
    pub fn register_peer(
        &self,
        id: &str,
        public_key: &[u8],
        x25519_key: &[u8],
        display_name: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO peers (id, public_key, x25519_key, display_name)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, public_key, x25519_key, display_name],
        )?;
        Ok(())
    }

    /// Get peer count
    pub fn peer_count(&self) -> Result<u64> {
        let count: u64 = self.conn.query_row(
            "SELECT COUNT(*) FROM peers",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_creation_and_migration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = SovereignDb::new(&path).unwrap();
        db.migrate().unwrap();
    }

    #[test]
    fn test_peer_registration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = SovereignDb::new(&path).unwrap();
        db.migrate().unwrap();

        db.register_peer("peer1", &[1, 2, 3], &[4, 5, 6], Some("Alice")).unwrap();
        assert_eq!(db.peer_count().unwrap(), 1);
    }

    #[test]
    fn test_audit_log() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = SovereignDb::new(&path).unwrap();
        db.migrate().unwrap();

        db.audit_log("startup", None, "Server started").unwrap();
    }
}
