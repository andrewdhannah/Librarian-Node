//! # Migration Definitions
//!
//! All governance database migrations. Add new migrations at the end.
//! Never renumber or modify existing migrations — additive changes only.

use super::framework::Migration;

/// Returns all defined migrations in order.
pub fn all_migrations() -> Vec<Migration> {
    vec![
        migration_001_initial_governance(),
    ]
}

/// Migration 001: Initial governance schema.
///
/// Creates the core governance tables: lifecycle cursors, custody events,
/// evidence records, receipts, and receipt parent linkages.
///
/// This is the initial schema that all WO-001 through WO-006 work targeted.
pub fn migration_001_initial_governance() -> Migration {
    Migration {
        id: 1,
        description: "Create initial governance schema (lifecycle cursors, custody, evidence, receipts)",
        up_sql: indoc::indoc! {"
            CREATE TABLE IF NOT EXISTS lifecycle_cursors (
                project_id TEXT PRIMARY KEY,
                current_state TEXT NOT NULL,
                cycle INTEGER NOT NULL DEFAULT 1,
                cursor_position INTEGER NOT NULL DEFAULT 0,
                last_transition_at TEXT NOT NULL,
                schema_version TEXT NOT NULL DEFAULT '1.1.0',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS custody_events (
                event_id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                node_id TEXT NOT NULL,
                document_reference TEXT NOT NULL,
                custody_action TEXT NOT NULL,
                previous_mode TEXT,
                resulting_mode TEXT,
                timestamp TEXT NOT NULL,
                schema_version TEXT NOT NULL DEFAULT '1.0.0'
            );

            CREATE TABLE IF NOT EXISTS evidence_records (
                record_id TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                description TEXT NOT NULL,
                payload TEXT NOT NULL,
                payload_hash TEXT NOT NULL,
                recorded_at TEXT NOT NULL,
                produced_by TEXT NOT NULL,
                schema_version TEXT NOT NULL DEFAULT '1.0.0'
            );

            CREATE TABLE IF NOT EXISTS receipts (
                receipt_id TEXT PRIMARY KEY,
                receipt_type TEXT NOT NULL,
                receipt_version TEXT NOT NULL,
                action TEXT NOT NULL,
                initiated_by TEXT NOT NULL,
                authorized_by TEXT,
                summary TEXT NOT NULL,
                recorded_at TEXT NOT NULL,
                schema_version TEXT NOT NULL DEFAULT '1.0.0'
            );

            CREATE TABLE IF NOT EXISTS receipt_parents (
                receipt_id TEXT NOT NULL,
                parent_receipt_id TEXT NOT NULL,
                PRIMARY KEY (receipt_id, parent_receipt_id),
                FOREIGN KEY (receipt_id) REFERENCES receipts(receipt_id)
            );
        "},
        down_sql: Some(indoc::indoc! {"
            DROP TABLE IF EXISTS receipt_parents;
            DROP TABLE IF EXISTS receipts;
            DROP TABLE IF EXISTS evidence_records;
            DROP TABLE IF EXISTS custody_events;
            DROP TABLE IF EXISTS lifecycle_cursors;
        "}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_has_description() {
        let m = migration_001_initial_governance();
        assert!(!m.description.is_empty());
        assert_eq!(m.id, 1);
    }

    #[test]
    fn test_all_migrations_are_sequential() {
        let migrations = all_migrations();
        for (i, m) in migrations.iter().enumerate() {
            assert_eq!(m.id as usize, i + 1, "Migration {} is out of sequence", m.id);
        }
    }

    #[test]
    fn test_all_migrations_have_up_sql() {
        for m in all_migrations() {
            assert!(!m.up_sql.is_empty(), "Migration {} has no up SQL", m.id);
        }
    }
}
