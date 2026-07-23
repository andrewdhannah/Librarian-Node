//! Routing log — audit trail for every routing decision.
//!
//! Every time the router selects (or fails to select) a projection for
//! a work packet, a routing_log entry is created. This provides a
//! complete audit trail of routing decisions.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Routing decision status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoutingStatus {
    /// A projection was selected for this packet.
    #[serde(rename = "selected")]
    Selected,
    /// No active projection matched the requested role.
    #[serde(rename = "no_projection")]
    NoProjection,
    /// Multiple projections matched but hardware constraints eliminated all.
    #[serde(rename = "rejected_by_constraints")]
    RejectedByConstraints,
    /// Multiple projections matched with no way to disambiguate.
    #[serde(rename = "ambiguous_role")]
    AmbiguousRole,
    /// The packet was rejected before routing (malformed, etc.).
    #[serde(rename = "packet_rejected")]
    PacketRejected,
}

impl RoutingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Selected => "selected",
            Self::NoProjection => "no_projection",
            Self::RejectedByConstraints => "rejected_by_constraints",
            Self::AmbiguousRole => "ambiguous_role",
            Self::PacketRejected => "packet_rejected",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "selected" => Some(Self::Selected),
            "no_projection" => Some(Self::NoProjection),
            "rejected_by_constraints" => Some(Self::RejectedByConstraints),
            "ambiguous_role" => Some(Self::AmbiguousRole),
            "packet_rejected" => Some(Self::PacketRejected),
            _ => None,
        }
    }
}

/// Routing log entry — records a single routing decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoutingLogEntry {
    /// Unique log entry identifier.
    pub log_id: String,

    /// The work packet identifier that triggered this routing decision.
    pub packet_id: String,

    /// The role requested by the work packet.
    pub role: String,

    /// The projection that was selected (if any).
    pub projection_id: Option<String>,

    /// The model that was selected (if any).
    pub model_id: Option<String>,

    /// The profile that was selected (if any).
    pub profile_id: Option<String>,

    /// The routing decision status.
    pub status: RoutingStatus,

    /// Human-readable explanation of the routing decision.
    pub reason: String,

    /// When the routing decision was made (RFC 3339).
    pub created_at: String,

    /// SHA-256 hash of the log entry content.
    pub content_hash: String,
}

impl RoutingLogEntry {
    /// Compute a deterministic log ID from packet_id and timestamp.
    pub fn compute_log_id(packet_id: &str, created_at: &str) -> String {
        let input = format!("{}:{}", packet_id, created_at);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compute SHA-256 hash of the log entry content.
    pub fn compute_content_hash(&self) -> Result<String> {
        let content = serde_json::json!({
            "packet_id": self.packet_id,
            "role": self.role,
            "projection_id": self.projection_id,
            "model_id": self.model_id,
            "profile_id": self.profile_id,
            "status": self.status.as_str(),
            "reason": self.reason,
        });
        let json = content.to_string();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Validate the log entry structure.
    pub fn validate(&self) -> Result<()> {
        if self.log_id.is_empty() {
            anyhow::bail!("log_id is empty");
        }
        if self.packet_id.is_empty() {
            anyhow::bail!("packet_id is empty");
        }
        if self.role.is_empty() {
            anyhow::bail!("role is empty");
        }
        if self.reason.is_empty() {
            anyhow::bail!("reason is empty");
        }
        Ok(())
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).context("Failed to serialize log entry to JSON")
    }

    /// Parse from JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to parse log entry from JSON")
    }
}

/// Create a routing log entry for a successful selection.
pub fn log_selected(
    packet_id: &str,
    role: &str,
    projection_id: &str,
    model_id: &str,
    profile_id: &str,
    reason: &str,
) -> Result<RoutingLogEntry> {
    let created_at = chrono::Utc::now().to_rfc3339();
    let log_id = RoutingLogEntry::compute_log_id(packet_id, &created_at);

    let mut entry = RoutingLogEntry {
        log_id,
        packet_id: packet_id.to_string(),
        role: role.to_string(),
        projection_id: Some(projection_id.to_string()),
        model_id: Some(model_id.to_string()),
        profile_id: Some(profile_id.to_string()),
        status: RoutingStatus::Selected,
        reason: reason.to_string(),
        created_at,
        content_hash: String::new(),
    };

    entry.content_hash = entry.compute_content_hash()?;
    Ok(entry)
}

/// Create a routing log entry for a rejected or failed selection.
///
/// The `status` parameter determines the routing status recorded in the
/// log entry (e.g., `NoProjection` when no role match exists, or
/// `RejectedByConstraints` when hardware constraints eliminated all
/// candidates).
pub fn log_rejected(
    packet_id: &str,
    role: &str,
    reason: &str,
    status: RoutingStatus,
) -> Result<RoutingLogEntry> {
    let created_at = chrono::Utc::now().to_rfc3339();
    let log_id = RoutingLogEntry::compute_log_id(packet_id, &created_at);

    let mut entry = RoutingLogEntry {
        log_id,
        packet_id: packet_id.to_string(),
        role: role.to_string(),
        projection_id: None,
        model_id: None,
        profile_id: None,
        status,
        reason: reason.to_string(),
        created_at,
        content_hash: String::new(),
    };

    entry.content_hash = entry.compute_content_hash()?;
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    // R2-L1: Log entry validates
    #[test]
    fn test_log_entry_validates() {
        let entry = log_selected("pkt-001", "classifier", "proj-001", "model-1", "prof-001", "exact match").unwrap();
        assert!(entry.validate().is_ok());
    }

    // R2-L2: Log ID is deterministic
    #[test]
    fn test_log_id_deterministic() {
        let id1 = RoutingLogEntry::compute_log_id("pkt-001", "2026-07-11T12:00:00Z");
        let id2 = RoutingLogEntry::compute_log_id("pkt-001", "2026-07-11T12:00:00Z");
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 64);
    }

    // R2-L3: Log ID depends on inputs
    #[test]
    fn test_log_id_depends_on_inputs() {
        let id1 = RoutingLogEntry::compute_log_id("pkt-001", "2026-07-11T12:00:00Z");
        let id2 = RoutingLogEntry::compute_log_id("pkt-002", "2026-07-11T12:00:00Z");
        assert_ne!(id1, id2);
    }

    // R2-L4: Status string round-trip
    #[test]
    fn test_status_string_roundtrip() {
        let statuses = vec![
            RoutingStatus::Selected,
            RoutingStatus::NoProjection,
            RoutingStatus::RejectedByConstraints,
            RoutingStatus::AmbiguousRole,
            RoutingStatus::PacketRejected,
        ];
        for status in &statuses {
            let s = status.as_str();
            assert!(!s.is_empty());
            assert_eq!(RoutingStatus::from_str(s), Some(status.clone()));
        }
    }

    // R2-L5: Serialization round-trip
    #[test]
    fn test_serialization_round_trip() {
        let entry = log_selected("pkt-001", "classifier", "proj-001", "model-1", "prof-001", "exact match").unwrap();
        let json = entry.to_json().unwrap();
        let parsed = RoutingLogEntry::from_json(&json).unwrap();
        assert_eq!(entry, parsed);
    }

    // R2-L6: log_selected creates correct entry
    #[test]
    fn test_log_selected_creates_entry() {
        let entry = log_selected("pkt-001", "classifier", "proj-001", "model-1", "prof-001", "selected by role").unwrap();
        assert_eq!(entry.packet_id, "pkt-001");
        assert_eq!(entry.role, "classifier");
        assert_eq!(entry.projection_id, Some("proj-001".to_string()));
        assert_eq!(entry.model_id, Some("model-1".to_string()));
        assert_eq!(entry.profile_id, Some("prof-001".to_string()));
        assert_eq!(entry.status, RoutingStatus::Selected);
    }

    // R2-L7: log_rejected creates correct entry
    #[test]
    fn test_log_rejected_creates_entry() {
        let entry = log_rejected("pkt-002", "summarizer", "no active projection for role", RoutingStatus::NoProjection).unwrap();
        assert_eq!(entry.packet_id, "pkt-002");
        assert_eq!(entry.role, "summarizer");
        assert_eq!(entry.projection_id, None);
        assert_eq!(entry.model_id, None);
        assert_eq!(entry.profile_id, None);
        assert_eq!(entry.status, RoutingStatus::NoProjection);
    }

    // R2-L8: Validate fails on empty packet_id
    #[test]
    fn test_validate_empty_packet_id() {
        let mut entry = log_selected("pkt-001", "classifier", "proj-001", "model-1", "prof-001", "ok").unwrap();
        entry.packet_id = "".to_string();
        assert!(entry.validate().is_err());
    }

    // R2-L9: Validate fails on empty role
    #[test]
    fn test_validate_empty_role() {
        let mut entry = log_selected("pkt-001", "classifier", "proj-001", "model-1", "prof-001", "ok").unwrap();
        entry.role = "".to_string();
        assert!(entry.validate().is_err());
    }

    // R2-L10: Validate fails on empty reason
    #[test]
    fn test_validate_empty_reason() {
        let mut entry = log_selected("pkt-001", "classifier", "proj-001", "model-1", "prof-001", "ok").unwrap();
        entry.reason = "".to_string();
        assert!(entry.validate().is_err());
    }

    // R2-L11: Content hash is computed
    #[test]
    fn test_content_hash_computed() {
        let entry = log_selected("pkt-001", "classifier", "proj-001", "model-1", "prof-001", "ok").unwrap();
        assert!(!entry.content_hash.is_empty());
        assert_eq!(entry.content_hash.len(), 64);
    }

    // R2-L12: Content hash is deterministic
    #[test]
    fn test_content_hash_deterministic() {
        let entry1 = RoutingLogEntry {
            log_id: "test".to_string(),
            packet_id: "pkt-001".to_string(),
            role: "classifier".to_string(),
            projection_id: Some("proj-001".to_string()),
            model_id: Some("model-1".to_string()),
            profile_id: Some("prof-001".to_string()),
            status: RoutingStatus::Selected,
            reason: "ok".to_string(),
            created_at: "2026-07-11T12:00:00Z".to_string(),
            content_hash: String::new(),
        };
        let entry2 = entry1.clone();
        assert_eq!(entry1.compute_content_hash().unwrap(), entry2.compute_content_hash().unwrap());
    }

    // R2-L13: created_at is set
    #[test]
    fn test_created_at_set() {
        let entry = log_selected("pkt-001", "classifier", "proj-001", "model-1", "prof-001", "ok").unwrap();
        assert!(!entry.created_at.is_empty());
        assert!(entry.created_at.contains("T"));
    }
}
