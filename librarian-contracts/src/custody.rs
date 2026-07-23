//! # Custody Contract Types
//!
//! Custody envelope and operation types for multi-node document custody.
//! Maps to Swift `MCPCustodyEvent`, `MCPCustodyMode`, custody models.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Schema version for custody contracts.
pub const CUSTODY_CONTRACT_VERSION: &str = "1.0.0";

/// Custody mode — the custody status of a document or artifact.
/// Maps to Swift `MCPCustodyMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CustodyMode {
    /// Owned by the owner — highest authority.
    OwnerHeld,
    /// Local canonical copy.
    LocalCanonical,
    /// Local working copy (mutated by agent).
    LocalWorkingCopy,
    /// Delegated to a worker node for execution.
    DelegatedWorker,
    /// Delegated read-only access.
    DelegatedReadOnly,
    /// Mirrored read-only copy.
    MirroredReadOnly,
    /// Transfer pending acceptance.
    TransferPending,
    /// Transfer accepted by target.
    TransferAccepted,
    /// External reference (not locally stored).
    ExternalReference,
    /// Advisory context only — no custody authority.
    AdvisoryContextOnly,
}

impl CustodyMode {
    /// All known custody modes.
    pub const ALL: &'static [CustodyMode] = &[
        CustodyMode::OwnerHeld,
        CustodyMode::LocalCanonical,
        CustodyMode::LocalWorkingCopy,
        CustodyMode::DelegatedWorker,
        CustodyMode::DelegatedReadOnly,
        CustodyMode::MirroredReadOnly,
        CustodyMode::TransferPending,
        CustodyMode::TransferAccepted,
        CustodyMode::ExternalReference,
        CustodyMode::AdvisoryContextOnly,
    ];

    /// Whether this mode permits mutation.
    pub fn allows_mutation(&self) -> bool {
        matches!(
            self,
            CustodyMode::OwnerHeld
                | CustodyMode::LocalCanonical
                | CustodyMode::LocalWorkingCopy
                | CustodyMode::DelegatedWorker
        )
    }
}

impl fmt::Display for CustodyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();
        write!(f, "{}", s)
    }
}

/// Custody action type.
/// Maps to Swift `MCPCustodyAction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustodyAction {
    /// Read access.
    Read,
    /// Claim custody.
    Claim,
    /// Transfer custody to another node.
    Transfer,
    /// Release custody.
    Release,
    /// Validate custody claim.
    Validate,
    /// Refuse custody action.
    Refuse,
}

/// Authority role in a custody event.
/// Maps to Swift `MCPCustodyAuthorityRole`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustodyAuthorityRole {
    /// Platform owner.
    Owner,
    /// AI model.
    Model,
    /// Agent.
    Agent,
    /// Node.
    Node,
    /// System process.
    System,
    /// Advisory-only role.
    Advisory,
}

/// Mutation allowance level.
/// Maps to Swift `MCPCustodyMutationAllowance`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MutationAllowance {
    /// No mutation allowed.
    None,
    /// Read-only access.
    ReadOnly,
    /// Commentary only.
    CommentaryOnly,
    /// Derived artifact creation only.
    DerivedArtifactOnly,
    /// Working copy mutation only.
    WorkingCopyOnly,
    /// Canonical mutation pending owner approval.
    CanonicalMutationPendingOwner,
    /// Canonical mutation approved.
    CanonicalMutationApproved,
}

/// A custody event record.
/// Maps to Swift `MCPCustodyEvent`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyEvent {
    /// Unique event identifier.
    pub event_id: String,
    /// Project this event belongs to.
    pub project_id: String,
    /// MCP session that issued this event.
    pub mcp_session_id: String,
    /// Source node ID.
    pub node_id: String,
    /// Window/context identifier.
    pub window_id: Option<String>,
    /// Work packet ID.
    pub work_packet_id: Option<String>,
    /// MCP tool name that triggered this action.
    pub tool_name: String,
    /// Authority role at time of event.
    pub authority_role: CustodyAuthorityRole,
    /// Reference to the affected document/packet/receipt.
    pub document_reference: String,
    /// Custody action performed.
    pub custody_action: CustodyAction,
    /// Previous custody mode (before action).
    pub previous_custody_mode: Option<CustodyMode>,
    /// Resulting custody mode (after action).
    pub resulting_custody_mode: Option<CustodyMode>,
    /// Mutation allowance after action.
    pub mutation_allowance: Option<MutationAllowance>,
    /// Reference to the owner decision authorizing this event.
    pub decision_reference: Option<String>,
    /// Reference to the provenance receipt.
    pub provenance_receipt: Option<String>,
    /// Reason if custody_action is Refuse.
    pub refusal_reason: Option<String>,
    /// Target project ID (for transfers).
    pub target_project_id: Option<String>,
    /// Target session ID (for transfers).
    pub target_session_id: Option<String>,
    /// Target node ID (for transfers).
    pub target_node_id: Option<String>,
    /// ISO 8601 timestamp.
    pub timestamp: String,
}

/// Custody status for a document reference.
/// Maps to Swift `MCPCustodyStatus`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyStatus {
    /// Document reference.
    pub document_reference: String,
    /// Project ID.
    pub project_id: String,
    /// Whether the custody claim is currently valid.
    pub custody_claim_valid: bool,
    /// Number of active events.
    pub active_event_count: u32,
    /// The latest custody event.
    pub latest_event: Option<CustodyEvent>,
    /// Refusal reason (if any).
    pub refusal_reason: Option<String>,
    /// Cross-context issues detected.
    pub cross_context_issues: Vec<String>,
}

/// A custody envelope — wraps a document reference with its custody metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyEnvelope {
    /// The document or artifact reference.
    pub document_reference: String,
    /// Current custody mode.
    pub mode: CustodyMode,
    /// Node holding custody.
    pub held_by: String,
    /// When custody was acquired.
    pub acquired_at: String,
    /// When custody expires (if applicable).
    pub expires_at: Option<String>,
    /// The event that established this custody.
    pub establishing_event_id: String,
    /// SHA-256 of the document content (for integrity).
    pub content_hash: String,
    /// Schema version.
    pub schema_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custody_mode_allows_mutation() {
        assert!(CustodyMode::OwnerHeld.allows_mutation());
        assert!(CustodyMode::LocalCanonical.allows_mutation());
        assert!(!CustodyMode::MirroredReadOnly.allows_mutation());
        assert!(!CustodyMode::AdvisoryContextOnly.allows_mutation());
    }

    #[test]
    fn test_custody_event_serde() {
        let event = CustodyEvent {
            event_id: "ce-001".into(),
            project_id: "librarian".into(),
            mcp_session_id: "mcp-session-001".into(),
            node_id: "node-001".into(),
            window_id: None,
            work_packet_id: Some("wp-001".into()),
            tool_name: "custody_claim".into(),
            authority_role: CustodyAuthorityRole::Owner,
            document_reference: "doc://project-state/sprint-ledger.json".into(),
            custody_action: CustodyAction::Claim,
            previous_custody_mode: None,
            resulting_custody_mode: Some(CustodyMode::LocalCanonical),
            mutation_allowance: Some(MutationAllowance::WorkingCopyOnly),
            decision_reference: Some("AR-001".into()),
            provenance_receipt: None,
            refusal_reason: None,
            target_project_id: None,
            target_session_id: None,
            target_node_id: None,
            timestamp: "2026-07-23T00:00:00Z".into(),
        };
        let json = serde_json::to_string_pretty(&event).unwrap();
        let deserialized: CustodyEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event.event_id, deserialized.event_id);
        assert_eq!(event.custody_action, deserialized.custody_action);
    }

    #[test]
    fn test_custody_envelope() {
        let envelope = CustodyEnvelope {
            document_reference: "doc://contracts/ROUTER-HTTP.md".into(),
            mode: CustodyMode::OwnerHeld,
            held_by: "andrewdhannah".into(),
            acquired_at: "2026-07-23T00:00:00Z".into(),
            expires_at: None,
            establishing_event_id: "ce-001".into(),
            content_hash: "abc123def456".into(),
            schema_version: CUSTODY_CONTRACT_VERSION.into(),
        };
        let json = serde_json::to_string_pretty(&envelope).unwrap();
        assert!(json.contains("doc://contracts/ROUTER-HTTP.md"));
        assert!(json.contains("OWNER_HELD"));
    }

    #[test]
    fn test_custody_mode_all() {
        assert_eq!(CustodyMode::ALL.len(), 10);
    }
}
