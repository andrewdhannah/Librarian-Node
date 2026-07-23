//! # Entity Registry
//!
//! Persistent entity storage for actors, nodes, capabilities, and resources
//! that participate in governed execution.
//!
//! ENTITY-001 adds knowledge of existence. It does NOT add authority.
//! Authentication, authorization, and permissions belong to later sprints.

use anyhow::Result;
use librarian_contracts::evidence::{EvidenceCategory, EvidenceRecord};
use librarian_contracts::prelude::*;
use librarian_contracts::serialization::hash_canonical;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Entity type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    /// A human user or operator.
    Human,
    /// An automated agent.
    Agent,
    /// A Librarian Node (runtime instance).
    Node,
    /// A governed capability.
    Capability,
    /// A resource (model, service, storage).
    Resource,
    /// An organization or group.
    Organization,
}

impl EntityType {
    pub const ALL: &'static [EntityType] = &[
        EntityType::Human,
        EntityType::Agent,
        EntityType::Node,
        EntityType::Capability,
        EntityType::Resource,
        EntityType::Organization,
    ];
}

/// Entity lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityStatus {
    Active,
    Suspended,
    Retired,
}

/// A persistent entity record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRecord {
    /// Unique entity identifier.
    pub entity_id: String,
    /// Entity type classification.
    pub entity_type: EntityType,
    /// Human-readable display name.
    pub display_name: String,
    /// Reference to an external identity (e.g., NodeId, provider subject).
    pub external_id: Option<String>,
    /// Parent entity ID (for ownership hierarchy).
    pub parent_entity_id: Option<String>,
    /// Lifecycle status.
    pub status: EntityStatus,
    /// JSON metadata.
    pub metadata: serde_json::Value,
    /// ISO 8601 timestamp.
    pub created_at: String,
    /// ISO 8601 timestamp.
    pub updated_at: String,
    /// Who registered this entity.
    pub registered_by: String,
}

/// The entity registry — persists actors, nodes, capabilities, resources.
pub struct EntityRegistry {
    db: crate::governance::db::GovernanceDb,
}

impl EntityRegistry {
    /// Create a new entity registry backed by the governance database.
    pub fn new(db: crate::governance::db::GovernanceDb) -> Self {
        Self { db }
    }

    /// Register a new entity in the governance database.
    /// Generates an evidence record for the registration.
    pub fn register(&self, entity: &EntityRecord) -> Result<EntityRecord> {
        let conn = self.db.connection()?;

        conn.execute(
            "INSERT INTO entities (entity_id, entity_type, display_name, external_id,
             parent_entity_id, status, metadata, created_at, updated_at, registered_by)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                entity.entity_id,
                serde_json::to_string(&entity.entity_type)?,
                entity.display_name,
                entity.external_id,
                entity.parent_entity_id,
                serde_json::to_string(&entity.status)?,
                entity.metadata.to_string(),
                entity.created_at,
                entity.updated_at,
                entity.registered_by,
            ],
        )?;

        // Generate evidence for the registration
        let evidence_payload = serde_json::json!({
            "action": "entity_registered",
            "entity_id": entity.entity_id,
            "entity_type": format!("{:?}", entity.entity_type),
            "display_name": entity.display_name,
            "registered_by": entity.registered_by,
        });
        let payload_hash = hash_canonical(&evidence_payload)?;
        let now = chrono::Utc::now().to_rfc3339();
        let evidence_id = format!("evt-entity-{}", Uuid::new_v4());

        conn.execute(
            "INSERT INTO evidence_records (record_id, category, description, payload,
             payload_hash, recorded_at, produced_by, schema_version)
             VALUES (?1, 'contract_validation', ?2, ?3, ?4, ?5, 'entity-registry', '1.0.0')",
            rusqlite::params![
                evidence_id,
                format!("Entity registered: {} ({:?})", entity.display_name, entity.entity_type),
                evidence_payload.to_string(),
                payload_hash,
                now,
            ],
        )?;

        Ok(entity.clone())
    }

    /// Get an entity by ID.
    pub fn get(&self, entity_id: &str) -> Result<Option<EntityRecord>> {
        let conn = self.db.connection()?;
        let mut stmt = conn.prepare(
            "SELECT entity_id, entity_type, display_name, external_id, parent_entity_id,
                    status, metadata, created_at, updated_at, registered_by
             FROM entities WHERE entity_id = ?1"
        )?;
        let mut rows = stmt.query(rusqlite::params![entity_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(EntityRecord {
                entity_id: row.get(0)?,
                entity_type: serde_json::from_str(&row.get::<_, String>(1)?)?,
                display_name: row.get(2)?,
                external_id: row.get(3)?,
                parent_entity_id: row.get(4)?,
                status: serde_json::from_str(&row.get::<_, String>(5)?)?,
                metadata: serde_json::from_str(&row.get::<_, String>(6)?)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                registered_by: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List entities by type.
    pub fn list_by_type(&self, entity_type: EntityType) -> Result<Vec<EntityRecord>> {
        let conn = self.db.connection()?;
        let type_str = serde_json::to_string(&entity_type)?;
        let mut stmt = conn.prepare(
            "SELECT entity_id, entity_type, display_name, external_id, parent_entity_id,
                    status, metadata, created_at, updated_at, registered_by
             FROM entities WHERE entity_type = ?1 ORDER BY created_at"
        )?;
        let rows = stmt.query_map(rusqlite::params![type_str], |row| {
            Ok(EntityRecord {
                entity_id: row.get(0)?,
                entity_type: serde_json::from_str(&row.get::<_, String>(1)?).unwrap(),
                display_name: row.get(2)?,
                external_id: row.get(3)?,
                parent_entity_id: row.get(4)?,
                status: serde_json::from_str(&row.get::<_, String>(5)?).unwrap(),
                metadata: serde_json::from_str(&row.get::<_, String>(6)?).unwrap(),
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                registered_by: row.get(9)?,
            })
        })?;
        let mut entities = Vec::new();
        for row in rows {
            entities.push(row?);
        }
        Ok(entities)
    }

    /// List all entities.
    pub fn list_all(&self) -> Result<Vec<EntityRecord>> {
        let conn = self.db.connection()?;
        let mut stmt = conn.prepare(
            "SELECT entity_id, entity_type, display_name, external_id, parent_entity_id,
                    status, metadata, created_at, updated_at, registered_by
             FROM entities ORDER BY created_at"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(EntityRecord {
                entity_id: row.get(0)?,
                entity_type: serde_json::from_str(&row.get::<_, String>(1)?).unwrap(),
                display_name: row.get(2)?,
                external_id: row.get(3)?,
                parent_entity_id: row.get(4)?,
                status: serde_json::from_str(&row.get::<_, String>(5)?).unwrap(),
                metadata: serde_json::from_str(&row.get::<_, String>(6)?).unwrap(),
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                registered_by: row.get(9)?,
            })
        })?;
        let mut entities = Vec::new();
        for row in rows {
            entities.push(row?);
        }
        Ok(entities)
    }

    /// Update entity status.
    pub fn update_status(&self, entity_id: &str, status: EntityStatus) -> Result<bool> {
        let conn = self.db.connection()?;
        let now = chrono::Utc::now().to_rfc3339();
        let status_str = serde_json::to_string(&status)?;
        let affected = conn.execute(
            "UPDATE entities SET status = ?1, updated_at = ?2 WHERE entity_id = ?3",
            rusqlite::params![status_str, now, entity_id],
        )?;
        Ok(affected > 0)
    }

    /// Count entities by type.
    pub fn count_by_type(&self, entity_type: EntityType) -> Result<u64> {
        let conn = self.db.connection()?;
        let type_str = serde_json::to_string(&entity_type)?;
        let count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM entities WHERE entity_type = ?1",
            rusqlite::params![type_str],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::db::GovernanceDb;

    fn setup() -> EntityRegistry {
        let db = GovernanceDb::open_in_memory().unwrap();
        EntityRegistry::new(db)
    }

    fn sample_node() -> EntityRecord {
        EntityRecord {
            entity_id: "node-windows-01".into(),
            entity_type: EntityType::Node,
            display_name: "Windows Runtime Node 1".into(),
            external_id: Some("nid-win-001".into()),
            parent_entity_id: None,
            status: EntityStatus::Active,
            metadata: serde_json::json!({"platform": "windows", "version": "0.1.0"}),
            created_at: "2026-07-23T00:00:00Z".into(),
            updated_at: "2026-07-23T00:00:00Z".into(),
            registered_by: "migration-runner".into(),
        }
    }

    #[test]
    fn test_register_and_get() {
        let registry = setup();
        let entity = sample_node();
        registry.register(&entity).unwrap();

        let loaded = registry.get("node-windows-01").unwrap().unwrap();
        assert_eq!(loaded.entity_id, "node-windows-01");
        assert_eq!(loaded.entity_type, EntityType::Node);
        assert_eq!(loaded.display_name, "Windows Runtime Node 1");
        assert_eq!(loaded.external_id, Some("nid-win-001".into()));
    }

    #[test]
    fn test_register_human() {
        let registry = setup();
        let entity = EntityRecord {
            entity_id: "user-andrew".into(),
            entity_type: EntityType::Human,
            display_name: "Andrew Hannah".into(),
            external_id: None,
            parent_entity_id: None,
            status: EntityStatus::Active,
            metadata: serde_json::json!({}),
            created_at: "2026-07-23T00:00:00Z".into(),
            updated_at: "2026-07-23T00:00:00Z".into(),
            registered_by: "system".into(),
        };
        registry.register(&entity).unwrap();
        let loaded = registry.get("user-andrew").unwrap().unwrap();
        assert_eq!(loaded.entity_type, EntityType::Human);
    }

    #[test]
    fn test_register_capability() {
        let registry = setup();
        let entity = EntityRecord {
            entity_id: "cap-model-phi4".into(),
            entity_type: EntityType::Capability,
            display_name: "Model: phi-4".into(),
            external_id: Some("cap-001".into()),
            parent_entity_id: Some("node-windows-01".into()),
            status: EntityStatus::Active,
            metadata: serde_json::json!({"category": "model_execution"}),
            created_at: "2026-07-23T00:00:00Z".into(),
            updated_at: "2026-07-23T00:00:00Z".into(),
            registered_by: "owner".into(),
        };
        registry.register(&entity).unwrap();

        // Verify parent-child relationship
        let loaded = registry.get("cap-model-phi4").unwrap().unwrap();
        assert_eq!(loaded.parent_entity_id, Some("node-windows-01".into()));
    }

    #[test]
    fn test_list_by_type() {
        let registry = setup();

        registry.register(&EntityRecord {
            entity_id: "node-1".into(),
            entity_type: EntityType::Node,
            display_name: "Node 1".into(),
            external_id: None,
            parent_entity_id: None,
            status: EntityStatus::Active,
            metadata: serde_json::json!({}),
            created_at: "2026-07-23T00:00:00Z".into(),
            updated_at: "2026-07-23T00:00:00Z".into(),
            registered_by: "system".into(),
        }).unwrap();

        registry.register(&EntityRecord {
            entity_id: "node-2".into(),
            entity_type: EntityType::Node,
            display_name: "Node 2".into(),
            external_id: None,
            parent_entity_id: None,
            status: EntityStatus::Active,
            metadata: serde_json::json!({}),
            created_at: "2026-07-23T00:00:00Z".into(),
            updated_at: "2026-07-23T00:00:00Z".into(),
            registered_by: "system".into(),
        }).unwrap();

        let nodes = registry.list_by_type(EntityType::Node).unwrap();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn test_update_status() {
        let registry = setup();
        let entity = sample_node();
        registry.register(&entity).unwrap();

        let updated = registry.update_status("node-windows-01", EntityStatus::Suspended).unwrap();
        assert!(updated);

        let loaded = registry.get("node-windows-01").unwrap().unwrap();
        assert_eq!(loaded.status, EntityStatus::Suspended);
    }

    #[test]
    fn test_count_by_type() {
        let registry = setup();
        registry.register(&sample_node()).unwrap();
        registry.register(&EntityRecord {
            entity_id: "user-andrew".into(),
            entity_type: EntityType::Human,
            display_name: "Andrew".into(),
            external_id: None,
            parent_entity_id: None,
            status: EntityStatus::Active,
            metadata: serde_json::json!({}),
            created_at: "2026-07-23T00:00:00Z".into(),
            updated_at: "2026-07-23T00:00:00Z".into(),
            registered_by: "system".into(),
        }).unwrap();

        assert_eq!(registry.count_by_type(EntityType::Node).unwrap(), 1);
        assert_eq!(registry.count_by_type(EntityType::Human).unwrap(), 1);
        assert_eq!(registry.count_by_type(EntityType::Capability).unwrap(), 0);
    }

    #[test]
    fn test_all_entity_types() {
        assert!(EntityType::ALL.contains(&EntityType::Human));
        assert!(EntityType::ALL.contains(&EntityType::Agent));
        assert!(EntityType::ALL.contains(&EntityType::Node));
        assert!(EntityType::ALL.contains(&EntityType::Capability));
        assert!(EntityType::ALL.contains(&EntityType::Resource));
        assert!(EntityType::ALL.contains(&EntityType::Organization));
        assert_eq!(EntityType::ALL.len(), 6);
    }
}
