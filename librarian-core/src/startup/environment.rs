//! Phase 4 — environment validation.
//!
//! Opens/creates the SQLite database, applies the canonical capability-registry
//! schema, and verifies integrity and table coverage.

use std::path::PathBuf;

use rusqlite::Connection;

/// Tables required by the canonical schema (9 Phase-2 + 2 Phase-3).
pub const EXPECTED_TABLES: [&str; 11] = [
    "capabilities",
    "capability_versions",
    "capability_dependencies",
    "capability_types",
    "qualification_profiles",
    "capability_qualifications",
    "policies",
    "policy_bindings",
    "capability_registry_meta",
    "qualification_lifecycle_events",
    "qualification_evidence_records",
];

/// Database context supplied by the adapter — the engine opens the path and
/// applies the provided schema; it makes no assumption about where the path
/// comes from.
#[derive(Debug, Clone)]
pub struct DatabaseContext {
    /// SQLite database file path.
    pub path: PathBuf,
    /// Canonical schema SQL to apply when the database is empty.
    pub schema_sql: String,
}

/// Verify the runtime environment: SQLite openable, canonical schema applied,
/// integrity ok, all 11 required tables present.
///
/// Returns `(passed, detail)` — environment failures are recorded in the
/// startup receipt, not returned as errors.
pub fn verify(db: &DatabaseContext) -> (bool, String) {
    if let Some(parent) = db.path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return (
                    false,
                    format!(
                        "cannot create database directory '{}': {e}",
                        parent.display()
                    ),
                );
            }
        }
    }

    let conn = match Connection::open(&db.path) {
        Ok(conn) => conn,
        Err(e) => {
            return (
                false,
                format!("cannot open database '{}': {e}", db.path.display()),
            );
        }
    };

    // Apply the canonical schema only to a fresh database (idempotent anyway).
    let table_count: i64 = match conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        [],
        |row| row.get(0),
    ) {
        Ok(n) => n,
        Err(e) => return (false, format!("cannot inspect database: {e}")),
    };
    if table_count == 0 {
        if let Err(e) = conn.execute_batch(&db.schema_sql) {
            return (false, format!("canonical schema application failed: {e}"));
        }
    }

    // Required table coverage.
    let tables: Vec<String> = {
        let mut stmt = match conn.prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        ) {
            Ok(stmt) => stmt,
            Err(e) => return (false, format!("cannot list tables: {e}")),
        };
        let rows = stmt.query_map([], |row| row.get::<_, String>(0));
        match rows {
            Ok(rows) => rows.filter_map(Result::ok).collect(),
            Err(e) => return (false, format!("cannot list tables: {e}")),
        }
    };
    let missing: Vec<&str> = EXPECTED_TABLES
        .iter()
        .copied()
        .filter(|name| !tables.iter().any(|t| t == name))
        .collect();
    if !missing.is_empty() {
        return (
            false,
            format!("missing canonical tables: {}", missing.join(", ")),
        );
    }

    // Integrity check.
    let integrity: String = match conn.query_row("PRAGMA integrity_check", [], |row| row.get(0)) {
        Ok(value) => value,
        Err(e) => return (false, format!("integrity check failed to run: {e}")),
    };
    if integrity != "ok" {
        return (false, format!("integrity_check returned '{integrity}'"));
    }

    (
        true,
        format!(
            "environment validated: '{}' ready, {} tables, integrity ok",
            db.path.display(),
            tables.len()
        ),
    )
}
