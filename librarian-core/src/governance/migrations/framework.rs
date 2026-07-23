//! # Migration Framework
//!
//! Core migration types and runner. Each migration has an ID, description,
//! up SQL, and optional down SQL. The runner tracks applied migrations
//! and applies pending ones in order.

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::Serialize;
use uuid::Uuid;

/// A single numbered migration.
pub struct Migration {
    /// Migration number (order is defined by this).
    pub id: u32,
    /// Human-readable description.
    pub description: &'static str,
    /// SQL to apply the migration.
    pub up_sql: &'static str,
    /// SQL to roll back the migration (optional).
    pub down_sql: Option<&'static str>,
}

/// A record of an applied migration.
#[derive(Debug, Clone, Serialize)]
pub struct MigrationRecord {
    /// Migration number.
    pub migration_id: u32,
    /// Migration description.
    pub description: String,
    /// SHA-256 of the applied SQL.
    pub sql_hash: String,
    /// When it was applied.
    pub applied_at: String,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Evidence record ID (if evidence was generated).
    pub evidence_id: Option<String>,
    /// Receipt ID (if receipt was generated).
    pub receipt_id: Option<String>,
}

/// Errors that can occur during migration.
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("Migration {0} has already been applied")]
    AlreadyApplied(u32),
    #[error("Migration {0} not found in migration list")]
    NotFound(u32),
    #[error("Missing migration {0}: gap in sequence")]
    GapDetected(u32),
    #[error("Database error: {0}")]
    Database(String),
}

impl From<anyhow::Error> for MigrationError {
    fn from(e: anyhow::Error) -> Self {
        MigrationError::Database(e.to_string())
    }
}

/// The migration runner. Applies pending migrations to a database connection.
pub struct MigrationRunner {
    migrations: Vec<Migration>,
}

impl MigrationRunner {
    /// Create a new migration runner with the given migration list.
    pub fn new(migrations: Vec<Migration>) -> Self {
        Self { migrations }
    }

    /// Initialize the migration meta-tables.
    /// This is not a numbered migration — it's the framework bootstrap.
    fn ensure_meta_tables(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at TEXT NOT NULL,
                sql_hash TEXT NOT NULL,
                duration_ms INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS migration_log (
                log_id INTEGER PRIMARY KEY AUTOINCREMENT,
                migration_id INTEGER NOT NULL,
                action TEXT NOT NULL CHECK (action IN ('up', 'down')),
                description TEXT NOT NULL,
                sql_hash TEXT NOT NULL,
                applied_at TEXT NOT NULL,
                duration_ms INTEGER DEFAULT 0,
                evidence_id TEXT,
                receipt_id TEXT
            );"
        )?;
        Ok(())
    }

    /// Get the current schema version from the database.
    pub fn current_version(conn: &Connection) -> Result<u32> {
        let result: Result<u32, _> = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        );
        Ok(result.unwrap_or(0))
    }

    /// Check if a specific migration has been applied.
    pub fn is_applied(conn: &Connection, migration_id: u32) -> Result<bool> {
        let count: Result<u32, _> = conn.query_row(
            "SELECT COUNT(*) FROM schema_version WHERE version = ?1",
            params![migration_id],
            |row| row.get(0),
        );
        Ok(count.unwrap_or(0) > 0)
    }

    /// Get the full migration log.
    pub fn get_migration_log(conn: &Connection) -> Result<Vec<MigrationRecord>> {
        let mut stmt = conn.prepare(
            "SELECT migration_id, description, sql_hash, applied_at, duration_ms,
                    evidence_id, receipt_id
             FROM migration_log WHERE action = 'up' ORDER BY migration_id"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(MigrationRecord {
                migration_id: row.get(0)?,
                description: row.get(1)?,
                sql_hash: row.get(2)?,
                applied_at: row.get(3)?,
                duration_ms: row.get(4)?,
                evidence_id: row.get(5)?,
                receipt_id: row.get(6)?,
            })
        })?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    /// Run all pending migrations. Returns the list of applied migration records.
    pub fn run_pending(&self, conn: &Connection, evidence_id: Option<&str>, receipt_id: Option<&str>) -> Result<Vec<MigrationRecord>> {
        Self::ensure_meta_tables(conn)?;
        let current = Self::current_version(conn)?;
        let mut applied = Vec::new();

        for migration in &self.migrations {
            if migration.id <= current {
                continue; // Already applied
            }

            let start = std::time::Instant::now();

            // Apply the migration
            conn.execute_batch(migration.up_sql)
                .with_context(|| format!("Failed to apply migration {}", migration.id))?;

            let duration_ms = start.elapsed().as_millis() as u64;

            // Compute SQL hash
            let sql_hash = {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(migration.up_sql.as_bytes());
                format!("{:x}", hasher.finalize())
            };

            let applied_at = Utc::now().to_rfc3339();

            // Record in schema_version
            conn.execute(
                "INSERT INTO schema_version (version, description, applied_at, sql_hash, duration_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![migration.id, migration.description, applied_at, sql_hash, duration_ms],
            )?;

            // Record in migration_log
            conn.execute(
                "INSERT INTO migration_log (migration_id, action, description, sql_hash, applied_at,
                 duration_ms, evidence_id, receipt_id)
                 VALUES (?1, 'up', ?2, ?3, ?4, ?5, ?6, ?7)",
                params![migration.id, migration.description, sql_hash, applied_at,
                        duration_ms, evidence_id, receipt_id],
            )?;

            applied.push(MigrationRecord {
                migration_id: migration.id,
                description: migration.description.to_string(),
                sql_hash,
                applied_at,
                duration_ms,
                evidence_id: evidence_id.map(|s| s.to_string()),
                receipt_id: receipt_id.map(|s| s.to_string()),
            });
        }

        Ok(applied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn create_test_migrations() -> Vec<Migration> {
        vec![
            Migration {
                id: 1,
                description: "Create initial tables",
                up_sql: "CREATE TABLE IF NOT EXISTS test_table_1 (id INTEGER PRIMARY KEY, name TEXT)",
                down_sql: Some("DROP TABLE IF EXISTS test_table_1"),
            },
            Migration {
                id: 2,
                description: "Add metadata column",
                up_sql: "ALTER TABLE test_table_1 ADD COLUMN metadata TEXT DEFAULT ''",
                down_sql: None,
            },
        ]
    }

    #[test]
    fn test_meta_tables_created() {
        let conn = Connection::open_in_memory().unwrap();
        MigrationRunner::ensure_meta_tables(&conn).unwrap();

        // Verify schema_version exists
        let version = MigrationRunner::current_version(&conn).unwrap();
        assert_eq!(version, 0);
    }

    #[test]
    fn test_run_single_migration() {
        let conn = Connection::open_in_memory().unwrap();
        let migrations = create_test_migrations();
        let runner = MigrationRunner::new(migrations);

        let applied = runner.run_pending(&conn, Some("evt-mig-001"), Some("receipt-mig-001")).unwrap();
        assert_eq!(applied.len(), 2);
        assert_eq!(applied[0].migration_id, 1);
        assert_eq!(applied[1].migration_id, 2);

        let version = MigrationRunner::current_version(&conn).unwrap();
        assert_eq!(version, 2);
    }

    #[test]
    fn test_migrations_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        let migrations = create_test_migrations();
        let runner = MigrationRunner::new(migrations);

        // First run
        let applied = runner.run_pending(&conn, None, None).unwrap();
        assert_eq!(applied.len(), 2);

        // Second run — no new migrations
        let applied = runner.run_pending(&conn, None, None).unwrap();
        assert_eq!(applied.len(), 0);
    }

    #[test]
    fn test_migration_log() {
        let conn = Connection::open_in_memory().unwrap();
        let migrations = create_test_migrations();
        let runner = MigrationRunner::new(migrations);
        runner.run_pending(&conn, None, None).unwrap();

        let log = MigrationRunner::get_migration_log(&conn).unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].migration_id, 1);
        assert_eq!(log[1].migration_id, 2);
    }

    #[test]
    fn test_current_version_tracking() {
        let conn = Connection::open_in_memory().unwrap();
        assert_eq!(MigrationRunner::current_version(&conn).unwrap(), 0);

        let migrations = create_test_migrations();
        let runner = MigrationRunner::new(migrations);
        runner.run_pending(&conn, None, None).unwrap();
        assert_eq!(MigrationRunner::current_version(&conn).unwrap(), 2);
    }
}
