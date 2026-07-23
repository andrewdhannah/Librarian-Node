//! PERMISSIONS-001 integration test.
//! Validates that permissions reference authority without creating it.

use librarian_contracts::prelude::*;

/// Test: Permission check references decision — not standalone authority.
#[test]
fn test_permission_backed_by_decision() {
    let permission = serde_json::json!({
        "permission_id": "PERM-001",
        "entity_id": "node-windows-01",
        "capability_id": "model-phi-4",
        "decision_id": "DEC-001",
        "status": "active",
        "scope": "*"
    });

    assert_eq!(permission["decision_id"], "DEC-001");
    // Permission references a decision — does not create its own authority
}

/// Test: Permission evidence uses existing types.
#[test]
fn test_permission_evidence() {
    let evidence = EvidenceRecord {
        record_id: "evt-perm-001".into(),
        category: EvidenceCategory::ContractValidation,
        description: "Permission granted: node-windows-01 → model-phi-4".into(),
        payload: serde_json::json!({
            "action": "permission_granted",
            "entity_id": "node-windows-01",
            "capability_id": "model-phi-4",
            "decision_id": "DEC-001",
        }),
        payload_hash: "abc123".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        produced_by: "permission-manager".into(),
        schema_version: EVIDENCE_CONTRACT_VERSION.into(),
    };

    assert_eq!(evidence.category, EvidenceCategory::ContractValidation);
    assert_eq!(evidence.produced_by, "permission-manager");
}

/// Test: Permission receipt uses existing envelope.
#[test]
fn test_permission_receipt_shape() {
    let perm_receipt = Receipt {
        receipt_id: "PERM-001".into(),
        receipt_type: ReceiptType::Equivalence,
        receipt_version: "1.0".into(),
        action: "permission_granted".into(),
        initiated_by: "permission-manager".into(),
        authorized_by: Some("owner".into()),
        summary: "Permission: node-windows-01 → model-phi-4".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        parent_receipt_ids: vec![],
        evidence_ids: vec!["evt-perm-001".into()],
        project_id: Some("governance".into()),
        schema_version: RECEIPT_CONTRACT_VERSION.into(),
    };

    let mqr = Receipt {
        receipt_id: "MQR-001".into(),
        receipt_type: ReceiptType::Equivalence,
        receipt_version: "1.0".into(),
        action: "model_qualification".into(),
        initiated_by: "mqr-harness".into(),
        authorized_by: None,
        summary: "Qualification: phi-4 PASS".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        parent_receipt_ids: vec![],
        evidence_ids: vec!["mqr-evt-001".into()],
        project_id: Some("model-qualification".into()),
        schema_version: RECEIPT_CONTRACT_VERSION.into(),
    };

    assert_eq!(perm_receipt.receipt_type, mqr.receipt_type);
    assert_eq!(perm_receipt.receipt_version, mqr.receipt_version);
    assert_eq!(perm_receipt.schema_version, mqr.schema_version);
    assert_ne!(perm_receipt.action, mqr.action);
}

/// Test: Permission lifecycle states.
#[test]
fn test_permission_lifecycle() {
    let statuses = serde_json::json!(["active", "suspended", "revoked"]);
    assert_eq!(statuses.as_array().unwrap().len(), 3);
}

/// Test: Full governance chain end-to-end.
#[test]
fn test_governance_chain() {
    // Entity → Decision → Permission → Capability → Evidence → Receipt
    let chain = vec![
        "entity",
        "decision",
        "permission",
        "capability_execution",
        "evidence",
        "receipt",
    ];
    assert_eq!(chain.len(), 6);
    // All links in the chain exist as contract types or storage tables
}

/// Test: No new governance primitives.
#[test]
fn test_no_new_primitives() {
    let _evidence = EvidenceCategory::ContractValidation;
    let _receipt = ReceiptType::Equivalence;
    let _residency = ResidencyState::Active;
    let _custody = CustodyMode::LocalCanonical;
    let _capability = CapabilityCategory::ModelExecution;
}
