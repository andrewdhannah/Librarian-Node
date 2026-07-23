//! Schema migrations for the canonical (Mac-side) database.
//!
//! Mirrors the Windows operational DB migration pattern:
//! - Versioned and idempotent
//! - schema_migrations table tracks applied migrations
//! - Each migration checks version before applying

use anyhow::{Context, Result};
use rusqlite::Connection;

/// Current schema version. Increment when adding new migrations.
const CURRENT_VERSION: i64 = 1;

/// Apply all pending migrations. Idempotent — safe to call multiple times.
pub fn migrate(conn: &Connection) -> Result<()> {
    // Ensure migration tracking table exists
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );",
    )
    .context("Failed to create schema_migrations table")?;

    let current: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .context("Failed to read migration version")?;

    if current >= CURRENT_VERSION {
        return Ok(());
    }

    if current < 1 {
        migration_0001(conn)?;
    }

    // Future migrations go here:
    // if current < 2 { migration_0002(conn)?; }

    Ok(())
}

/// Migration 0001: Create canonical identity and system tables.
fn migration_0001(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        -- =====================================================================
        -- Mac Canonical Tables
        -- Identity: model_identity_record, system_profile
        -- Fixtures: task_pack, validator_pack
        -- =====================================================================

        CREATE TABLE IF NOT EXISTS model_identity_record (
            identity_id TEXT PRIMARY KEY,
            model_id_ref TEXT NOT NULL,
            gguf_metadata_hash TEXT,
            chat_template_id TEXT,
            license_spdx TEXT,
            qualification_scope TEXT NOT NULL DEFAULT 'full',
            roles_json TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS system_profile (
            system_profile_id TEXT PRIMARY KEY,
            os TEXT,
            cpu TEXT,
            ram_mb INTEGER,
            gpu_description TEXT,
            notes TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS task_pack (
            task_pack_id TEXT PRIMARY KEY,
            version INTEGER NOT NULL,
            role TEXT NOT NULL,
            description TEXT,
            fixture_hash TEXT NOT NULL,
            fixture_path TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS validator_pack (
            validator_pack_id TEXT PRIMARY KEY,
            version INTEGER NOT NULL,
            role TEXT NOT NULL,
            description TEXT,
            rules_hash TEXT NOT NULL,
            rules_path TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            created_at TEXT NOT NULL
        );

        -- Indexes for common query patterns
        CREATE INDEX IF NOT EXISTS idx_model_identity_model_ref
            ON model_identity_record(model_id_ref);
        CREATE INDEX IF NOT EXISTS idx_task_pack_role
            ON task_pack(role);
        CREATE INDEX IF NOT EXISTS idx_task_pack_status
            ON task_pack(status);
        CREATE INDEX IF NOT EXISTS idx_validator_pack_role
            ON validator_pack(role);
        CREATE INDEX IF NOT EXISTS idx_validator_pack_status
            ON validator_pack(status);
        ",
    )
    .context("Failed to apply canonical migration 0001")?;

    // Record migration
    conn.execute(
        "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![1, "0001_create_canonical_tables", chrono::Utc::now().to_rfc3339()],
    )
    .context("Failed to record canonical migration 0001")?;

    Ok(())
}

/// Verify that all expected tables exist.
pub fn verify_tables(conn: &Connection) -> Result<()> {
    let expected_tables = vec![
        "schema_migrations",
        "model_identity_record",
        "system_profile",
        "task_pack",
        "validator_pack",
    ];

    for table in &expected_tables {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                rusqlite::params![table],
                |row| row.get(0),
            )
            .with_context(|| format!("Failed to query for table '{}'", table))?;

        if count == 0 {
            anyhow::bail!("Required table '{}' not found", table);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_creates_tables() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        verify_tables(&conn).unwrap();
    }

    #[test]
    fn test_migration_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap();

        // Version should still be 1
        let version: i64 = conn
            .query_row(
                "SELECT MAX(version) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, 1);
    }

    #[test]
    fn test_no_canonical_tables_leak_into_windows_schema() {
        // Verify these tables are NOT in the canonical schema
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();

        let forbidden = vec![
            "local_models",
            "runtime_profiles",
            "hardware_profiles",
            "job_leases",
            "runtime_runs",
            "lifecycle_evidence",
        ];

        for table in &forbidden {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    rusqlite::params![table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 0, "Windows table '{}' should not exist in canonical DB", table);
        }
    }
}
