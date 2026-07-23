//! # Permissions
//!
//! Capability access mapping — which entities may access which capabilities,
//! backed by recorded decisions. Permissions reference authority; they do
//! not create it.
//!
//! PERMISSIONS-001 adds knowledge of who may perform what action.
//! It does NOT add authentication, enforcement, or policy evaluation.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Permission lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionStatus {
    Active,
    Suspended,
    Revoked,
}

/// A permission record — entity may access capability, backed by a decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRecord {
    /// Unique permission identifier.
    pub permission_id: String,
    /// Entity ID that holds this permission.
    pub entity_id: String,
    /// Capability identifier.
    pub capability_id: String,
    /// Decision ID that authorized this permission.
    pub decision_id: String,
    /// Current lifecycle status.
    pub status: PermissionStatus,
    /// Scope of the permission (* = all).
    pub scope: String,
    /// ISO 8601 timestamp.
    pub granted_at: String,
    /// ISO 8601 expiration (optional).
    pub expires_at: Option<String>,
}

/// The permissions manager.
pub struct PermissionManager {
    db: crate::governance::db::GovernanceDb,
}

impl PermissionManager {
    /// Create a new permission manager.
    pub fn new(db: crate::governance::db::GovernanceDb) -> Self {
        Self { db }
    }

    /// Grant a permission.
    pub fn grant(&self, permission: &PermissionRecord) -> Result<PermissionRecord> {
        let conn = self.db.connection()?;

        conn.execute(
            "INSERT INTO permissions (permission_id, entity_id, capability_id, decision_id,
             status, scope, granted_at, expires_at, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, '1.0.0')",
            rusqlite::params![
                permission.permission_id,
                permission.entity_id,
                permission.capability_id,
                permission.decision_id,
                serde_json::to_string(&permission.status)?,
                permission.scope,
                permission.granted_at,
                permission.expires_at,
            ],
        )?;

        // Generate evidence
        let evidence_payload = serde_json::json!({
            "action": "permission_granted",
            "permission_id": permission.permission_id,
            "entity_id": permission.entity_id,
            "capability_id": permission.capability_id,
            "decision_id": permission.decision_id,
            "status": format!("{:?}", permission.status),
        });
        let now = chrono::Utc::now().to_rfc3339();
        let evidence_id = format!("evt-perm-{}", Uuid::new_v4());

        conn.execute(
            "INSERT INTO evidence_records (record_id, category, description, payload,
             payload_hash, recorded_at, produced_by, schema_version)
             VALUES (?1, 'contract_validation', ?2, ?3, ?4, ?5, 'permission-manager', '1.0.0')",
            rusqlite::params![
                evidence_id,
                format!("Permission granted: {} → {} ({:?})",
                    permission.entity_id, permission.capability_id, permission.status),
                evidence_payload.to_string(),
                "permission-evidence",
                now,
            ],
        )?;

        Ok(permission.clone())
    }

    /// Check if an entity has permission for a capability.
    pub fn check(&self, entity_id: &str, capability_id: &str) -> Result<bool> {
        let conn = self.db.connection()?;
        let count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM permissions
             WHERE entity_id = ?1 AND capability_id = ?2 AND status = 'active'
             AND (expires_at IS NULL OR expires_at > datetime('now'))",
            rusqlite::params![entity_id, capability_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Get a permission record.
    pub fn get(&self, permission_id: &str) -> Result<Option<PermissionRecord>> {
        let conn = self.db.connection()?;
        let mut stmt = conn.prepare(
            "SELECT permission_id, entity_id, capability_id, decision_id, status, scope,
                    granted_at, expires_at
             FROM permissions WHERE permission_id = ?1"
        )?;
        let mut rows = stmt.query(rusqlite::params![permission_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(PermissionRecord {
                permission_id: row.get(0)?,
                entity_id: row.get(1)?,
                capability_id: row.get(2)?,
                decision_id: row.get(3)?,
                status: serde_json::from_str(&row.get::<_, String>(4)?)?,
                scope: row.get(5)?,
                granted_at: row.get(6)?,
                expires_at: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Revoke a permission.
    pub fn revoke(&self, permission_id: &str) -> Result<bool> {
        let conn = self.db.connection()?;
        let status_str = serde_json::to_string(&PermissionStatus::Revoked)?;
        let now = chrono::Utc::now().to_rfc3339();
        let affected = conn.execute(
            "UPDATE permissions SET status = ?1, expires_at = ?2 WHERE permission_id = ?3",
            rusqlite::params![status_str, now, permission_id],
        )?;
        Ok(affected > 0)
    }

    /// Suspend a permission.
    pub fn suspend(&self, permission_id: &str) -> Result<bool> {
        let conn = self.db.connection()?;
        let status_str = serde_json::to_string(&PermissionStatus::Suspended)?;
        let affected = conn.execute(
            "UPDATE permissions SET status = ?1 WHERE permission_id = ?2",
            rusqlite::params![status_str, permission_id],
        )?;
        Ok(affected > 0)
    }

    /// List permissions for an entity.
    pub fn list_by_entity(&self, entity_id: &str) -> Result<Vec<PermissionRecord>> {
        let conn = self.db.connection()?;
        let mut stmt = conn.prepare(
            "SELECT permission_id, entity_id, capability_id, decision_id, status, scope,
                    granted_at, expires_at
             FROM permissions WHERE entity_id = ?1 ORDER BY granted_at DESC"
        )?;
        let rows = stmt.query_map(rusqlite::params![entity_id], |row| {
            Ok(PermissionRecord {
                permission_id: row.get(0)?,
                entity_id: row.get(1)?,
                capability_id: row.get(2)?,
                decision_id: row.get(3)?,
                status: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or(PermissionStatus::Active),
                scope: row.get(5)?,
                granted_at: row.get(6)?,
                expires_at: row.get(7)?,
            })
        })?;
        let mut permissions = Vec::new();
        for row in rows {
            permissions.push(row?);
        }
        Ok(permissions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::db::GovernanceDb;

    fn setup() -> PermissionManager {
        let db = GovernanceDb::open_in_memory().unwrap();
        PermissionManager::new(db)
    }

    fn sample_permission() -> PermissionRecord {
        PermissionRecord {
            permission_id: "PERM-001".into(),
            entity_id: "node-windows-01".into(),
            capability_id: "model-phi-4".into(),
            decision_id: "DEC-001".into(),
            status: PermissionStatus::Active,
            scope: "*".into(),
            granted_at: "2026-07-23T00:00:00Z".into(),
            expires_at: None,
        }
    }

    #[test]
    fn test_grant_and_check() {
        let mgr = setup();
        mgr.grant(&sample_permission()).unwrap();

        let has_perm = mgr.check("node-windows-01", "model-phi-4").unwrap();
        assert!(has_perm);

        let no_perm = mgr.check("node-windows-01", "model-other").unwrap();
        assert!(!no_perm);
    }

    #[test]
    fn test_get_permission() {
        let mgr = setup();
        mgr.grant(&sample_permission()).unwrap();

        let loaded = mgr.get("PERM-001").unwrap().unwrap();
        assert_eq!(loaded.entity_id, "node-windows-01");
        assert_eq!(loaded.capability_id, "model-phi-4");
        assert_eq!(loaded.decision_id, "DEC-001");
    }

    #[test]
    fn test_revoke_permission() {
        let mgr = setup();
        mgr.grant(&sample_permission()).unwrap();

        assert!(mgr.check("node-windows-01", "model-phi-4").unwrap());

        mgr.revoke("PERM-001").unwrap();
        assert!(!mgr.check("node-windows-01", "model-phi-4").unwrap());
    }

    #[test]
    fn test_suspend_permission() {
        let mgr = setup();
        mgr.grant(&sample_permission()).unwrap();
        mgr.suspend("PERM-001").unwrap();

        let loaded = mgr.get("PERM-001").unwrap().unwrap();
        assert_eq!(loaded.status, PermissionStatus::Suspended);
        assert!(!mgr.check("node-windows-01", "model-phi-4").unwrap());
    }

    #[test]
    fn test_list_by_entity() {
        let mgr = setup();
        mgr.grant(&sample_permission()).unwrap();
        mgr.grant(&PermissionRecord {
            permission_id: "PERM-002".into(),
            entity_id: "node-windows-01".into(),
            capability_id: "model-qwen".into(),
            decision_id: "DEC-002".into(),
            status: PermissionStatus::Active,
            scope: "*".into(),
            granted_at: "2026-07-23T00:00:00Z".into(),
            expires_at: None,
        }).unwrap();

        let list = mgr.list_by_entity("node-windows-01").unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_permission_with_expiry() {
        let mgr = setup();
        let past = "2020-01-01T00:00:00Z";
        mgr.grant(&PermissionRecord {
            permission_id: "PERM-EXP".into(),
            entity_id: "node-expired".into(),
            capability_id: "model-test".into(),
            decision_id: "DEC-003".into(),
            status: PermissionStatus::Active,
            scope: "*".into(),
            granted_at: "2020-01-01T00:00:00Z".into(),
            expires_at: Some(past.into()),
        }).unwrap();

        let has_perm = mgr.check("node-expired", "model-test").unwrap();
        assert!(!has_perm);
    }
}
