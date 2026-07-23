//! Schema migrations for the operational database.
//!
//! Migrations are versioned and idempotent. Each migration checks its version
//! before applying. The schema_migrations table tracks applied migrations.

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

/// Migration 0001: Create the six operational domain tables.
fn migration_0001(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        -- =====================================================================
        -- Domain tables: local_models, runtime_profiles, hardware_profiles
        -- State tables: job_leases, runtime_runs, lifecycle_evidence
        -- =====================================================================

        CREATE TABLE IF NOT EXISTS local_models (
            model_id TEXT PRIMARY KEY,
            display_name TEXT NOT NULL,
            family TEXT,
            source_repository TEXT,
            filename TEXT NOT NULL,
            quantization TEXT,
            file_size_bytes INTEGER,
            sha256 TEXT,
            capability_classes_json TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS runtime_profiles (
            profile_id TEXT PRIMARY KEY,
            model_id TEXT NOT NULL REFERENCES local_models(model_id),
            device_backend TEXT NOT NULL,
            gpu_layers INTEGER,
            context_tokens INTEGER,
            estimated_vram_mb INTEGER,
            measured_vram_mb INTEGER,
            measured_tokens_per_sec REAL,
            practical_context_tokens INTEGER,
            profile_priority INTEGER NOT NULL DEFAULT 0,
            enabled INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE IF NOT EXISTS hardware_profiles (
            hw_profile_id TEXT PRIMARY KEY,
            device_name TEXT,
            vulkan_device TEXT,
            total_vram_mb INTEGER,
            available_vram_mb INTEGER,
            driver_version TEXT,
            measured_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS job_leases (
            lease_id TEXT PRIMARY KEY,
            model_id TEXT NOT NULL REFERENCES local_models(model_id),
            profile_id TEXT REFERENCES runtime_profiles(profile_id),
            port INTEGER,
            process_id INTEGER,
            state TEXT NOT NULL DEFAULT 'unloaded',
            loaded_at TEXT,
            released_at TEXT,
            vram_allocated_mb INTEGER,
            vram_released_at TEXT
        );

        CREATE TABLE IF NOT EXISTS runtime_runs (
            run_id TEXT PRIMARY KEY,
            lease_id TEXT NOT NULL REFERENCES job_leases(lease_id),
            packet_id TEXT,
            input_tokens INTEGER,
            output_tokens INTEGER,
            load_duration_ms INTEGER,
            generation_duration_ms INTEGER,
            exit_status TEXT,
            started_at TEXT NOT NULL,
            ended_at TEXT
        );

        CREATE TABLE IF NOT EXISTS lifecycle_evidence (
            evidence_id TEXT PRIMARY KEY,
            event_type TEXT NOT NULL,
            model_id TEXT,
            profile_id TEXT,
            lease_id TEXT,
            run_id TEXT,
            process_id INTEGER,
            observed_state TEXT,
            observation_json TEXT NOT NULL,
            occurred_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL
        );

        -- Indexes for common query patterns
        CREATE INDEX IF NOT EXISTS idx_runtime_profiles_model
            ON runtime_profiles(model_id);
        CREATE INDEX IF NOT EXISTS idx_job_leases_model
            ON job_leases(model_id);
        CREATE INDEX IF NOT EXISTS idx_job_leases_state
            ON job_leases(state);
        CREATE INDEX IF NOT EXISTS idx_runtime_runs_lease
            ON runtime_runs(lease_id);
        CREATE INDEX IF NOT EXISTS idx_lifecycle_evidence_lease
            ON lifecycle_evidence(lease_id);
        CREATE INDEX IF NOT EXISTS idx_lifecycle_evidence_event
            ON lifecycle_evidence(event_type);
        CREATE INDEX IF NOT EXISTS idx_lifecycle_evidence_occurred
            ON lifecycle_evidence(occurred_at);
        ",
    )
    .context("Failed to apply migration 0001")?;

    // Record migration
    conn.execute(
        "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![1, "0001_create_domain_tables", chrono::Utc::now().to_rfc3339()],
    )
    .context("Failed to record migration 0001")?;

    Ok(())
}

/// Verify that all expected tables exist.
pub fn verify_tables(conn: &Connection) -> Result<()> {
    let expected_tables = vec![
        "schema_migrations",
        "local_models",
        "runtime_profiles",
        "hardware_profiles",
        "job_leases",
        "runtime_runs",
        "lifecycle_evidence",
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
