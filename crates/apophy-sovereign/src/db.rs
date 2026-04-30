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
            CREATE INDEX IF NOT EXISTS idx_agent_tasks_status ON agent_tasks(status);

            -- Infrastructure integration tables
            CREATE TABLE IF NOT EXISTS infra_services (
                name TEXT PRIMARY KEY,
                service_type TEXT NOT NULL,
                base_url TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                last_health_check INTEGER,
                last_status TEXT DEFAULT 'unknown',
                created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                updated_at INTEGER NOT NULL DEFAULT (unixepoch())
            );

            CREATE TABLE IF NOT EXISTS infra_webhooks (
                id TEXT PRIMARY KEY,
                source TEXT NOT NULL,
                event_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                processed INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL DEFAULT (unixepoch())
            );

            CREATE INDEX IF NOT EXISTS idx_webhooks_source ON infra_webhooks(source);
            CREATE INDEX IF NOT EXISTS idx_webhooks_created ON infra_webhooks(created_at);

            -- Agent fleet tables (Divine Synarchy)
            CREATE TABLE IF NOT EXISTS agent_fleet (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                tier TEXT NOT NULL,
                domain TEXT NOT NULL,
                capabilities TEXT NOT NULL DEFAULT '[]',
                prompt_template TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'ready',
                parent_id TEXT,
                tasks_completed INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                updated_at INTEGER NOT NULL DEFAULT (unixepoch())
            );

            CREATE TABLE IF NOT EXISTS agent_connections (
                agent_id TEXT NOT NULL,
                connected_to TEXT NOT NULL,
                connection_type TEXT NOT NULL DEFAULT 'hierarchy',
                created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                PRIMARY KEY (agent_id, connected_to),
                FOREIGN KEY (agent_id) REFERENCES agent_fleet(id),
                FOREIGN KEY (connected_to) REFERENCES agent_fleet(id)
            );

            CREATE TABLE IF NOT EXISTS agent_prompts (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                domain TEXT NOT NULL,
                specialization TEXT,
                prompt_text TEXT NOT NULL,
                category TEXT,
                created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                FOREIGN KEY (agent_id) REFERENCES agent_fleet(id)
            );

            CREATE INDEX IF NOT EXISTS idx_fleet_tier ON agent_fleet(tier);
            CREATE INDEX IF NOT EXISTS idx_fleet_domain ON agent_fleet(domain);
            CREATE INDEX IF NOT EXISTS idx_fleet_status ON agent_fleet(status);
            CREATE INDEX IF NOT EXISTS idx_prompts_agent ON agent_prompts(agent_id);
            CREATE INDEX IF NOT EXISTS idx_prompts_domain ON agent_prompts(domain);"
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn peer_count(&self) -> Result<u64> {
        let count: u64 = self.conn.query_row(
            "SELECT COUNT(*) FROM peers",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Log an incoming webhook
    pub fn log_webhook(&self, id: &str, source: &str, event_type: &str, payload: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO infra_webhooks (id, source, event_type, payload) VALUES (?1, ?2, ?3, ?4)",
            params![id, source, event_type, payload],
        )?;
        Ok(())
    }

    /// Mark a webhook as processed
    #[allow(dead_code)]
    pub fn mark_webhook_processed(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE infra_webhooks SET processed = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Update service health status
    pub fn update_service_status(&self, name: &str, service_type: &str, base_url: &str, status: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO infra_services (name, service_type, base_url, last_health_check, last_status, updated_at)
             VALUES (?1, ?2, ?3, unixepoch(), ?4, unixepoch())
             ON CONFLICT(name) DO UPDATE SET
                last_health_check = unixepoch(),
                last_status = ?4,
                updated_at = unixepoch()",
            params![name, service_type, base_url, status],
        )?;
        Ok(())
    }

    // === Agent Fleet Methods ===

    /// Register an agent in the fleet
    pub fn register_agent(
        &self,
        id: &str,
        name: &str,
        tier: &str,
        domain: &str,
        capabilities_json: &str,
        prompt_template: &str,
        parent_id: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO agent_fleet (id, name, tier, domain, capabilities, prompt_template, parent_id, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, unixepoch())",
            params![id, name, tier, domain, capabilities_json, prompt_template, parent_id],
        )?;
        Ok(())
    }

    /// Add a connection between two agents
    pub fn add_agent_connection(&self, agent_id: &str, connected_to: &str, connection_type: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO agent_connections (agent_id, connected_to, connection_type)
             VALUES (?1, ?2, ?3)",
            params![agent_id, connected_to, connection_type],
        )?;
        Ok(())
    }

    /// Get fleet counts by tier
    #[allow(dead_code)]
    pub fn fleet_counts(&self) -> Result<(u64, u64, u64)> {
        let (strategic, tactical, operational) = self.conn.query_row(
            "SELECT
                SUM(CASE WHEN tier = 'strategic' THEN 1 ELSE 0 END),
                SUM(CASE WHEN tier = 'tactical' THEN 1 ELSE 0 END),
                SUM(CASE WHEN tier = 'operational' THEN 1 ELSE 0 END)
             FROM agent_fleet",
            [],
            |row| Ok((
                row.get::<_, Option<u64>>(0)?.unwrap_or(0),
                row.get::<_, Option<u64>>(1)?.unwrap_or(0),
                row.get::<_, Option<u64>>(2)?.unwrap_or(0),
            )),
        )?;
        Ok((strategic, tactical, operational))
    }

    /// Get total agent count
    #[allow(dead_code)]
    pub fn agent_count(&self) -> Result<u64> {
        let count: u64 = self.conn.query_row(
            "SELECT COUNT(*) FROM agent_fleet", [], |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Update agent status
    #[allow(dead_code)]
    pub fn update_agent_status(&self, id: &str, status: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE agent_fleet SET status = ?2, updated_at = unixepoch() WHERE id = ?1",
            params![id, status],
        )?;
        Ok(())
    }

    /// Get total webhook count
    pub fn webhook_count(&self) -> Result<u64> {
        let count: u64 = self.conn.query_row(
            "SELECT COUNT(*) FROM infra_webhooks",
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
