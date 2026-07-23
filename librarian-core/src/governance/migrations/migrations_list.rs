//! # Migration Definitions
//!
//! All governance database migrations. Add new migrations at the end.
//! Never renumber or modify existing migrations — additive changes only.

use super::framework::Migration;

/// Returns all defined migrations in order.
pub fn all_migrations() -> Vec<Migration> {
    vec![
        migration_001_initial_governance(),
        migration_002_entity_registry(),
        migration_003_decision_records(),
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

/// Migration 002: Entity registry.
///
/// Creates the entities table for persistent actor, node, capability,
/// and resource tracking. This is the first step toward multi-actor
/// governance — it establishes what exists without defining authority.
pub fn migration_002_entity_registry() -> Migration {
    Migration {
        id: 2,
        description: "Create entity registry (actors, nodes, capabilities, resources)",
        up_sql: indoc::indoc! {"
            CREATE TABLE IF NOT EXISTS entities (
                entity_id TEXT PRIMARY KEY,
                entity_type TEXT NOT NULL CHECK (entity_type IN (
                    'human', 'agent', 'node', 'capability', 'resource', 'organization'
                )),
                display_name TEXT NOT NULL,
                external_id TEXT,
                parent_entity_id TEXT REFERENCES entities(entity_id),
                status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended', 'retired')),
                metadata TEXT DEFAULT '{}',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                registered_by TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);
            CREATE INDEX IF NOT EXISTS idx_entities_parent ON entities(parent_entity_id);
        "},
        down_sql: Some(indoc::indoc! {"
            DROP INDEX IF EXISTS idx_entities_parent;
            DROP INDEX IF EXISTS idx_entities_type;
            DROP TABLE IF EXISTS entities;
        "}),
    }
}

/// Migration 003: Decision records.
///
/// Creates the decisions table for durable owner authority records.
/// This is the first migration that stores human authority intent:
/// what was approved, by whom, and under what context.
///
/// Decision records link to:
/// - Entity (who authorized or was the subject)
/// - Evidence (what supported the decision)
/// - Receipts (what recorded the decision)
pub fn migration_003_decision_records() -> Migration {
    Migration {
        id: 3,
        description: "Create decision records (owner authority, approvals, authorizations)",
        up_sql: indoc::indoc! {"
            CREATE TABLE IF NOT EXISTS decisions (
                decision_id TEXT PRIMARY KEY,
                decision_type TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'approved', 'rejected', 'deferred', 'superseded')),
                summary TEXT NOT NULL,
                rationale TEXT,
                entity_id TEXT NOT NULL REFERENCES entities(entity_id),
                target_entity_id TEXT REFERENCES entities(entity_id),
                evidence_id TEXT,
                receipt_id TEXT,
                created_at TEXT NOT NULL,
                decided_at TEXT,
                decided_by TEXT,
                superseded_by TEXT REFERENCES decisions(decision_id),
                schema_version TEXT NOT NULL DEFAULT '1.0.0'
            );

            CREATE INDEX IF NOT EXISTS idx_decisions_entity ON decisions(entity_id);
            CREATE INDEX IF NOT EXISTS idx_decisions_status ON decisions(status);
            CREATE INDEX IF NOT EXISTS idx_decisions_target ON decisions(target_entity_id);
        "},
        down_sql: Some(indoc::indoc! {"
            DROP INDEX IF EXISTS idx_decisions_target;
            DROP INDEX IF EXISTS idx_decisions_status;
            DROP INDEX IF EXISTS idx_decisions_entity;
            DROP TABLE IF EXISTS decisions;
        "}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_001_has_description() {
        let m = migration_001_initial_governance();
        assert!(!m.description.is_empty());
        assert_eq!(m.id, 1);
    }

    #[test]
    fn test_migration_002_has_description() {
        let m = migration_002_entity_registry();
        assert!(!m.description.is_empty());
        assert_eq!(m.id, 2);
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

    #[test]
    fn test_migration_002_has_correct_types() {
        let m = migration_002_entity_registry();
        assert!(m.up_sql.contains("'human'"));
        assert!(m.up_sql.contains("'agent'"));
        assert!(m.up_sql.contains("'node'"));
        assert!(m.up_sql.contains("'capability'"));
        assert!(m.up_sql.contains("'resource'"));
        assert!(m.up_sql.contains("'organization'"));
    }

    #[test]
    fn test_migration_002_has_status_checks() {
        let m = migration_002_entity_registry();
        assert!(m.up_sql.contains("'active'"));
        assert!(m.up_sql.contains("'suspended'"));
        assert!(m.up_sql.contains("'retired'"));
    }

    #[test]
    fn test_migration_003_has_description() {
        let m = migration_003_decision_records();
        assert!(!m.description.is_empty());
        assert_eq!(m.id, 3);
    }

    #[test]
    fn test_migration_003_has_decision_statuses() {
        let m = migration_003_decision_records();
        assert!(m.up_sql.contains("'pending'"));
        assert!(m.up_sql.contains("'approved'"));
        assert!(m.up_sql.contains("'rejected'"));
        assert!(m.up_sql.contains("'deferred'"));
        assert!(m.up_sql.contains("'superseded'"));
    }

    #[test]
    fn test_migration_003_has_entity_reference() {
        let m = migration_003_decision_records();
        assert!(m.up_sql.contains("entity_id"));
        assert!(m.up_sql.contains("target_entity_id"));
    }
}
