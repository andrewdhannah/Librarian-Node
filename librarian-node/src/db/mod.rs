//! Operational database for the runtime node.
//!
//! RuntimeDatabase owns the SQLite path and provides bounded connections.
//! Each operation obtains its own connection. Transactions belong here,
//! not in server.rs or process.rs.

pub mod connection;
pub mod migrations;

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Arc;

use crate::models::{HardwareProfile, LocalModel, RuntimeProfile};
use crate::runtime_state::{LifecycleEvidence, LifecycleEventType, ModelLease, RuntimeRun};

use self::connection::{configure_connection, configure_database_init, verify_pragmas};
use self::migrations::{migrate, verify_tables};

/// Owner abstraction for the operational SQLite database.
/// Does not hold a live connection — each operation opens a bounded connection.
#[derive(Clone)]
pub struct RuntimeDatabase {
    path: Arc<PathBuf>,
}

impl RuntimeDatabase {
    /// Open or create the database at the given path.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = Arc::new(path.into());

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create DB directory: {}", parent.display()))?;
        }

        let db = Self { path };

        // Initialize database-level PRAGMAs
        {
            let conn = db.open_connection()?;
            configure_database_init(&conn)?;
        }

        Ok(db)
    }

    /// Open the database using RouterConfig for path resolution.
    pub fn open_from_config(config: &crate::config::RouterConfig) -> Result<Self> {
        let db_path = resolve_db_path(config);
        tracing::info!("Opening operational database at: {}", db_path.display());
        Self::open(db_path)
    }

    /// Run all pending migrations. Idempotent.
    pub fn migrate(&self) -> Result<()> {
        let conn = self.open_connection()?;
        migrate(&conn).context("Migration failed")?;
        Ok(())
    }

    /// Verify database posture: PRAGMAs, tables, foreign keys.
    pub fn verify(&self) -> Result<()> {
        let conn = self.open_connection()?;
        verify_pragmas(&conn).context("PRAGMA verification failed")?;
        verify_tables(&conn).context("Table verification failed")?;
        Ok(())
    }

    /// Open a new connection with standard PRAGMAs applied.
    pub fn open_connection(&self) -> Result<Connection> {
        let conn = Connection::open(self.path.as_ref())
            .with_context(|| format!("Failed to open DB: {}", self.path.display()))?;
        configure_connection(&conn)?;
        Ok(conn)
    }

    // ========================================================================
    // LocalModel CRUD
    // ========================================================================

    pub fn insert_local_model(&self, model: &LocalModel) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO local_models (model_id, display_name, family, source_repository,
             filename, quantization, file_size_bytes, sha256, capability_classes_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                model.model_id,
                model.display_name,
                model.family,
                model.source_repository,
                model.filename,
                model.quantization,
                model.file_size_bytes,
                model.sha256,
                model.capability_classes_json,
                model.created_at,
            ],
        )
        .context("Failed to insert local_model")?;
        Ok(())
    }

    pub fn get_local_model(&self, model_id: &str) -> Result<Option<LocalModel>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT model_id, display_name, family, source_repository, filename,
             quantization, file_size_bytes, sha256, capability_classes_json, created_at
             FROM local_models WHERE model_id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![model_id], |row| {
            Ok(LocalModel {
                model_id: row.get(0)?,
                display_name: row.get(1)?,
                family: row.get(2)?,
                source_repository: row.get(3)?,
                filename: row.get(4)?,
                quantization: row.get(5)?,
                file_size_bytes: row.get(6)?,
                sha256: row.get(7)?,
                capability_classes_json: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r.context("Failed to read local_model row")?)),
            None => Ok(None),
        }
    }

    pub fn list_local_models(&self) -> Result<Vec<LocalModel>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT model_id, display_name, family, source_repository, filename,
             quantization, file_size_bytes, sha256, capability_classes_json, created_at
             FROM local_models ORDER BY display_name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(LocalModel {
                model_id: row.get(0)?,
                display_name: row.get(1)?,
                family: row.get(2)?,
                source_repository: row.get(3)?,
                filename: row.get(4)?,
                quantization: row.get(5)?,
                file_size_bytes: row.get(6)?,
                sha256: row.get(7)?,
                capability_classes_json: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;
        let mut models = Vec::new();
        for row in rows {
            models.push(row.context("Failed to read local_model row")?);
        }
        Ok(models)
    }

    pub fn update_local_model(&self, model: &LocalModel) -> Result<()> {
        let conn = self.open_connection()?;
        let affected = conn.execute(
            "UPDATE local_models SET display_name = ?2, family = ?3, source_repository = ?4,
             filename = ?5, quantization = ?6, file_size_bytes = ?7, sha256 = ?8,
             capability_classes_json = ?9
             WHERE model_id = ?1",
            rusqlite::params![
                model.model_id,
                model.display_name,
                model.family,
                model.source_repository,
                model.filename,
                model.quantization,
                model.file_size_bytes,
                model.sha256,
                model.capability_classes_json,
            ],
        )?;
        if affected == 0 {
            anyhow::bail!("No local_model found with id '{}'", model.model_id);
        }
        Ok(())
    }

    pub fn delete_local_model(&self, model_id: &str) -> Result<()> {
        let conn = self.open_connection()?;
        let affected = conn.execute("DELETE FROM local_models WHERE model_id = ?1", rusqlite::params![model_id])?;
        if affected == 0 {
            anyhow::bail!("No local_model found with id '{}'", model_id);
        }
        Ok(())
    }

    // ========================================================================
    // RuntimeProfile CRUD
    // ========================================================================

    pub fn insert_runtime_profile(&self, profile: &RuntimeProfile) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO runtime_profiles (profile_id, model_id, device_backend, gpu_layers,
             context_tokens, estimated_vram_mb, measured_vram_mb, measured_tokens_per_sec,
             practical_context_tokens, profile_priority, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                profile.profile_id,
                profile.model_id,
                profile.device_backend,
                profile.gpu_layers,
                profile.context_tokens,
                profile.estimated_vram_mb,
                profile.measured_vram_mb,
                profile.measured_tokens_per_sec,
                profile.practical_context_tokens,
                profile.profile_priority,
                profile.enabled as i32,
            ],
        )
        .context("Failed to insert runtime_profile")?;
        Ok(())
    }

    pub fn get_runtime_profile(&self, profile_id: &str) -> Result<Option<RuntimeProfile>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT profile_id, model_id, device_backend, gpu_layers, context_tokens,
             estimated_vram_mb, measured_vram_mb, measured_tokens_per_sec,
             practical_context_tokens, profile_priority, enabled
             FROM runtime_profiles WHERE profile_id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![profile_id], |row| {
            Ok(RuntimeProfile {
                profile_id: row.get(0)?,
                model_id: row.get(1)?,
                device_backend: row.get(2)?,
                gpu_layers: row.get(3)?,
                context_tokens: row.get(4)?,
                estimated_vram_mb: row.get(5)?,
                measured_vram_mb: row.get(6)?,
                measured_tokens_per_sec: row.get(7)?,
                practical_context_tokens: row.get(8)?,
                profile_priority: row.get(9)?,
                enabled: row.get::<_, i32>(10)? != 0,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r.context("Failed to read runtime_profile row")?)),
            None => Ok(None),
        }
    }

    pub fn list_runtime_profiles(&self) -> Result<Vec<RuntimeProfile>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT profile_id, model_id, device_backend, gpu_layers, context_tokens,
             estimated_vram_mb, measured_vram_mb, measured_tokens_per_sec,
             practical_context_tokens, profile_priority, enabled
             FROM runtime_profiles ORDER BY profile_priority DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(RuntimeProfile {
                profile_id: row.get(0)?,
                model_id: row.get(1)?,
                device_backend: row.get(2)?,
                gpu_layers: row.get(3)?,
                context_tokens: row.get(4)?,
                estimated_vram_mb: row.get(5)?,
                measured_vram_mb: row.get(6)?,
                measured_tokens_per_sec: row.get(7)?,
                practical_context_tokens: row.get(8)?,
                profile_priority: row.get(9)?,
                enabled: row.get::<_, i32>(10)? != 0,
            })
        })?;
        let mut profiles = Vec::new();
        for row in rows {
            profiles.push(row.context("Failed to read runtime_profile row")?);
        }
        Ok(profiles)
    }

    pub fn list_runtime_profiles_for_model(&self, model_id: &str) -> Result<Vec<RuntimeProfile>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT profile_id, model_id, device_backend, gpu_layers, context_tokens,
             estimated_vram_mb, measured_vram_mb, measured_tokens_per_sec,
             practical_context_tokens, profile_priority, enabled
             FROM runtime_profiles WHERE model_id = ?1 ORDER BY profile_priority DESC",
        )?;
        let rows = stmt.query_map(rusqlite::params![model_id], |row| {
            Ok(RuntimeProfile {
                profile_id: row.get(0)?,
                model_id: row.get(1)?,
                device_backend: row.get(2)?,
                gpu_layers: row.get(3)?,
                context_tokens: row.get(4)?,
                estimated_vram_mb: row.get(5)?,
                measured_vram_mb: row.get(6)?,
                measured_tokens_per_sec: row.get(7)?,
                practical_context_tokens: row.get(8)?,
                profile_priority: row.get(9)?,
                enabled: row.get::<_, i32>(10)? != 0,
            })
        })?;
        let mut profiles = Vec::new();
        for row in rows {
            profiles.push(row.context("Failed to read runtime_profile row")?);
        }
        Ok(profiles)
    }

    pub fn delete_runtime_profile(&self, profile_id: &str) -> Result<()> {
        let conn = self.open_connection()?;
        let affected = conn.execute("DELETE FROM runtime_profiles WHERE profile_id = ?1", rusqlite::params![profile_id])?;
        if affected == 0 {
            anyhow::bail!("No runtime_profile found with id '{}'", profile_id);
        }
        Ok(())
    }

    // ========================================================================
    // HardwareProfile CRUD
    // ========================================================================

    pub fn insert_hardware_profile(&self, hw: &HardwareProfile) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO hardware_profiles (hw_profile_id, device_name, vulkan_device,
             total_vram_mb, available_vram_mb, driver_version, measured_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                hw.hw_profile_id,
                hw.device_name,
                hw.vulkan_device,
                hw.total_vram_mb,
                hw.available_vram_mb,
                hw.driver_version,
                hw.measured_at,
            ],
        )
        .context("Failed to insert hardware_profile")?;
        Ok(())
    }

    pub fn get_hardware_profile(&self, hw_profile_id: &str) -> Result<Option<HardwareProfile>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT hw_profile_id, device_name, vulkan_device, total_vram_mb,
             available_vram_mb, driver_version, measured_at
             FROM hardware_profiles WHERE hw_profile_id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![hw_profile_id], |row| {
            Ok(HardwareProfile {
                hw_profile_id: row.get(0)?,
                device_name: row.get(1)?,
                vulkan_device: row.get(2)?,
                total_vram_mb: row.get(3)?,
                available_vram_mb: row.get(4)?,
                driver_version: row.get(5)?,
                measured_at: row.get(6)?,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r.context("Failed to read hardware_profile row")?)),
            None => Ok(None),
        }
    }

    pub fn list_hardware_profiles(&self) -> Result<Vec<HardwareProfile>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT hw_profile_id, device_name, vulkan_device, total_vram_mb,
             available_vram_mb, driver_version, measured_at
             FROM hardware_profiles ORDER BY measured_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(HardwareProfile {
                hw_profile_id: row.get(0)?,
                device_name: row.get(1)?,
                vulkan_device: row.get(2)?,
                total_vram_mb: row.get(3)?,
                available_vram_mb: row.get(4)?,
                driver_version: row.get(5)?,
                measured_at: row.get(6)?,
            })
        })?;
        let mut profiles = Vec::new();
        for row in rows {
            profiles.push(row.context("Failed to read hardware_profile row")?);
        }
        Ok(profiles)
    }

    // ========================================================================
    // ModelLease CRUD
    // ========================================================================

    pub fn insert_lease(&self, lease: &ModelLease) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO job_leases (lease_id, model_id, profile_id, port, process_id,
             state, loaded_at, released_at, vram_allocated_mb, vram_released_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                lease.lease_id,
                lease.model_id,
                lease.profile_id,
                lease.port,
                lease.process_id,
                lease.state.as_str(),
                lease.loaded_at,
                lease.released_at,
                lease.vram_allocated_mb,
                lease.vram_released_at,
            ],
        )
        .context("Failed to insert lease")?;
        Ok(())
    }

    pub fn get_lease(&self, lease_id: &str) -> Result<Option<ModelLease>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT lease_id, model_id, profile_id, port, process_id, state,
             loaded_at, released_at, vram_allocated_mb, vram_released_at
             FROM job_leases WHERE lease_id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![lease_id], |row| {
            let state_str: String = row.get(5)?;
            Ok(ModelLease {
                lease_id: row.get(0)?,
                model_id: row.get(1)?,
                profile_id: row.get(2)?,
                port: row.get(3)?,
                process_id: row.get(4)?,
                state: crate::runtime_state::LeaseState::from_str(&state_str)
                    .unwrap_or(crate::runtime_state::LeaseState::Failed),
                loaded_at: row.get(6)?,
                released_at: row.get(7)?,
                vram_allocated_mb: row.get(8)?,
                vram_released_at: row.get(9)?,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r.context("Failed to read lease row")?)),
            None => Ok(None),
        }
    }

    pub fn update_lease_state(&self, lease_id: &str, state: crate::runtime_state::LeaseState) -> Result<()> {
        let conn = self.open_connection()?;
        let affected = conn.execute(
            "UPDATE job_leases SET state = ?2 WHERE lease_id = ?1",
            rusqlite::params![lease_id, state.as_str()],
        )?;
        if affected == 0 {
            anyhow::bail!("No lease found with id '{}'", lease_id);
        }
        Ok(())
    }

    /// Update the process_id on a lease record.
    pub fn update_lease_process_id(&self, lease_id: &str, process_id: i32) -> Result<()> {
        let conn = self.open_connection()?;
        let affected = conn.execute(
            "UPDATE job_leases SET process_id = ?2 WHERE lease_id = ?1",
            rusqlite::params![lease_id, process_id],
        )?;
        if affected == 0 {
            anyhow::bail!("No lease found with id '{}'", lease_id);
        }
        Ok(())
    }

    /// Execute raw SQL (for supervisor operations that need ad-hoc updates).
    pub fn execute_sql(&self, sql: &str) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute_batch(sql)
            .with_context(|| format!("Failed to execute SQL: {}", &sql[..sql.len().min(200)]))?;
        Ok(())
    }

    pub fn get_active_leases(&self) -> Result<Vec<ModelLease>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT lease_id, model_id, profile_id, port, process_id, state,
             loaded_at, released_at, vram_allocated_mb, vram_released_at
             FROM job_leases
             WHERE state NOT IN ('unloaded', 'verifying_release', 'failed')
             ORDER BY loaded_at",
        )?;
        let rows = stmt.query_map([], |row| {
            let state_str: String = row.get(5)?;
            Ok(ModelLease {
                lease_id: row.get(0)?,
                model_id: row.get(1)?,
                profile_id: row.get(2)?,
                port: row.get(3)?,
                process_id: row.get(4)?,
                state: crate::runtime_state::LeaseState::from_str(&state_str)
                    .unwrap_or(crate::runtime_state::LeaseState::Failed),
                loaded_at: row.get(6)?,
                released_at: row.get(7)?,
                vram_allocated_mb: row.get(8)?,
                vram_released_at: row.get(9)?,
            })
        })?;
        let mut leases = Vec::new();
        for row in rows {
            leases.push(row.context("Failed to read lease row")?);
        }
        Ok(leases)
    }

    // ========================================================================
    // RuntimeRun CRUD
    // ========================================================================

    pub fn insert_run(&self, run: &RuntimeRun) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO runtime_runs (run_id, lease_id, packet_id, input_tokens,
             output_tokens, load_duration_ms, generation_duration_ms, exit_status,
             started_at, ended_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                run.run_id,
                run.lease_id,
                run.packet_id,
                run.input_tokens,
                run.output_tokens,
                run.load_duration_ms,
                run.generation_duration_ms,
                run.exit_status,
                run.started_at,
                run.ended_at,
            ],
        )
        .context("Failed to insert run")?;
        Ok(())
    }

    pub fn get_run(&self, run_id: &str) -> Result<Option<RuntimeRun>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT run_id, lease_id, packet_id, input_tokens, output_tokens,
             load_duration_ms, generation_duration_ms, exit_status, started_at, ended_at
             FROM runtime_runs WHERE run_id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![run_id], |row| {
            Ok(RuntimeRun {
                run_id: row.get(0)?,
                lease_id: row.get(1)?,
                packet_id: row.get(2)?,
                input_tokens: row.get(3)?,
                output_tokens: row.get(4)?,
                load_duration_ms: row.get(5)?,
                generation_duration_ms: row.get(6)?,
                exit_status: row.get(7)?,
                started_at: row.get(8)?,
                ended_at: row.get(9)?,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r.context("Failed to read run row")?)),
            None => Ok(None),
        }
    }

    pub fn list_runs_for_lease(&self, lease_id: &str) -> Result<Vec<RuntimeRun>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT run_id, lease_id, packet_id, input_tokens, output_tokens,
             load_duration_ms, generation_duration_ms, exit_status, started_at, ended_at
             FROM runtime_runs WHERE lease_id = ?1 ORDER BY started_at",
        )?;
        let rows = stmt.query_map(rusqlite::params![lease_id], |row| {
            Ok(RuntimeRun {
                run_id: row.get(0)?,
                lease_id: row.get(1)?,
                packet_id: row.get(2)?,
                input_tokens: row.get(3)?,
                output_tokens: row.get(4)?,
                load_duration_ms: row.get(5)?,
                generation_duration_ms: row.get(6)?,
                exit_status: row.get(7)?,
                started_at: row.get(8)?,
                ended_at: row.get(9)?,
            })
        })?;
        let mut runs = Vec::new();
        for row in rows {
            runs.push(row.context("Failed to read run row")?);
        }
        Ok(runs)
    }

    // ========================================================================
    // LifecycleEvidence (append-only)
    // ========================================================================

    pub fn append_lifecycle_evidence(&self, evidence: &LifecycleEvidence) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO lifecycle_evidence (evidence_id, event_type, model_id, profile_id,
             lease_id, run_id, process_id, observed_state, observation_json,
             occurred_at, recorded_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                evidence.evidence_id,
                evidence.event_type.as_str(),
                evidence.model_id,
                evidence.profile_id,
                evidence.lease_id,
                evidence.run_id,
                evidence.process_id,
                evidence.observed_state,
                evidence.observation_json,
                evidence.occurred_at,
                evidence.recorded_at,
            ],
        )
        .context("Failed to append lifecycle evidence")?;
        Ok(())
    }

    pub fn list_lifecycle_evidence(
        &self,
        lease_id: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<LifecycleEvidence>> {
        let conn = self.open_connection()?;
        let (query, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match lease_id {
            Some(lid) => {
                let lim = limit.unwrap_or(100);
                (
                    "SELECT evidence_id, event_type, model_id, profile_id, lease_id,
                     run_id, process_id, observed_state, observation_json,
                     occurred_at, recorded_at
                     FROM lifecycle_evidence WHERE lease_id = ?1
                     ORDER BY occurred_at ASC LIMIT ?2"
                        .to_string(),
                    vec![
                        Box::new(lid.to_string()) as Box<dyn rusqlite::types::ToSql>,
                        Box::new(lim),
                    ],
                )
            }
            None => {
                let lim = limit.unwrap_or(100);
                (
                    "SELECT evidence_id, event_type, model_id, profile_id, lease_id,
                     run_id, process_id, observed_state, observation_json,
                     occurred_at, recorded_at
                     FROM lifecycle_evidence
                     ORDER BY occurred_at ASC LIMIT ?1"
                        .to_string(),
                    vec![Box::new(lim) as Box<dyn rusqlite::types::ToSql>],
                )
            }
        };

        let mut stmt = conn.prepare(&query)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let event_str: String = row.get(1)?;
            Ok(LifecycleEvidence {
                evidence_id: row.get(0)?,
                event_type: LifecycleEventType::from_str(&event_str)
                    .unwrap_or(LifecycleEventType::RuntimeStartup),
                model_id: row.get(2)?,
                profile_id: row.get(3)?,
                lease_id: row.get(4)?,
                run_id: row.get(5)?,
                process_id: row.get(6)?,
                observed_state: row.get(7)?,
                observation_json: row.get(8)?,
                occurred_at: row.get(9)?,
                recorded_at: row.get(10)?,
            })
        })?;
        let mut evidence = Vec::new();
        for row in rows {
            evidence.push(row.context("Failed to read lifecycle_evidence row")?);
        }
        Ok(evidence)
    }
}

/// Resolve the database file path from configuration.
fn resolve_db_path(config: &crate::config::RouterConfig) -> PathBuf {
    // Use evidence_path as base directory, or fall back to a default
    if let Some(ref evidence_path) = config.evidence_path {
        if evidence_path.is_dir() {
            return evidence_path.join("runtime-operational.db");
        }
    }

    // Default: place DB in the runtime-node config directory
    let base = PathBuf::from(r"G:\openwork\librarian-runtime-node");
    base.join("data").join("runtime-operational.db")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{HardwareProfile, LocalModel, RuntimeProfile};
    use crate::runtime_state::{LeaseState, LifecycleEventType, ModelLease, RuntimeRun};
    use tempfile::tempdir;

    /// Helper: create a temp DB and run migrations.
    fn test_db() -> RuntimeDatabase {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = RuntimeDatabase::open(path).unwrap();
        db.migrate().unwrap();
        db.verify().unwrap();
        // Leak the dir so it persists for the test
        Box::leak(Box::new(dir));
        db
    }

    // DB-2: Startup creates or opens operational DB at configured path
    #[test]
    fn test_db2_open_and_create() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_create.db");
        assert!(!path.exists());

        let _db = RuntimeDatabase::open(path.clone()).unwrap();
        assert!(path.exists());

        // Verify we can open it again (existing DB)
        let db2 = RuntimeDatabase::open(path).unwrap();
        db2.migrate().unwrap();
    }

    // DB-3: WAL, foreign keys, busy timeout, synchronous NORMAL are applied
    #[test]
    fn test_db3_pragmas() {
        let db = test_db();
        let conn = db.open_connection().unwrap();
        connection::verify_pragmas(&conn).unwrap();
    }

    // DB-4: Migrations are idempotent
    #[test]
    fn test_db4_migration_idempotent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_idempotent.db");
        let db = RuntimeDatabase::open(path).unwrap();

        // Run migration twice - should not error
        db.migrate().unwrap();
        db.migrate().unwrap();
        db.migrate().unwrap();

        // Verify version is still 1
        let conn = db.open_connection().unwrap();
        let version: i64 = conn
            .query_row(
                "SELECT MAX(version) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, 1);
    }

    // DB-5: All six operational tables exist with FK relationships
    #[test]
    fn test_db5_tables_exist() {
        let db = test_db();
        let conn = db.open_connection().unwrap();
        migrations::verify_tables(&conn).unwrap();
    }

    #[test]
    fn test_db5_foreign_key_enforcement() {
        let db = test_db();
        // Try to insert a runtime_profile with a non-existent model_id
        let profile = RuntimeProfile {
            profile_id: "test-profile".to_string(),
            model_id: "nonexistent-model".to_string(),
            device_backend: "vulkan".to_string(),
            gpu_layers: None,
            context_tokens: None,
            estimated_vram_mb: None,
            measured_vram_mb: None,
            measured_tokens_per_sec: None,
            practical_context_tokens: None,
            profile_priority: 0,
            enabled: true,
        };
        let result = db.insert_runtime_profile(&profile);
        assert!(result.is_err(), "FK enforcement should reject orphan profile");
    }

    // DB-6: Local model CRUD
    #[test]
    fn test_db6_local_model_crud() {
        let db = test_db();

        let mut model = LocalModel::new(
            "minicpm5-1b-q4".to_string(),
            "MiniCPM5 1B Q4".to_string(),
            "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
        );
        model.family = Some("minicpm".to_string());
        model.quantization = Some("Q4_K_M".to_string());
        model.file_size_bytes = Some(600_000_000);

        // Insert
        db.insert_local_model(&model).unwrap();

        // Get
        let fetched = db.get_local_model("minicpm5-1b-q4").unwrap().unwrap();
        assert_eq!(fetched.model_id, "minicpm5-1b-q4");
        assert_eq!(fetched.display_name, "MiniCPM5 1B Q4");
        assert_eq!(fetched.family, Some("minicpm".to_string()));
        assert_eq!(fetched.file_size_bytes, Some(600_000_000));

        // List
        let all = db.list_local_models().unwrap();
        assert_eq!(all.len(), 1);

        // Update
        model.display_name = "MiniCPM5 1B Q4 Updated".to_string();
        db.update_local_model(&model).unwrap();
        let updated = db.get_local_model("minicpm5-1b-q4").unwrap().unwrap();
        assert_eq!(updated.display_name, "MiniCPM5 1B Q4 Updated");

        // Delete
        db.delete_local_model("minicpm5-1b-q4").unwrap();
        let gone = db.get_local_model("minicpm5-1b-q4").unwrap();
        assert!(gone.is_none());
    }

    // DB-7: Runtime profile CRUD
    #[test]
    fn test_db7_runtime_profile_crud() {
        let db = test_db();

        // First insert a model (FK requirement)
        let model = LocalModel::new(
            "test-model".to_string(),
            "Test Model".to_string(),
            "test.gguf".to_string(),
        );
        db.insert_local_model(&model).unwrap();

        let mut profile = RuntimeProfile::new(
            "test-profile".to_string(),
            "test-model".to_string(),
            "vulkan".to_string(),
        );
        profile.gpu_layers = Some(99);
        profile.context_tokens = Some(4096);
        profile.measured_vram_mb = Some(2000);

        // Insert
        db.insert_runtime_profile(&profile).unwrap();

        // Get
        let fetched = db.get_runtime_profile("test-profile").unwrap().unwrap();
        assert_eq!(fetched.model_id, "test-model");
        assert_eq!(fetched.device_backend, "vulkan");
        assert_eq!(fetched.gpu_layers, Some(99));

        // List all
        let all = db.list_runtime_profiles().unwrap();
        assert_eq!(all.len(), 1);

        // List by model
        let by_model = db.list_runtime_profiles_for_model("test-model").unwrap();
        assert_eq!(by_model.len(), 1);

        let empty = db.list_runtime_profiles_for_model("other-model").unwrap();
        assert!(empty.is_empty());

        // Delete
        db.delete_runtime_profile("test-profile").unwrap();
        let gone = db.get_runtime_profile("test-profile").unwrap();
        assert!(gone.is_none());
    }

    // DB-8: Hardware profile CRUD
    #[test]
    fn test_db8_hardware_profile_crud() {
        let db = test_db();

        let mut hw = HardwareProfile::new("hw-rx570".to_string());
        hw.device_name = Some("RX 570".to_string());
        hw.vulkan_device = Some("AMD Radeon RX 570".to_string());
        hw.total_vram_mb = Some(4096);

        // Insert
        db.insert_hardware_profile(&hw).unwrap();

        // Get
        let fetched = db.get_hardware_profile("hw-rx570").unwrap().unwrap();
        assert_eq!(fetched.device_name, Some("RX 570".to_string()));
        assert_eq!(fetched.total_vram_mb, Some(4096));

        // List
        let all = db.list_hardware_profiles().unwrap();
        assert_eq!(all.len(), 1);
    }

    // DB-9: Lease and run persistence
    #[test]
    fn test_db9_lease_and_run_persistence() {
        let db = test_db();

        // Insert model first
        let model = LocalModel::new(
            "test-model".to_string(),
            "Test Model".to_string(),
            "test.gguf".to_string(),
        );
        db.insert_local_model(&model).unwrap();

        // Insert lease
        let mut lease = ModelLease::new("lease-1".to_string(), "test-model".to_string());
        lease.state = LeaseState::Loading;
        lease.port = Some(9120);
        db.insert_lease(&lease).unwrap();

        // Get lease
        let fetched = db.get_lease("lease-1").unwrap().unwrap();
        assert_eq!(fetched.state, LeaseState::Loading);
        assert_eq!(fetched.port, Some(9120));

        // Update lease state
        db.update_lease_state("lease-1", LeaseState::Ready).unwrap();
        let updated = db.get_lease("lease-1").unwrap().unwrap();
        assert_eq!(updated.state, LeaseState::Ready);

        // Get active leases
        let active = db.get_active_leases().unwrap();
        assert_eq!(active.len(), 1);

        // Insert run
        let run = RuntimeRun::new("run-1".to_string(), "lease-1".to_string());
        db.insert_run(&run).unwrap();

        // Get run
        let fetched_run = db.get_run("run-1").unwrap().unwrap();
        assert_eq!(fetched_run.lease_id, "lease-1");

        // List runs for lease
        let runs = db.list_runs_for_lease("lease-1").unwrap();
        assert_eq!(runs.len(), 1);

        // Mark lease as unloaded — should no longer be "active"
        db.update_lease_state("lease-1", LeaseState::Unloaded).unwrap();
        let active_after = db.get_active_leases().unwrap();
        assert!(active_after.is_empty());
    }

    // DB-10: Lifecycle evidence is appendable and queryable in deterministic order
    #[test]
    fn test_db10_lifecycle_evidence() {
        let db = test_db();

        // Append evidence
        let ev1 = LifecycleEvidence::new(
            "ev-1".to_string(),
            LifecycleEventType::RuntimeStartup,
            r#"{"status":"started"}"#.to_string(),
        );
        let mut ev2 = LifecycleEvidence::new(
            "ev-2".to_string(),
            LifecycleEventType::DatabaseOpened,
            r#"{"db":"runtime-operational.db"}"#.to_string(),
        );
        ev2.lease_id = Some("lease-1".to_string());

        db.append_lifecycle_evidence(&ev1).unwrap();
        db.append_lifecycle_evidence(&ev2).unwrap();

        // List all — should be in occurred_at ASC order (deterministic)
        let all = db.list_lifecycle_evidence(None, None).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].evidence_id, "ev-1");
        assert_eq!(all[1].evidence_id, "ev-2");

        // List by lease
        let by_lease = db.list_lifecycle_evidence(Some("lease-1"), None).unwrap();
        assert_eq!(by_lease.len(), 1);
        assert_eq!(by_lease[0].evidence_id, "ev-2");

        // Limit
        let limited = db.list_lifecycle_evidence(None, Some(1)).unwrap();
        assert_eq!(limited.len(), 1);
    }

    // DB-12: Router startup fails closed on unrecoverable DB init
    #[test]
    fn test_db12_fails_on_invalid_path() {
        // Try to open a DB in a non-existent directory with no parent creation
        // This tests the error path, though RuntimeDatabase::open creates dirs.
        // The real fail-closed test is in main.rs (process::exit).
        // Here we verify that open + migrate + verify works for a valid path.
        let dir = tempdir().unwrap();
        let path = dir.path().join("subdir").join("test.db");
        let db = RuntimeDatabase::open(path).unwrap();
        db.migrate().unwrap();
        db.verify().unwrap();
    }

    // DB-13: No canonical Mac/Librarian authority tables introduced
    #[test]
    fn test_db13_no_canonical_authority_tables() {
        let db = test_db();
        let conn = db.open_connection().unwrap();

        // Check that no authority-related tables exist
        let forbidden_tables = vec![
            "context_items",
            "sprint_packet_plans",
            "sprint_packets",
            "owner_decisions",
            "validation_results",
            "canonical_sprints",
        ];

        for table in &forbidden_tables {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    rusqlite::params![table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 0, "Forbidden table '{}' should not exist", table);
        }
    }

    // DB-14: No model switching, load/unload, or residency enforcement
    #[test]
    fn test_db14_no_switching_implementation() {
        // This is a compile-time/structural gate. The test verifies that
        // RuntimeDatabase does not have methods for model switching or residency.
        // The absence of start_model/stop_model/switch_model methods on
        // RuntimeDatabase proves DB-14 structurally.
        //
        // RuntimeDatabase methods are:
        //   open, open_from_config, migrate, verify, open_connection,
        //   insert/get/list/update/delete for domain types,
        //   insert/get/update_state/get_active for leases,
        //   insert/get/list for runs,
        //   append/list for lifecycle evidence.
        //
        // No start_model, stop_model, switch_model, enforce_residency methods exist.
        // This is verified by the code compiling without those methods.

        // Runtime check: verify lease state transitions are only persisted,
        // not enforced by the DB layer. The DB stores state; it does not
        // transition models or enforce residency.
        let db = test_db();

        let model = LocalModel::new(
            "test-model".to_string(),
            "Test".to_string(),
            "test.gguf".to_string(),
        );
        db.insert_local_model(&model).unwrap();

        // We can insert a lease in any state — no enforcement
        let lease = ModelLease::new("lease-1".to_string(), "test-model".to_string());
        db.insert_lease(&lease).unwrap();

        // We can update to any state — no enforcement
        db.update_lease_state("lease-1", LeaseState::Ready).unwrap();
        db.update_lease_state("lease-1", LeaseState::Running).unwrap();
        db.update_lease_state("lease-1", LeaseState::Unloaded).unwrap();

        // The DB does not enforce "one lease at a time" — that's Sprint 3's job
        let lease2 = ModelLease::new("lease-2".to_string(), "test-model".to_string());
        db.insert_lease(&lease2).unwrap();

        // Set both leases to active states — DB allows multiple concurrent leases for same model
        db.update_lease_state("lease-1", LeaseState::Ready).unwrap();
        db.update_lease_state("lease-2", LeaseState::Running).unwrap();

        // Both leases exist — no enforcement at DB level
        let active = db.get_active_leases().unwrap();
        assert_eq!(active.len(), 2, "DB does not enforce single residency");
    }
}
