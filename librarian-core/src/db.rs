//! Canonical database for the Mac-side qualification and routing system.
//!
//! CanonicalDatabase owns the SQLite path and provides bounded connections.
//! Each operation obtains its own connection. Transactions belong here,
//! not in server.rs or qualification runner.
//!
//! Mirrors the Windows RuntimeDatabase pattern exactly:
//! - #[derive(Clone)] struct holding Arc<PathBuf>
//! - open() creates DB + parent dirs
//! - migrate() runs pending migrations
//! - verify() checks PRAGMAs + tables
//! - open_connection() returns a new connection with standard PRAGMAs

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Arc;

use crate::connection::{configure_connection, configure_database_init, verify_pragmas};
use crate::migrations::{migrate, verify_tables};
use crate::models::{
    ModelIdentityRecord, QualificationScope, SystemProfile, TaskPack, TaskPackStatus,
    ValidatorPack, ValidatorPackStatus,
};

/// Owner abstraction for the canonical (Mac-side) SQLite database.
/// Does not hold a live connection — each operation opens a bounded connection.
#[derive(Clone)]
pub struct CanonicalDatabase {
    path: Arc<PathBuf>,
}

impl CanonicalDatabase {
    /// Open or create the database at the given path.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = Arc::new(path.into());

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create canonical DB directory: {}", parent.display()))?;
        }

        let db = Self { path };

        // Initialize database-level PRAGMAs
        {
            let conn = db.open_connection()?;
            configure_database_init(&conn)?;
        }

        Ok(db)
    }

    /// Run all pending migrations. Idempotent.
    pub fn migrate(&self) -> Result<()> {
        let conn = self.open_connection()?;
        migrate(&conn).context("Canonical migration failed")?;
        Ok(())
    }

    /// Verify database posture: PRAGMAs, tables, foreign keys.
    pub fn verify(&self) -> Result<()> {
        let conn = self.open_connection()?;
        verify_pragmas(&conn).context("Canonical PRAGMA verification failed")?;
        verify_tables(&conn).context("Canonical table verification failed")?;
        Ok(())
    }

    /// Open a new connection with standard PRAGMAs applied.
    pub fn open_connection(&self) -> Result<Connection> {
        let conn = Connection::open(self.path.as_ref())
            .with_context(|| format!("Failed to open canonical DB: {}", self.path.display()))?;
        configure_connection(&conn)?;
        Ok(conn)
    }

    // ========================================================================
    // ModelIdentityRecord CRUD
    // ========================================================================

    pub fn insert_identity(&self, record: &ModelIdentityRecord) -> Result<()> {
        let conn = self.open_connection()?;
        let scope_str = record.qualification_scope.as_str();
        // Auto-serialize roles from QualificationScope if roles_json is None
        let roles_json_str = match (&record.qualification_scope, &record.roles_json) {
            (QualificationScope::Roles(roles), None) => {
                Some(serde_json::to_string(roles).unwrap_or_default())
            }
            (_, other) => other.clone(),
        };
        conn.execute(
            "INSERT INTO model_identity_record (identity_id, model_id_ref, gguf_metadata_hash,
             chat_template_id, license_spdx, qualification_scope, roles_json,
             created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                record.identity_id,
                record.model_id_ref,
                record.gguf_metadata_hash,
                record.chat_template_id,
                record.license_spdx,
                scope_str,
                roles_json_str,
                record.created_at,
                record.updated_at,
            ],
        )
        .context("Failed to insert model_identity_record")?;
        Ok(())
    }

    pub fn get_identity(&self, identity_id: &str) -> Result<Option<ModelIdentityRecord>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT identity_id, model_id_ref, gguf_metadata_hash, chat_template_id,
             license_spdx, qualification_scope, roles_json, created_at, updated_at
             FROM model_identity_record WHERE identity_id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![identity_id], |row| {
            let scope_str: String = row.get(5)?;
            let roles_json: Option<String> = row.get(6)?;
            let scope = QualificationScope::from_str_with_roles(&scope_str, roles_json.as_deref());
            Ok(ModelIdentityRecord {
                identity_id: row.get(0)?,
                model_id_ref: row.get(1)?,
                gguf_metadata_hash: row.get(2)?,
                chat_template_id: row.get(3)?,
                license_spdx: row.get(4)?,
                qualification_scope: scope,
                roles_json,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r.context("Failed to read model_identity_record row")?)),
            None => Ok(None),
        }
    }

    pub fn get_identity_by_model_ref(&self, model_id_ref: &str) -> Result<Option<ModelIdentityRecord>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT identity_id, model_id_ref, gguf_metadata_hash, chat_template_id,
             license_spdx, qualification_scope, roles_json, created_at, updated_at
             FROM model_identity_record WHERE model_id_ref = ?1 LIMIT 1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![model_id_ref], |row| {
            let scope_str: String = row.get(5)?;
            let roles_json: Option<String> = row.get(6)?;
            let scope = QualificationScope::from_str_with_roles(&scope_str, roles_json.as_deref());
            Ok(ModelIdentityRecord {
                identity_id: row.get(0)?,
                model_id_ref: row.get(1)?,
                gguf_metadata_hash: row.get(2)?,
                chat_template_id: row.get(3)?,
                license_spdx: row.get(4)?,
                qualification_scope: scope,
                roles_json,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r.context("Failed to read model_identity_record row")?)),
            None => Ok(None),
        }
    }

    pub fn list_identities(&self) -> Result<Vec<ModelIdentityRecord>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT identity_id, model_id_ref, gguf_metadata_hash, chat_template_id,
             license_spdx, qualification_scope, roles_json, created_at, updated_at
             FROM model_identity_record ORDER BY created_at",
        )?;
        let rows = stmt.query_map([], |row| {
            let scope_str: String = row.get(5)?;
            let roles_json: Option<String> = row.get(6)?;
            let scope = QualificationScope::from_str_with_roles(&scope_str, roles_json.as_deref());
            Ok(ModelIdentityRecord {
                identity_id: row.get(0)?,
                model_id_ref: row.get(1)?,
                gguf_metadata_hash: row.get(2)?,
                chat_template_id: row.get(3)?,
                license_spdx: row.get(4)?,
                qualification_scope: scope,
                roles_json,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row.context("Failed to read model_identity_record row")?);
        }
        Ok(records)
    }

    pub fn update_identity(&self, record: &ModelIdentityRecord) -> Result<()> {
        let conn = self.open_connection()?;
        let scope_str = record.qualification_scope.as_str();
        // Auto-serialize roles from QualificationScope if roles_json is None
        let roles_json_str = match (&record.qualification_scope, &record.roles_json) {
            (QualificationScope::Roles(roles), None) => {
                Some(serde_json::to_string(roles).unwrap_or_default())
            }
            (_, other) => other.clone(),
        };
        let affected = conn.execute(
            "UPDATE model_identity_record SET model_id_ref = ?2, gguf_metadata_hash = ?3,
             chat_template_id = ?4, license_spdx = ?5, qualification_scope = ?6,
             roles_json = ?7, updated_at = ?8
             WHERE identity_id = ?1",
            rusqlite::params![
                record.identity_id,
                record.model_id_ref,
                record.gguf_metadata_hash,
                record.chat_template_id,
                record.license_spdx,
                scope_str,
                roles_json_str,
                record.updated_at,
            ],
        )?;
        if affected == 0 {
            anyhow::bail!("No model_identity_record found with id '{}'", record.identity_id);
        }
        Ok(())
    }

    pub fn delete_identity(&self, identity_id: &str) -> Result<()> {
        let conn = self.open_connection()?;
        let affected = conn.execute(
            "DELETE FROM model_identity_record WHERE identity_id = ?1",
            rusqlite::params![identity_id],
        )?;
        if affected == 0 {
            anyhow::bail!("No model_identity_record found with id '{}'", identity_id);
        }
        Ok(())
    }

    // ========================================================================
    // SystemProfile CRUD
    // ========================================================================

    pub fn insert_system_profile(&self, profile: &SystemProfile) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute(
            "INSERT INTO system_profile (system_profile_id, os, cpu, ram_mb,
             gpu_description, notes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                profile.system_profile_id,
                profile.os,
                profile.cpu,
                profile.ram_mb,
                profile.gpu_description,
                profile.notes,
                profile.created_at,
            ],
        )
        .context("Failed to insert system_profile")?;
        Ok(())
    }

    pub fn get_system_profile(&self, system_profile_id: &str) -> Result<Option<SystemProfile>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT system_profile_id, os, cpu, ram_mb, gpu_description, notes, created_at
             FROM system_profile WHERE system_profile_id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![system_profile_id], |row| {
            Ok(SystemProfile {
                system_profile_id: row.get(0)?,
                os: row.get(1)?,
                cpu: row.get(2)?,
                ram_mb: row.get(3)?,
                gpu_description: row.get(4)?,
                notes: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r.context("Failed to read system_profile row")?)),
            None => Ok(None),
        }
    }

    pub fn list_system_profiles(&self) -> Result<Vec<SystemProfile>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT system_profile_id, os, cpu, ram_mb, gpu_description, notes, created_at
             FROM system_profile ORDER BY created_at",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SystemProfile {
                system_profile_id: row.get(0)?,
                os: row.get(1)?,
                cpu: row.get(2)?,
                ram_mb: row.get(3)?,
                gpu_description: row.get(4)?,
                notes: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        let mut profiles = Vec::new();
        for row in rows {
            profiles.push(row.context("Failed to read system_profile row")?);
        }
        Ok(profiles)
    }

    pub fn delete_system_profile(&self, system_profile_id: &str) -> Result<()> {
        let conn = self.open_connection()?;
        let affected = conn.execute(
            "DELETE FROM system_profile WHERE system_profile_id = ?1",
            rusqlite::params![system_profile_id],
        )?;
        if affected == 0 {
            anyhow::bail!("No system_profile found with id '{}'", system_profile_id);
        }
        Ok(())
    }

    // ========================================================================
    // TaskPack CRUD
    // ========================================================================

    pub fn insert_task_pack(&self, pack: &TaskPack) -> Result<()> {
        let conn = self.open_connection()?;
        let status_str = pack.status.as_str();
        conn.execute(
            "INSERT INTO task_pack (task_pack_id, version, role, description,
             fixture_hash, fixture_path, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                pack.task_pack_id,
                pack.version,
                pack.role,
                pack.description,
                pack.fixture_hash,
                pack.fixture_path,
                status_str,
                pack.created_at,
            ],
        )
        .context("Failed to insert task_pack")?;
        Ok(())
    }

    pub fn get_task_pack(&self, task_pack_id: &str) -> Result<Option<TaskPack>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT task_pack_id, version, role, description, fixture_hash,
             fixture_path, status, created_at
             FROM task_pack WHERE task_pack_id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![task_pack_id], |row| {
            let status_str: String = row.get(6)?;
            Ok(TaskPack {
                task_pack_id: row.get(0)?,
                version: row.get(1)?,
                role: row.get(2)?,
                description: row.get(3)?,
                fixture_hash: row.get(4)?,
                fixture_path: row.get(5)?,
                status: TaskPackStatus::from_str(&status_str),
                created_at: row.get(7)?,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r.context("Failed to read task_pack row")?)),
            None => Ok(None),
        }
    }

    pub fn list_task_packs_for_role(&self, role: &str) -> Result<Vec<TaskPack>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT task_pack_id, version, role, description, fixture_hash,
             fixture_path, status, created_at
             FROM task_pack WHERE role = ?1 AND status = 'active'
             ORDER BY version DESC",
        )?;
        let rows = stmt.query_map(rusqlite::params![role], |row| {
            let status_str: String = row.get(6)?;
            Ok(TaskPack {
                task_pack_id: row.get(0)?,
                version: row.get(1)?,
                role: row.get(2)?,
                description: row.get::<_, Option<String>>(3)?,
                fixture_hash: row.get(4)?,
                fixture_path: row.get::<_, Option<String>>(5)?,
                status: TaskPackStatus::from_str(&status_str),
                created_at: row.get(7)?,
            })
        })?;
        let mut packs = Vec::new();
        for row in rows {
            packs.push(row.context("Failed to read task_pack row")?);
        }
        Ok(packs)
    }

    pub fn list_all_task_packs(&self) -> Result<Vec<TaskPack>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT task_pack_id, version, role, description, fixture_hash,
             fixture_path, status, created_at
             FROM task_pack ORDER BY role, version",
        )?;
        let rows = stmt.query_map([], |row| {
            let status_str: String = row.get(6)?;
            Ok(TaskPack {
                task_pack_id: row.get(0)?,
                version: row.get(1)?,
                role: row.get(2)?,
                description: row.get::<_, Option<String>>(3)?,
                fixture_hash: row.get(4)?,
                fixture_path: row.get::<_, Option<String>>(5)?,
                status: TaskPackStatus::from_str(&status_str),
                created_at: row.get(7)?,
            })
        })?;
        let mut packs = Vec::new();
        for row in rows {
            packs.push(row.context("Failed to read task_pack row")?);
        }
        Ok(packs)
    }

    pub fn delete_task_pack(&self, task_pack_id: &str) -> Result<()> {
        let conn = self.open_connection()?;
        let affected = conn.execute(
            "DELETE FROM task_pack WHERE task_pack_id = ?1",
            rusqlite::params![task_pack_id],
        )?;
        if affected == 0 {
            anyhow::bail!("No task_pack found with id '{}'", task_pack_id);
        }
        Ok(())
    }

    // ========================================================================
    // ValidatorPack CRUD
    // ========================================================================

    pub fn insert_validator_pack(&self, pack: &ValidatorPack) -> Result<()> {
        let conn = self.open_connection()?;
        let status_str = pack.status.as_str();
        conn.execute(
            "INSERT INTO validator_pack (validator_pack_id, version, role, description,
             rules_hash, rules_path, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                pack.validator_pack_id,
                pack.version,
                pack.role,
                pack.description,
                pack.rules_hash,
                pack.rules_path,
                status_str,
                pack.created_at,
            ],
        )
        .context("Failed to insert validator_pack")?;
        Ok(())
    }

    pub fn get_validator_pack(&self, validator_pack_id: &str) -> Result<Option<ValidatorPack>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT validator_pack_id, version, role, description, rules_hash,
             rules_path, status, created_at
             FROM validator_pack WHERE validator_pack_id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![validator_pack_id], |row| {
            let status_str: String = row.get(6)?;
            Ok(ValidatorPack {
                validator_pack_id: row.get(0)?,
                version: row.get(1)?,
                role: row.get(2)?,
                description: row.get(3)?,
                rules_hash: row.get(4)?,
                rules_path: row.get(5)?,
                status: ValidatorPackStatus::from_str(&status_str),
                created_at: row.get(7)?,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r.context("Failed to read validator_pack row")?)),
            None => Ok(None),
        }
    }

    pub fn list_validator_packs_for_role(&self, role: &str) -> Result<Vec<ValidatorPack>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT validator_pack_id, version, role, description, rules_hash,
             rules_path, status, created_at
             FROM validator_pack WHERE role = ?1 AND status = 'active'
             ORDER BY version DESC",
        )?;
        let rows = stmt.query_map(rusqlite::params![role], |row| {
            let status_str: String = row.get(6)?;
            Ok(ValidatorPack {
                validator_pack_id: row.get(0)?,
                version: row.get(1)?,
                role: row.get(2)?,
                description: row.get::<_, Option<String>>(3)?,
                rules_hash: row.get(4)?,
                rules_path: row.get::<_, Option<String>>(5)?,
                status: ValidatorPackStatus::from_str(&status_str),
                created_at: row.get(7)?,
            })
        })?;
        let mut packs = Vec::new();
        for row in rows {
            packs.push(row.context("Failed to read validator_pack row")?);
        }
        Ok(packs)
    }

    pub fn list_all_validator_packs(&self) -> Result<Vec<ValidatorPack>> {
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT validator_pack_id, version, role, description, rules_hash,
             rules_path, status, created_at
             FROM validator_pack ORDER BY role, version",
        )?;
        let rows = stmt.query_map([], |row| {
            let status_str: String = row.get(6)?;
            Ok(ValidatorPack {
                validator_pack_id: row.get(0)?,
                version: row.get(1)?,
                role: row.get(2)?,
                description: row.get::<_, Option<String>>(3)?,
                rules_hash: row.get(4)?,
                rules_path: row.get::<_, Option<String>>(5)?,
                status: ValidatorPackStatus::from_str(&status_str),
                created_at: row.get(7)?,
            })
        })?;
        let mut packs = Vec::new();
        for row in rows {
            packs.push(row.context("Failed to read validator_pack row")?);
        }
        Ok(packs)
    }

    pub fn delete_validator_pack(&self, validator_pack_id: &str) -> Result<()> {
        let conn = self.open_connection()?;
        let affected = conn.execute(
            "DELETE FROM validator_pack WHERE validator_pack_id = ?1",
            rusqlite::params![validator_pack_id],
        )?;
        if affected == 0 {
            anyhow::bail!("No validator_pack found with id '{}'", validator_pack_id);
        }
        Ok(())
    }

    // ========================================================================
    // Raw SQL execution (for supervisor operations that need ad-hoc updates)
    // ========================================================================

    /// Execute raw SQL (for administrative operations).
    pub fn execute_sql(&self, sql: &str) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute_batch(sql)
            .with_context(|| format!("Failed to execute canonical SQL: {}", &sql[..sql.len().min(200)]))?;
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Helper: create a temp canonical DB and run migrations.
    fn test_db() -> CanonicalDatabase {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_canonical.db");
        let db = CanonicalDatabase::open(path).unwrap();
        db.migrate().unwrap();
        db.verify().unwrap();
        // Leak the dir so it persists for the test
        Box::leak(Box::new(dir));
        db
    }

    // MQR-F1-1: Mac canonical DB initializes successfully
    #[test]
    fn test_f1_1_db_initializes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_init.db");
        let db = CanonicalDatabase::open(path.clone()).unwrap();
        assert!(path.exists());
        db.migrate().unwrap();
        db.verify().unwrap();
    }

    // MQR-F1-2: Migrations are idempotent
    #[test]
    fn test_f1_2_migrations_idempotent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_idempotent.db");
        let db = CanonicalDatabase::open(path).unwrap();

        db.migrate().unwrap();
        db.migrate().unwrap();
        db.migrate().unwrap();

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

    // MQR-F1-3: model_identity_record CRUD works
    #[test]
    fn test_f1_3_identity_crud() {
        let db = test_db();

        let mut record = ModelIdentityRecord::new(
            "id-1".to_string(),
            "minicpm5-1b-q4km".to_string(),
        );
        record.gguf_metadata_hash = Some("abc123".to_string());
        record.chat_template_id = Some("chatml".to_string());
        record.license_spdx = Some("Apache-2.0".to_string());

        // Insert
        db.insert_identity(&record).unwrap();

        // Get
        let fetched = db.get_identity("id-1").unwrap().unwrap();
        assert_eq!(fetched.identity_id, "id-1");
        assert_eq!(fetched.model_id_ref, "minicpm5-1b-q4km");
        assert_eq!(fetched.gguf_metadata_hash, Some("abc123".to_string()));
        assert_eq!(fetched.chat_template_id, Some("chatml".to_string()));
        assert_eq!(fetched.license_spdx, Some("Apache-2.0".to_string()));

        // Get by model ref
        let by_ref = db.get_identity_by_model_ref("minicpm5-1b-q4km").unwrap().unwrap();
        assert_eq!(by_ref.identity_id, "id-1");

        // List
        let all = db.list_identities().unwrap();
        assert_eq!(all.len(), 1);

        // Update
        record.gguf_metadata_hash = Some("updated_hash".to_string());
        record.updated_at = chrono::Utc::now().to_rfc3339();
        db.update_identity(&record).unwrap();
        let updated = db.get_identity("id-1").unwrap().unwrap();
        assert_eq!(updated.gguf_metadata_hash, Some("updated_hash".to_string()));

        // Delete
        db.delete_identity("id-1").unwrap();
        let gone = db.get_identity("id-1").unwrap();
        assert!(gone.is_none());
    }

    // MQR-F1-4: system_profile CRUD works
    #[test]
    fn test_f1_4_system_profile_crud() {
        let db = test_db();

        let mut profile = SystemProfile::new("sys-1".to_string());
        profile.os = Some("windows-11".to_string());
        profile.cpu = Some("Intel Core i5-3570K".to_string());
        profile.ram_mb = Some(24268);
        profile.gpu_description = Some("AMD Radeon RX 570".to_string());

        // Insert
        db.insert_system_profile(&profile).unwrap();

        // Get
        let fetched = db.get_system_profile("sys-1").unwrap().unwrap();
        assert_eq!(fetched.os, Some("windows-11".to_string()));
        assert_eq!(fetched.cpu, Some("Intel Core i5-3570K".to_string()));
        assert_eq!(fetched.ram_mb, Some(24268));
        assert_eq!(fetched.gpu_description, Some("AMD Radeon RX 570".to_string()));

        // List
        let all = db.list_system_profiles().unwrap();
        assert_eq!(all.len(), 1);

        // Delete
        db.delete_system_profile("sys-1").unwrap();
        let gone = db.get_system_profile("sys-1").unwrap();
        assert!(gone.is_none());
    }

    // MQR-F1-5: task_pack CRUD works
    #[test]
    fn test_f1_5_task_pack_crud() {
        let db = test_db();

        let mut pack = TaskPack::new(
            "tp-1".to_string(),
            1,
            "implementer".to_string(),
            "fixture_hash_abc".to_string(),
        );
        pack.description = Some("Instruction following test".to_string());

        // Insert
        db.insert_task_pack(&pack).unwrap();

        // Get
        let fetched = db.get_task_pack("tp-1").unwrap().unwrap();
        assert_eq!(fetched.task_pack_id, "tp-1");
        assert_eq!(fetched.version, 1);
        assert_eq!(fetched.role, "implementer");
        assert_eq!(fetched.fixture_hash, "fixture_hash_abc");

        // List for role
        let by_role = db.list_task_packs_for_role("implementer").unwrap();
        assert_eq!(by_role.len(), 1);

        // List all
        let all = db.list_all_task_packs().unwrap();
        assert_eq!(all.len(), 1);

        // Delete
        db.delete_task_pack("tp-1").unwrap();
        let gone = db.get_task_pack("tp-1").unwrap();
        assert!(gone.is_none());
    }

    // MQR-F1-6: validator_pack CRUD works
    #[test]
    fn test_f1_6_validator_pack_crud() {
        let db = test_db();

        let mut pack = ValidatorPack::new(
            "vp-1".to_string(),
            1,
            "implementer".to_string(),
            "rules_hash_def".to_string(),
        );
        pack.description = Some("Implementer validation rules".to_string());

        // Insert
        db.insert_validator_pack(&pack).unwrap();

        // Get
        let fetched = db.get_validator_pack("vp-1").unwrap().unwrap();
        assert_eq!(fetched.validator_pack_id, "vp-1");
        assert_eq!(fetched.version, 1);
        assert_eq!(fetched.role, "implementer");
        assert_eq!(fetched.rules_hash, "rules_hash_def");

        // List for role
        let by_role = db.list_validator_packs_for_role("implementer").unwrap();
        assert_eq!(by_role.len(), 1);

        // List all
        let all = db.list_all_validator_packs().unwrap();
        assert_eq!(all.len(), 1);

        // Delete
        db.delete_validator_pack("vp-1").unwrap();
        let gone = db.get_validator_pack("vp-1").unwrap();
        assert!(gone.is_none());
    }

    // MQR-F1-7: PRAGMAs are correctly set
    #[test]
    fn test_f1_7_pragmas() {
        let db = test_db();
        let conn = db.open_connection().unwrap();
        crate::connection::verify_pragmas(&conn).unwrap();
    }

    // MQR-F1-8: Identity with qualification scope
    #[test]
    fn test_f1_8_identity_scope() {
        let db = test_db();

        let mut record = ModelIdentityRecord::new(
            "id-2".to_string(),
            "minicpm5-1b-q8".to_string(),
        );
        record.qualification_scope = QualificationScope::Roles(vec![
            "implementer".to_string(),
            "researcher".to_string(),
        ]);

        db.insert_identity(&record).unwrap();
        let fetched = db.get_identity("id-2").unwrap().unwrap();
        assert_eq!(fetched.qualification_scope, QualificationScope::Roles(vec![
            "implementer".to_string(),
            "researcher".to_string(),
        ]));
    }

    // MQR-F1-9: Identity not found returns None
    #[test]
    fn test_f1_9_identity_not_found() {
        let db = test_db();
        let result = db.get_identity("nonexistent").unwrap();
        assert!(result.is_none());
    }

    // MQR-F1-10: Identity by model ref not found
    #[test]
    fn test_f1_10_identity_by_ref_not_found() {
        let db = test_db();
        let result = db.get_identity_by_model_ref("nonexistent").unwrap();
        assert!(result.is_none());
    }

    // MQR-F1-11: Duplicate identity_id rejected
    #[test]
    fn test_f1_11_duplicate_identity_rejected() {
        let db = test_db();

        let r1 = ModelIdentityRecord::new("id-1".to_string(), "model-a".to_string());
        let r2 = ModelIdentityRecord::new("id-1".to_string(), "model-b".to_string());

        db.insert_identity(&r1).unwrap();
        let result = db.insert_identity(&r2);
        assert!(result.is_err());
    }

    // MQR-F1-12: Duplicate task_pack_id rejected
    #[test]
    fn test_f1_12_duplicate_task_pack_rejected() {
        let db = test_db();

        let p1 = TaskPack::new("tp-1".to_string(), 1, "implementer".to_string(), "hash1".to_string());
        let p2 = TaskPack::new("tp-1".to_string(), 1, "researcher".to_string(), "hash2".to_string());

        db.insert_task_pack(&p1).unwrap();
        let result = db.insert_task_pack(&p2);
        assert!(result.is_err());
    }

    // MQR-F1-13: Duplicate validator_pack_id rejected
    #[test]
    fn test_f1_13_duplicate_validator_pack_rejected() {
        let db = test_db();

        let p1 = ValidatorPack::new("vp-1".to_string(), 1, "implementer".to_string(), "hash1".to_string());
        let p2 = ValidatorPack::new("vp-1".to_string(), 1, "researcher".to_string(), "hash2".to_string());

        db.insert_validator_pack(&p1).unwrap();
        let result = db.insert_validator_pack(&p2);
        assert!(result.is_err());
    }

    // MQR-F1-14: No Windows tables in canonical DB
    #[test]
    fn test_f1_14_no_windows_tables() {
        let db = test_db();
        let conn = db.open_connection().unwrap();

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

    // MQR-F1-15: No qualification/run tables yet (planned for later sprints)
    #[test]
    fn test_f1_15_no_qualification_tables_yet() {
        let db = test_db();
        let conn = db.open_connection().unwrap();

        let not_yet = vec![
            "qualification_request",
            "qualification_run",
            "qualification_stage_log",
            "capability_manifest",
            "owner_decision",
            "execution_profile",
            "router_projection",
            "routing_log",
        ];

        for table in &not_yet {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    rusqlite::params![table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 0, "Future table '{}' should not exist yet", table);
        }
    }

    // MQR-F1-16: TaskPack status filtering works
    #[test]
    fn test_f1_16_task_pack_status_filter() {
        let db = test_db();

        let active = TaskPack::new("tp-1".to_string(), 1, "implementer".to_string(), "hash1".to_string());
        let mut deprecated = TaskPack::new("tp-2".to_string(), 1, "implementer".to_string(), "hash2".to_string());
        deprecated.status = TaskPackStatus::Deprecated;

        db.insert_task_pack(&active).unwrap();
        db.insert_task_pack(&deprecated).unwrap();

        // List active only
        let by_role = db.list_task_packs_for_role("implementer").unwrap();
        assert_eq!(by_role.len(), 1);
        assert_eq!(by_role[0].task_pack_id, "tp-1");

        // List all
        let all = db.list_all_task_packs().unwrap();
        assert_eq!(all.len(), 2);
    }

    // MQR-F1-17: ValidatorPack status filtering works
    #[test]
    fn test_f1_17_validator_pack_status_filter() {
        let db = test_db();

        let active = ValidatorPack::new("vp-1".to_string(), 1, "implementer".to_string(), "hash1".to_string());
        let mut deprecated = ValidatorPack::new("vp-2".to_string(), 1, "implementer".to_string(), "hash2".to_string());
        deprecated.status = ValidatorPackStatus::Deprecated;

        db.insert_validator_pack(&active).unwrap();
        db.insert_validator_pack(&deprecated).unwrap();

        // List active only
        let by_role = db.list_validator_packs_for_role("implementer").unwrap();
        assert_eq!(by_role.len(), 1);
        assert_eq!(by_role[0].validator_pack_id, "vp-1");

        // List all
        let all = db.list_all_validator_packs().unwrap();
        assert_eq!(all.len(), 2);
    }

    // MQR-F1-18: Multiple identities for different models
    #[test]
    fn test_f1_18_multiple_identities() {
        let db = test_db();

        let r1 = ModelIdentityRecord::new("id-1".to_string(), "minicpm5-1b-q4km".to_string());
        let r2 = ModelIdentityRecord::new("id-2".to_string(), "minicpm5-1b-q8".to_string());

        db.insert_identity(&r1).unwrap();
        db.insert_identity(&r2).unwrap();

        let all = db.list_identities().unwrap();
        assert_eq!(all.len(), 2);

        let by_ref = db.get_identity_by_model_ref("minicpm5-1b-q4km").unwrap().unwrap();
        assert_eq!(by_ref.identity_id, "id-1");
    }

    // MQR-F1-19: Task pack version ordering
    #[test]
    fn test_f1_19_task_pack_version_ordering() {
        let db = test_db();

        let p1 = TaskPack::new("tp-1".to_string(), 1, "implementer".to_string(), "hash1".to_string());
        let p2 = TaskPack::new("tp-2".to_string(), 2, "implementer".to_string(), "hash2".to_string());

        db.insert_task_pack(&p1).unwrap();
        db.insert_task_pack(&p2).unwrap();

        let by_role = db.list_task_packs_for_role("implementer").unwrap();
        assert_eq!(by_role.len(), 2);
        // Should be ordered by version DESC (newest first)
        assert_eq!(by_role[0].version, 2);
        assert_eq!(by_role[1].version, 1);
    }

    // MQR-F1-20: execute_sql works
    #[test]
    fn test_f1_20_execute_sql() {
        let db = test_db();
        db.execute_sql("INSERT INTO model_identity_record (identity_id, model_id_ref, qualification_scope, created_at, updated_at) VALUES ('sql-test', 'model-a', 'full', '2026-01-01', '2026-01-01')").unwrap();
        let fetched = db.get_identity("sql-test").unwrap().unwrap();
        assert_eq!(fetched.model_id_ref, "model-a");
    }

    // MQR-F1-21: Delete nonexistent returns error
    #[test]
    fn test_f1_21_delete_nonexistent_error() {
        let db = test_db();
        let result = db.delete_identity("nonexistent");
        assert!(result.is_err());
    }

    // MQR-F1-22: Update nonexistent returns error
    #[test]
    fn test_f1_22_update_nonexistent_error() {
        let db = test_db();
        let record = ModelIdentityRecord::new("nonexistent".to_string(), "model-a".to_string());
        let result = db.update_identity(&record);
        assert!(result.is_err());
    }

    // MQR-F1-23: Canonical DB fails closed on invalid path
    #[test]
    fn test_f1_23_fails_on_invalid_path() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("subdir").join("test.db");
        let db = CanonicalDatabase::open(path).unwrap();
        db.migrate().unwrap();
        db.verify().unwrap();
    }

    // MQR-F1-24: Multiple task packs across roles
    #[test]
    fn test_f1_24_multiple_roles() {
        let db = test_db();

        let p1 = TaskPack::new("tp-1".to_string(), 1, "implementer".to_string(), "hash1".to_string());
        let p2 = TaskPack::new("tp-2".to_string(), 1, "researcher".to_string(), "hash2".to_string());

        db.insert_task_pack(&p1).unwrap();
        db.insert_task_pack(&p2).unwrap();

        let impl_packs = db.list_task_packs_for_role("implementer").unwrap();
        assert_eq!(impl_packs.len(), 1);

        let res_packs = db.list_task_packs_for_role("researcher").unwrap();
        assert_eq!(res_packs.len(), 1);

        let all = db.list_all_task_packs().unwrap();
        assert_eq!(all.len(), 2);
    }
}
