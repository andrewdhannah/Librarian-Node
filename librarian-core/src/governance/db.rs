//! # Governance Database
//!
//! SQLite-backed canonical state persistence for governance primitives.
//! Uses `rusqlite` with bundled SQLite for zero-dependency deployment.
//!
//! Stores:
//! - Lifecycle cursors (per-project state position)
//! - Custody events (check-out/check-in records)
//! - Evidence records (append-only)
//! - Receipts (append-only governance spine)

use anyhow::{Context, Result};
use librarian_contracts::prelude::*;
use rusqlite::{params, Connection};

/// The governance database. Wraps a SQLite connection with governance-specific operations.
#[derive(Clone)]
pub struct GovernanceDb {
    path: std::path::PathBuf,
}

impl GovernanceDb {
    /// Open or create the governance database at the given path.
    pub fn open(path: impl Into<std::path::PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create governance db directory: {:?}", parent))?;
        }
        let db = Self { path: path.clone() };
        let conn = db.connection()?;
        db.migrate(&conn)?;
        Ok(db)
    }

    /// Open an in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self> {
        let db = Self {
            path: ":memory:".into(),
        };
        let conn = db.connection()?;
        db.migrate(&conn)?;
        Ok(db)
    }

    /// Get a new connection with standard PRAGMAs.
    fn connection(&self) -> Result<Connection> {
        let conn = Connection::open(&self.path)
            .with_context(|| format!("Failed to open governance database at {:?}", self.path))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;"
        )?;
        Ok(conn)
    }

    /// Run migrations to ensure schema is current.
    fn migrate(&self, conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS lifecycle_cursors (
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
            );"
        )?;
        Ok(())
    }

    /// Path to the database file.
    pub fn path(&self) -> &std::path::Path {
        self.path.as_path()
    }
}

// ============================================================================
// Lifecycle Cursor Operations
// ============================================================================

impl GovernanceDb {
    /// Save a lifecycle cursor to the database.
    pub fn save_cursor(&self, cursor: &LifecycleCursor) -> Result<()> {
        let conn = self.connection()?;
        conn.execute(
            "INSERT INTO lifecycle_cursors (project_id, current_state, cycle, cursor_position,
             last_transition_at, schema_version, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(project_id) DO UPDATE SET
             current_state = excluded.current_state,
             cycle = excluded.cycle,
             cursor_position = excluded.cursor_position,
             last_transition_at = excluded.last_transition_at,
             schema_version = excluded.schema_version,
             updated_at = excluded.updated_at",
            params![
                cursor.project_id,
                serde_json::to_string(&cursor.current_state)?,
                cursor.cycle,
                cursor.cursor_position,
                cursor.last_transition_at,
                cursor.schema_version,
                cursor.last_transition_at,
                cursor.last_transition_at,
            ],
        )?;
        Ok(())
    }

    /// Load a lifecycle cursor for a project.
    pub fn load_cursor(&self, project_id: &str) -> Result<Option<LifecycleCursor>> {
        let conn = self.connection()?;
        let mut stmt = conn.prepare(
            "SELECT project_id, current_state, cycle, cursor_position, last_transition_at,
                    schema_version FROM lifecycle_cursors WHERE project_id = ?1"
        )?;
        let mut rows = stmt.query(params![project_id])?;
        if let Some(row) = rows.next()? {
            let state_str: String = row.get(1)?;
            let state: LifecycleState = serde_json::from_str(&state_str)?;
            Ok(Some(LifecycleCursor {
                project_id: row.get(0)?,
                current_state: state,
                cycle: row.get(2)?,
                cursor_position: row.get(3)?,
                last_transition_at: row.get(4)?,
                last_reconciled_at: None,
                reason: None,
                schema_version: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Delete a lifecycle cursor.
    pub fn delete_cursor(&self, project_id: &str) -> Result<bool> {
        let conn = self.connection()?;
        let affected = conn.execute(
            "DELETE FROM lifecycle_cursors WHERE project_id = ?1",
            params![project_id],
        )?;
        Ok(affected > 0)
    }
}

// ============================================================================
// Custody Event Operations
// ============================================================================

impl GovernanceDb {
    /// Record a custody event.
    pub fn record_custody_event(&self, event: &CustodyEvent) -> Result<()> {
        let conn = self.connection()?;
        conn.execute(
            "INSERT INTO custody_events (event_id, project_id, node_id, document_reference,
             custody_action, previous_mode, resulting_mode, timestamp, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                event.event_id,
                event.project_id,
                event.node_id,
                event.document_reference,
                serde_json::to_string(&event.custody_action)?,
                event.previous_custody_mode.map(|m| serde_json::to_string(&m).unwrap()),
                event.resulting_custody_mode.map(|m| serde_json::to_string(&m).unwrap()),
                event.timestamp,
                CUSTODY_CONTRACT_VERSION,
            ],
        )?;
        Ok(())
    }

    /// Get custody events for a document reference.
    pub fn get_custody_events(&self, document_reference: &str) -> Result<Vec<CustodyEvent>> {
        let conn = self.connection()?;
        let mut stmt = conn.prepare(
            "SELECT event_id, project_id, node_id, document_reference, custody_action,
                    previous_mode, resulting_mode, timestamp, schema_version
             FROM custody_events WHERE document_reference = ?1 ORDER BY timestamp"
        )?;
        let rows = stmt.query_map(params![document_reference], |row| {
            Ok(CustodyEvent {
                event_id: row.get(0)?,
                project_id: row.get(1)?,
                node_id: row.get(2)?,
                document_reference: row.get(3)?,
                custody_action: serde_json::from_str(&row.get::<_, String>(4)?).unwrap(),
                previous_custody_mode: row.get::<_, Option<String>>(5)?.map(|s| serde_json::from_str(&s).unwrap()),
                resulting_custody_mode: row.get::<_, Option<String>>(6)?.map(|s| serde_json::from_str(&s).unwrap()),
                timestamp: row.get(7)?,
                mcp_session_id: String::new(),
                tool_name: String::new(),
                authority_role: CustodyAuthorityRole::System,
                window_id: None,
                work_packet_id: None,
                mutation_allowance: None,
                decision_reference: None,
                provenance_receipt: None,
                refusal_reason: None,
                target_project_id: None,
                target_session_id: None,
                target_node_id: None,
            })
        })?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }
}

// ============================================================================
// Evidence Operations
// ============================================================================

impl GovernanceDb {
    /// Store an evidence record.
    pub fn store_evidence(&self, record: &EvidenceRecord) -> Result<()> {
        let conn = self.connection()?;
        conn.execute(
            "INSERT INTO evidence_records (record_id, category, description, payload,
             payload_hash, recorded_at, produced_by, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.record_id,
                serde_json::to_string(&record.category)?,
                record.description,
                record.payload.to_string(),
                record.payload_hash,
                record.recorded_at,
                record.produced_by,
                record.schema_version,
            ],
        )?;
        Ok(())
    }

    /// List evidence records by category.
    pub fn list_evidence(&self, category: &EvidenceCategory) -> Result<Vec<EvidenceRecord>> {
        let conn = self.connection()?;
        let cat_str = serde_json::to_string(category)?;
        let mut stmt = conn.prepare(
            "SELECT record_id, category, description, payload, payload_hash,
                    recorded_at, produced_by, schema_version
             FROM evidence_records WHERE category = ?1 ORDER BY recorded_at DESC"
        )?;
        let rows = stmt.query_map(params![cat_str], |row| {
            Ok(EvidenceRecord {
                record_id: row.get(0)?,
                category: serde_json::from_str(&row.get::<_, String>(1)?).unwrap(),
                description: row.get(2)?,
                payload: serde_json::from_str(&row.get::<_, String>(3)?).unwrap(),
                payload_hash: row.get(4)?,
                recorded_at: row.get(5)?,
                produced_by: row.get(6)?,
                schema_version: row.get(7)?,
            })
        })?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }
}

// ============================================================================
// Receipt Operations
// ============================================================================

impl GovernanceDb {
    /// Store a receipt.
    pub fn store_receipt(&self, receipt: &Receipt) -> Result<()> {
        let conn = self.connection()?;
        conn.execute(
            "INSERT INTO receipts (receipt_id, receipt_type, receipt_version, action,
             initiated_by, authorized_by, summary, recorded_at, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                receipt.receipt_id,
                serde_json::to_string(&receipt.receipt_type)?,
                receipt.receipt_version,
                receipt.action,
                receipt.initiated_by,
                receipt.authorized_by,
                receipt.summary,
                receipt.recorded_at,
                receipt.schema_version,
            ],
        )?;

        // Store parent references
        for parent_id in &receipt.parent_receipt_ids {
            conn.execute(
                "INSERT OR IGNORE INTO receipt_parents (receipt_id, parent_receipt_id)
                 VALUES (?1, ?2)",
                params![receipt.receipt_id, parent_id],
            )?;
        }
        Ok(())
    }

    /// Get a receipt by ID.
    pub fn get_receipt(&self, receipt_id: &str) -> Result<Option<Receipt>> {
        let conn = self.connection()?;
        let mut stmt = conn.prepare(
            "SELECT receipt_id, receipt_type, receipt_version, action, initiated_by,
                    authorized_by, summary, recorded_at, schema_version
             FROM receipts WHERE receipt_id = ?1"
        )?;
        let mut rows = stmt.query(params![receipt_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Receipt {
                receipt_id: row.get(0)?,
                receipt_type: serde_json::from_str(&row.get::<_, String>(1)?).unwrap(),
                receipt_version: row.get(2)?,
                action: row.get(3)?,
                initiated_by: row.get(4)?,
                authorized_by: row.get(5)?,
                summary: row.get(6)?,
                recorded_at: row.get(7)?,
                parent_receipt_ids: vec![],
                evidence_ids: vec![],
                project_id: None,
                schema_version: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_in_memory() {
        let db = GovernanceDb::open_in_memory().unwrap();
        assert!(db.path().to_string_lossy().contains(":memory:"));
    }

    #[test]
    fn test_save_and_load_cursor() {
        let db = GovernanceDb::open_in_memory().unwrap();
        let cursor = LifecycleCursor {
            project_id: "test-project".into(),
            current_state: LifecycleState::Operational,
            cycle: 1,
            cursor_position: 42,
            last_transition_at: "2026-07-23T00:00:00Z".into(),
            last_reconciled_at: None,
            reason: None,
            schema_version: LIFECYCLE_CONTRACT_VERSION.into(),
        };
        db.save_cursor(&cursor).unwrap();
        let loaded = db.load_cursor("test-project").unwrap().unwrap();
        assert_eq!(loaded.project_id, "test-project");
        assert_eq!(loaded.current_state, LifecycleState::Operational);
        assert_eq!(loaded.cursor_position, 42);
    }

    #[test]
    fn test_custody_event_persistence() {
        let db = GovernanceDb::open_in_memory().unwrap();
        let event = CustodyEvent {
            event_id: "ce-test-001".into(),
            project_id: "test".into(),
            mcp_session_id: "session-1".into(),
            node_id: "node-1".into(),
            window_id: None,
            work_packet_id: None,
            tool_name: "test".into(),
            authority_role: CustodyAuthorityRole::System,
            document_reference: "doc://test".into(),
            custody_action: CustodyAction::Claim,
            previous_custody_mode: None,
            resulting_custody_mode: Some(CustodyMode::LocalCanonical),
            mutation_allowance: None,
            decision_reference: None,
            provenance_receipt: None,
            refusal_reason: None,
            target_project_id: None,
            target_session_id: None,
            target_node_id: None,
            timestamp: "2026-07-23T00:00:00Z".into(),
        };
        db.record_custody_event(&event).unwrap();
        let events = db.get_custody_events("doc://test").unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, "ce-test-001");
    }

    #[test]
    fn test_evidence_store_and_list() {
        let db = GovernanceDb::open_in_memory().unwrap();
        let record = EvidenceRecord {
            record_id: "ev-test-001".into(),
            category: EvidenceCategory::ContractValidation,
            description: "Test evidence".into(),
            payload: serde_json::json!({"result": "pass"}),
            payload_hash: "abc123".into(),
            recorded_at: "2026-07-23T00:00:00Z".into(),
            produced_by: "WO-004".into(),
            schema_version: EVIDENCE_CONTRACT_VERSION.into(),
        };
        db.store_evidence(&record).unwrap();
        let records = db.list_evidence(&EvidenceCategory::ContractValidation).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].record_id, "ev-test-001");
    }

    #[test]
    fn test_receipt_with_parents() {
        let db = GovernanceDb::open_in_memory().unwrap();
        let parent = Receipt {
            receipt_id: "parent-001".into(),
            receipt_type: ReceiptType::SprintAuthorization,
            receipt_version: "1.0".into(),
            action: "authorize".into(),
            initiated_by: "owner".into(),
            authorized_by: Some("owner".into()),
            summary: "Parent receipt".into(),
            recorded_at: "2026-07-23T00:00:00Z".into(),
            parent_receipt_ids: vec![],
            evidence_ids: vec![],
            project_id: None,
            schema_version: RECEIPT_CONTRACT_VERSION.into(),
        };
        let child = Receipt {
            receipt_id: "child-001".into(),
            receipt_type: ReceiptType::SprintSeal,
            receipt_version: "1.0".into(),
            action: "seal".into(),
            initiated_by: "agent".into(),
            authorized_by: Some("owner".into()),
            summary: "Child receipt".into(),
            recorded_at: "2026-07-23T00:00:00Z".into(),
            parent_receipt_ids: vec!["parent-001".into()],
            evidence_ids: vec![],
            project_id: None,
            schema_version: RECEIPT_CONTRACT_VERSION.into(),
        };
        db.store_receipt(&parent).unwrap();
        db.store_receipt(&child).unwrap();
        let loaded = db.get_receipt("child-001").unwrap().unwrap();
        assert_eq!(loaded.receipt_id, "child-001");
        assert_eq!(loaded.receipt_type, ReceiptType::SprintSeal);
    }
}
