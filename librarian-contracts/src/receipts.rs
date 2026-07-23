//! # Receipt Contract Types
//!
//! Governance receipt types for the Librarian platform.
//! Maps to Swift `DecisionResolutionReceipt`, `ProjectWorkReceipt`,
//! and related receipt models.
//!
//! Receipts are append-only. They form the governance spine.

use serde::{Deserialize, Serialize};

/// Schema version for receipt contracts.
pub const RECEIPT_CONTRACT_VERSION: &str = "1.0.0";

/// Type of governance receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptType {
    /// Sprint authorization receipt.
    SprintAuthorization,
    /// Decision resolution receipt.
    DecisionResolution,
    /// Sprint seal receipt.
    SprintSeal,
    /// Custody event receipt.
    CustodyEvent,
    /// Evidence receipt.
    Evidence,
    /// Migration equivalence receipt.
    Equivalence,
    /// Checkpoint receipt.
    Checkpoint,
    /// Owner action receipt.
    OwnerAction,
}

/// A governance receipt — the core audit record.
/// Maps to Swift `DecisionResolutionReceipt` and `ProjectWorkReceipt`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    /// Unique receipt identifier.
    pub receipt_id: String,
    /// Type of receipt.
    pub receipt_type: ReceiptType,
    /// Schema version of this receipt.
    pub receipt_version: String,
    /// ISO 8601 timestamp when recorded.
    pub recorded_at: String,
    /// What action this receipt records.
    pub action: String,
    /// Who or what initiated the action.
    pub initiated_by: String,
    /// Who or what authorized the action.
    pub authorized_by: Option<String>,
    /// Summary of the action.
    pub summary: String,
    /// References to related receipts (causal chain).
    pub parent_receipt_ids: Vec<String>,
    /// References to related evidence records.
    pub evidence_ids: Vec<String>,
    /// The project this receipt belongs to.
    pub project_id: Option<String>,
    /// Schema version.
    pub schema_version: String,
}

/// A reference to an existing receipt (for building causal chains).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptReference {
    /// Referenced receipt ID.
    pub receipt_id: String,
    /// Type of the referenced receipt.
    pub receipt_type: ReceiptType,
    /// Relationship to the current receipt.
    pub relationship: String,
}

/// A sprint authorization receipt.
/// Maps to the authorization receipt pattern used in WO-001.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SprintAuthorizationReceipt {
    /// Unique receipt ID.
    pub receipt_id: String,
    /// Receipt type (always sprint_authorization).
    pub receipt_type: ReceiptType,
    /// Work order / sprint ID being authorized.
    pub work_order_id: String,
    /// ISO 8601 timestamp.
    pub authorized_at: String,
    /// Who authorized.
    pub authorized_by: String,
    /// Target repository.
    pub repository: String,
    /// Sprint scope summary.
    pub scope: String,
    /// Capability expansion (if any).
    pub capability_expansion: Option<String>,
    /// Migration code flag.
    pub migration_code: bool,
    /// Schema version.
    pub schema_version: String,
}

/// A migration equivalence receipt.
/// Records that an equivalence check was performed and its result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquivalenceReceipt {
    /// Unique receipt ID.
    pub receipt_id: String,
    /// The equivalence run ID.
    pub equivalence_run_id: String,
    /// Baseline implementation ID.
    pub baseline_id: String,
    /// Candidate implementation ID.
    pub candidate_id: String,
    /// Overall result (PASS, FAIL, PASS_WITH_DEVIATIONS).
    pub result: String,
    /// Number of checks passed.
    pub checks_passed: u32,
    /// Number of checks failed.
    pub checks_failed: u32,
    /// ISO 8601 timestamp.
    pub completed_at: String,
    /// Path to the evidence packet.
    pub evidence_path: String,
    /// Schema version.
    pub schema_version: String,
}

/// A receipt summary (for list responses).
/// Maps to Swift bounded query result patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptSummary {
    pub receipt_id: String,
    pub receipt_type: ReceiptType,
    pub action: String,
    pub recorded_at: String,
    pub summary: String,
    pub has_parents: bool,
    pub has_evidence: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receipt_serde() {
        let receipt = Receipt {
            receipt_id: "receipt-001".into(),
            receipt_type: ReceiptType::SprintAuthorization,
            receipt_version: "1.0".into(),
            recorded_at: "2026-07-23T00:00:00Z".into(),
            action: "authorize_sprint".into(),
            initiated_by: "andrewdhannah".into(),
            authorized_by: Some("andrewdhannah".into()),
            summary: "Authorized WO-001".into(),
            parent_receipt_ids: vec![],
            evidence_ids: vec![],
            project_id: Some("librarian".into()),
            schema_version: RECEIPT_CONTRACT_VERSION.into(),
        };
        let json = serde_json::to_string_pretty(&receipt).unwrap();
        let deserialized: Receipt = serde_json::from_str(&json).unwrap();
        assert_eq!(receipt.receipt_id, deserialized.receipt_id);
        assert_eq!(receipt.receipt_type, deserialized.receipt_type);
    }

    #[test]
    fn test_sprint_authorization_receipt() {
        let r = SprintAuthorizationReceipt {
            receipt_id: "AR-WO-001-20260723".into(),
            receipt_type: ReceiptType::SprintAuthorization,
            work_order_id: "WO-001-EQ-CHECKS-AND-TEMPLATES-1".into(),
            authorized_at: "2026-07-23T00:00:00Z".into(),
            authorized_by: "Andrew Hannah".into(),
            repository: "Librarian-Platform-Equivalence".into(),
            scope: "Equivalence framework foundation".into(),
            capability_expansion: None,
            migration_code: false,
            schema_version: RECEIPT_CONTRACT_VERSION.into(),
        };
        let json = serde_json::to_string_pretty(&r).unwrap();
        assert!(json.contains("AR-WO-001-20260723"));
        assert!(json.contains("sprint_authorization"));
    }

    #[test]
    fn test_equivalence_receipt() {
        let r = EquivalenceReceipt {
            receipt_id: "EQ-RUN-001".into(),
            equivalence_run_id: "eq-run-20260723-001".into(),
            baseline_id: "swift-core-macos-v1.0".into(),
            candidate_id: "rust-core-win-v0.1".into(),
            result: "PASS".into(),
            checks_passed: 7,
            checks_failed: 0,
            completed_at: "2026-07-23T00:00:00Z".into(),
            evidence_path: "evidence/equivalence/eq-run-20260723-001/".into(),
            schema_version: RECEIPT_CONTRACT_VERSION.into(),
        };
        let json = serde_json::to_string_pretty(&r).unwrap();
        assert!(json.contains("EQ-RUN-001"));
        assert!(json.contains("swift-core-macos-v1.0"));
    }
}
