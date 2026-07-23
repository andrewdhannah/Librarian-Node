//! # Decision Records
//!
//! Persistent owner authority records — what was approved, by whom,
//! and under what context. This is the first migration that stores
//! human authority intent in a machine-readable, durable form.
//!
//! DECISIONS-001 adds knowledge of what was approved.
//! It does NOT add permissions, authentication, or enforcement.

use anyhow::Result;
use librarian_contracts::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Decision lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    Pending,
    Approved,
    Rejected,
    Deferred,
    Superseded,
}

/// A persistent decision record — what was approved, by whom, and why.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    /// Unique decision identifier.
    pub decision_id: String,
    /// Type of decision (e.g., capability_authorization, sprint_approval, access_grant).
    pub decision_type: String,
    /// Current lifecycle status.
    pub status: DecisionStatus,
    /// Human-readable summary.
    pub summary: String,
    /// Rationale for the decision.
    pub rationale: Option<String>,
    /// Entity ID of the subject or requester.
    pub entity_id: String,
    /// Entity ID of the target (capability, resource, node).
    pub target_entity_id: Option<String>,
    /// Reference to supporting evidence.
    pub evidence_id: Option<String>,
    /// Reference to the receipt that recorded this decision.
    pub receipt_id: Option<String>,
    /// ISO 8601 timestamp of creation.
    pub created_at: String,
    /// ISO 8601 timestamp of decision.
    pub decided_at: Option<String>,
    /// Who made the decision.
    pub decided_by: Option<String>,
    /// If superseded, the decision that replaced this one.
    pub superseded_by: Option<String>,
}

/// The decision records manager.
pub struct DecisionManager {
    db: crate::governance::db::GovernanceDb,
}

impl DecisionManager {
    /// Create a new decision manager.
    pub fn new(db: crate::governance::db::GovernanceDb) -> Self {
        Self { db }
    }

    /// Record a new decision.
    pub fn record(&self, decision: &DecisionRecord) -> Result<DecisionRecord> {
        let conn = self.db.connection()?;

        conn.execute(
            "INSERT INTO decisions (decision_id, decision_type, status, summary, rationale,
             entity_id, target_entity_id, evidence_id, receipt_id, created_at, decided_at,
             decided_by, superseded_by, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, '1.0.0')",
            rusqlite::params![
                decision.decision_id,
                decision.decision_type,
                serde_json::to_string(&decision.status)?,
                decision.summary,
                decision.rationale,
                decision.entity_id,
                decision.target_entity_id,
                decision.evidence_id,
                decision.receipt_id,
                decision.created_at,
                decision.decided_at,
                decision.decided_by,
                decision.superseded_by,
            ],
        )?;

        // Generate evidence
        let evidence_payload = serde_json::json!({
            "action": "decision_recorded",
            "decision_id": decision.decision_id,
            "decision_type": decision.decision_type,
            "status": format!("{:?}", decision.status),
            "entity_id": decision.entity_id,
            "summary": decision.summary,
        });
        let now = chrono::Utc::now().to_rfc3339();
        let evidence_id = format!("evt-decision-{}", Uuid::new_v4());

        conn.execute(
            "INSERT INTO evidence_records (record_id, category, description, payload,
             payload_hash, recorded_at, produced_by, schema_version)
             VALUES (?1, 'contract_validation', ?2, ?3, ?4, ?5, 'decision-manager', '1.0.0')",
            rusqlite::params![
                evidence_id,
                format!("Decision recorded: {} — {:?}", decision.summary, decision.status),
                evidence_payload.to_string(),
                "decision-evidence",
                now,
            ],
        )?;

        Ok(decision.clone())
    }

    /// Get a decision by ID.
    pub fn get(&self, decision_id: &str) -> Result<Option<DecisionRecord>> {
        let conn = self.db.connection()?;
        let mut stmt = conn.prepare(
            "SELECT decision_id, decision_type, status, summary, rationale,
                    entity_id, target_entity_id, evidence_id, receipt_id,
                    created_at, decided_at, decided_by, superseded_by
             FROM decisions WHERE decision_id = ?1"
        )?;
        let mut rows = stmt.query(rusqlite::params![decision_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(DecisionRecord {
                decision_id: row.get(0)?,
                decision_type: row.get(1)?,
                status: serde_json::from_str(&row.get::<_, String>(2)?)?,
                summary: row.get(3)?,
                rationale: row.get(4)?,
                entity_id: row.get(5)?,
                target_entity_id: row.get(6)?,
                evidence_id: row.get(7)?,
                receipt_id: row.get(8)?,
                created_at: row.get(9)?,
                decided_at: row.get(10)?,
                decided_by: row.get(11)?,
                superseded_by: row.get(12)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Update decision status.
    pub fn update_status(&self, decision_id: &str, status: DecisionStatus, decided_by: &str) -> Result<bool> {
        let conn = self.db.connection()?;
        let now = chrono::Utc::now().to_rfc3339();
        let status_str = serde_json::to_string(&status)?;
        let affected = conn.execute(
            "UPDATE decisions SET status = ?1, decided_at = ?2, decided_by = ?3 WHERE decision_id = ?4",
            rusqlite::params![status_str, now, decided_by, decision_id],
        )?;
        Ok(affected > 0)
    }

    /// List decisions for an entity.
    pub fn list_by_entity(&self, entity_id: &str) -> Result<Vec<DecisionRecord>> {
        let conn = self.db.connection()?;
        let mut stmt = conn.prepare(
            "SELECT decision_id, decision_type, status, summary, rationale,
                    entity_id, target_entity_id, evidence_id, receipt_id,
                    created_at, decided_at, decided_by, superseded_by
             FROM decisions WHERE entity_id = ?1 ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map(rusqlite::params![entity_id], Self::map_row)?;
        let mut decisions = Vec::new();
        for row in rows {
            decisions.push(row?);
        }
        Ok(decisions)
    }

    /// Helper to map a SQLite row to a DecisionRecord.
    fn map_row(row: &rusqlite::Row) -> rusqlite::Result<DecisionRecord> {
        Ok(DecisionRecord {
            decision_id: row.get(0)?,
            decision_type: row.get(1)?,
            status: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or(DecisionStatus::Pending),
            summary: row.get(3)?,
            rationale: row.get(4)?,
            entity_id: row.get(5)?,
            target_entity_id: row.get(6)?,
            evidence_id: row.get(7)?,
            receipt_id: row.get(8)?,
            created_at: row.get(9)?,
            decided_at: row.get(10)?,
            decided_by: row.get(11)?,
            superseded_by: row.get(12)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::db::GovernanceDb;

    fn setup() -> DecisionManager {
        let db = GovernanceDb::open_in_memory().unwrap();
        DecisionManager::new(db)
    }

    fn sample_decision() -> DecisionRecord {
        DecisionRecord {
            decision_id: "DEC-001".into(),
            decision_type: "capability_authorization".into(),
            status: DecisionStatus::Approved,
            summary: "Authorized phi-4 model execution on Windows node".into(),
            rationale: Some("Model passed qualification at 4096 context, ngl=99".into()),
            entity_id: "node-windows-01".into(),
            target_entity_id: Some("cap-model-phi4".into()),
            evidence_id: Some("evt-qual-phi4-001".into()),
            receipt_id: None,
            created_at: "2026-07-23T00:00:00Z".into(),
            decided_at: Some("2026-07-23T00:00:00Z".into()),
            decided_by: Some("andrew".into()),
            superseded_by: None,
        }
    }

    #[test]
    fn test_record_decision() {
        let mgr = setup();
        let decision = sample_decision();
        mgr.record(&decision).unwrap();

        let loaded = mgr.get("DEC-001").unwrap().unwrap();
        assert_eq!(loaded.decision_id, "DEC-001");
        assert_eq!(loaded.status, DecisionStatus::Approved);
        assert_eq!(loaded.entity_id, "node-windows-01");
    }

    #[test]
    fn test_update_status() {
        let mgr = setup();
        mgr.record(&sample_decision()).unwrap();

        let updated = mgr.update_status("DEC-001", DecisionStatus::Superseded, "owner").unwrap();
        assert!(updated);

        let loaded = mgr.get("DEC-001").unwrap().unwrap();
        assert_eq!(loaded.status, DecisionStatus::Superseded);
    }

    #[test]
    fn test_list_by_entity() {
        let mgr = setup();

        mgr.record(&DecisionRecord {
            decision_id: "DEC-001".into(),
            decision_type: "capability_authorization".into(),
            status: DecisionStatus::Approved,
            summary: "Authorized model phi-4".into(),
            rationale: None,
            entity_id: "node-windows-01".into(),
            target_entity_id: None,
            evidence_id: None,
            receipt_id: None,
            created_at: "2026-07-23T00:00:00Z".into(),
            decided_at: None,
            decided_by: Some("andrew".into()),
            superseded_by: None,
        }).unwrap();

        mgr.record(&DecisionRecord {
            decision_id: "DEC-002".into(),
            decision_type: "capability_authorization".into(),
            status: DecisionStatus::Approved,
            summary: "Authorized model qwen-coder".into(),
            rationale: None,
            entity_id: "node-windows-01".into(),
            target_entity_id: None,
            evidence_id: None,
            receipt_id: None,
            created_at: "2026-07-23T00:00:00Z".into(),
            decided_at: None,
            decided_by: Some("andrew".into()),
            superseded_by: None,
        }).unwrap();

        let decisions = mgr.list_by_entity("node-windows-01").unwrap();
        assert_eq!(decisions.len(), 2);
    }

    #[test]
    fn test_decision_status_values() {
        assert!(matches!(DecisionStatus::Pending, DecisionStatus::Pending));
        assert!(matches!(DecisionStatus::Approved, DecisionStatus::Approved));
        assert!(matches!(DecisionStatus::Rejected, DecisionStatus::Rejected));
        assert!(matches!(DecisionStatus::Deferred, DecisionStatus::Deferred));
        assert!(matches!(DecisionStatus::Superseded, DecisionStatus::Superseded));
    }

    #[test]
    fn test_decision_with_target_entity() {
        let mgr = setup();
        let decision = DecisionRecord {
            decision_id: "DEC-003".into(),
            decision_type: "access_grant".into(),
            status: DecisionStatus::Approved,
            summary: "Granted MCP access to capability".into(),
            rationale: None,
            entity_id: "user-andrew".into(),
            target_entity_id: Some("cap-model-phi4".into()),
            evidence_id: None,
            receipt_id: None,
            created_at: "2026-07-23T00:00:00Z".into(),
            decided_at: Some("2026-07-23T00:00:00Z".into()),
            decided_by: Some("owner".into()),
            superseded_by: None,
        };
        mgr.record(&decision).unwrap();
        let loaded = mgr.get("DEC-003").unwrap().unwrap();
        assert_eq!(loaded.target_entity_id, Some("cap-model-phi4".into()));
    }
}
