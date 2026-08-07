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

/// Custody metadata attached to a receipt envelope.
/// Maps to the metadata block on Swift custody chain entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyMetadata {
    /// Source of the receipt (e.g., `"node"`).
    pub source: String,
    /// Metadata schema version.
    pub version: String,
    /// Optional free-form notes.
    pub notes: Option<String>,
}

/// A receipt envelope — an immutable, hash-chained evidence record.
/// Each envelope links to its predecessor via `previous_envelope_id` and
/// `previous_envelope_hash`; `chain_hash` covers the cumulative chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptEnvelope {
    /// Unique envelope identifier.
    pub envelope_id: String,
    /// Node that produced the receipt.
    pub node_id: String,
    /// Receipt type (e.g., `"identity"`, `"workload_allocation_link"`).
    pub receipt_type: String,
    /// Original receipt identifier.
    pub receipt_id: String,
    /// The receipt payload itself.
    pub receipt_payload: serde_json::Value,
    /// SHA-256 of the serialized payload.
    pub receipt_hash: String,
    /// Envelope ID of the predecessor in the chain, if any.
    pub previous_envelope_id: Option<String>,
    /// Chain hash of the predecessor, if any.
    pub previous_envelope_hash: Option<String>,
    /// SHA-256 covering this envelope's receipt hash (and predecessor chain hash when present).
    pub chain_hash: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Optional custody metadata.
    pub metadata: Option<CustodyMetadata>,
}

/// A custody chain — aggregate state of a hash-linked envelope sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyChain {
    /// Unique chain identifier.
    pub chain_id: String,
    /// Node that owns the chain.
    pub node_id: String,
    /// When the chain was created (ISO 8601).
    pub created_at: String,
    /// Number of envelopes in the chain.
    pub envelope_count: u32,
    /// Envelope ID of the first envelope.
    pub first_envelope_id: String,
    /// Envelope ID of the last envelope.
    pub last_envelope_id: String,
    /// Chain hash of the last envelope.
    pub last_chain_hash: String,
    /// Chain status (e.g., `"active"`).
    pub status: String,
}

/// Provenance query — filter parameters for custody envelope lookups.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceQuery {
    /// Optional node ID filter.
    pub node_id: Option<String>,
    /// Optional receipt type filter.
    pub receipt_type: Option<String>,
    /// Inclusive lower timestamp bound.
    pub from_timestamp: Option<String>,
    /// Inclusive upper timestamp bound.
    pub to_timestamp: Option<String>,
}

/// A single provenance relationship between envelopes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceLink {
    /// Predecessor envelope ID.
    pub from_envelope_id: String,
    /// Successor envelope ID.
    pub to_envelope_id: String,
    /// Relationship label (e.g., `"precedes"`).
    pub relationship: String,
}

/// Provenance result — an envelope plus a human-readable summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceResult {
    /// The matched envelope.
    pub envelope: ReceiptEnvelope,
    /// Receipt type of the matched envelope.
    pub receipt_type: String,
    /// Summary string (`<receipt_type>:<receipt_id>`).
    pub receipt_summary: String,
}

/// Provenance graph — full custody history for a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceGraph {
    /// Node that owns the graph.
    pub node_id: String,
    /// All envelopes in the chain.
    pub envelopes: Vec<ReceiptEnvelope>,
    /// Precedes-relationships between envelopes.
    pub relationships: Vec<ProvenanceLink>,
}

/// Integrity error — a single detected violation in a custody chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityError {
    /// Envelope ID where the violation was detected.
    pub envelope_id: String,
    /// Error type (`tampered_payload`, `broken_chain`, `missing_previous`, `hash_mismatch`).
    pub error_type: String,
    /// Human-readable details.
    pub details: String,
}

/// Integrity report — result of verifying a custody chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    /// Chain that was verified.
    pub chain_id: String,
    /// Node that owns the chain.
    pub node_id: String,
    /// Whether the chain is intact.
    pub verified: bool,
    /// Number of envelopes recorded in the chain state.
    pub envelope_count: u32,
    /// Number of envelopes actually checked.
    pub envelopes_checked: u32,
    /// Detected violations (empty when verified).
    pub errors: Vec<IntegrityError>,
    /// ISO 8601 verification timestamp.
    pub verified_at: String,
}

/// Retention policy — bounds for pruning custody envelopes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Unique policy identifier.
    pub policy_id: String,
    /// Maximum number of envelopes to retain.
    pub max_envelopes: Option<u32>,
    /// Retain only envelopes newer than this many days.
    pub retention_days: Option<u32>,
}

/// Retention result — outcome of applying a retention policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionResult {
    /// Policy that was applied.
    pub policy_id: String,
    /// Envelopes before pruning.
    pub envelopes_before: u32,
    /// Envelopes after pruning.
    pub envelopes_after: u32,
    /// Envelopes archived (always 0 in current implementation).
    pub archived: u32,
    /// Envelopes deleted.
    pub deleted: u32,
    /// ISO 8601 application timestamp.
    pub applied_at: String,
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
