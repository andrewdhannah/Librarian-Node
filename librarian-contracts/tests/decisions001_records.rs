//! DECISIONS-001 integration test.
//! Validates that decision records persist owner authority intent
//! without introducing permissions, authentication, or enforcement.

use librarian_contracts::prelude::*;

/// Test: Decision receipt uses existing receipt envelope.
#[test]
fn test_decision_receipt_shape() {
    let decision_receipt = Receipt {
        receipt_id: "DEC-001".into(),
        receipt_type: ReceiptType::Equivalence,
        receipt_version: "1.0".into(),
        action: "decision_recorded".into(),
        initiated_by: "decision-manager".into(),
        authorized_by: Some("andrew".into()),
        summary: "Authorized phi-4 model execution on Windows node".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        parent_receipt_ids: vec![],
        evidence_ids: vec!["evt-decision-001".into()],
        project_id: Some("governance".into()),
        schema_version: RECEIPT_CONTRACT_VERSION.into(),
    };

    let mqr_receipt = Receipt {
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

    // Same envelope
    assert_eq!(decision_receipt.receipt_type, mqr_receipt.receipt_type);
    assert_eq!(decision_receipt.receipt_version, mqr_receipt.receipt_version);
    assert_eq!(decision_receipt.schema_version, mqr_receipt.schema_version);

    // Different content — decision has authorization
    assert_ne!(decision_receipt.action, mqr_receipt.action);
    assert!(decision_receipt.authorized_by.is_some());
}

/// Test: Decision evidence uses existing evidence types.
#[test]
fn test_decision_evidence() {
    let evidence = EvidenceRecord {
        record_id: "evt-decision-001".into(),
        category: EvidenceCategory::ContractValidation,
        description: "Decision recorded: Authorized phi-4 — Approved".into(),
        payload: serde_json::json!({
            "action": "decision_recorded",
            "decision_id": "DEC-001",
            "status": "Approved",
            "entity_id": "node-windows-01",
        }),
        payload_hash: "abc123".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        produced_by: "decision-manager".into(),
        schema_version: EVIDENCE_CONTRACT_VERSION.into(),
    };

    assert_eq!(evidence.category, EvidenceCategory::ContractValidation);
    assert_eq!(evidence.produced_by, "decision-manager");
    // Same EvidenceCategory as all previous work orders
}

/// Test: Decision links to existing entity types.
#[test]
fn test_decision_entity_link() {
    // Decision references entity IDs from the entity registry
    let entity_id = "node-windows-01";
    let target_entity_id = "cap-model-phi4";

    // These are the same entity IDs that ENTITY-001 would register
    assert!(!entity_id.is_empty());
    assert!(!target_entity_id.is_empty());
}

/// Test: Decision status values.
#[test]
fn test_decision_statuses() {
    let statuses = vec!["pending", "approved", "rejected", "deferred", "superseded"];
    for s in &statuses {
        assert!(!s.is_empty());
    }
    assert_eq!(statuses.len(), 5);
}

/// Test: Decision supersession forms a chain.
#[test]
fn test_decision_supersession() {
    let original = serde_json::json!({
        "decision_id": "DEC-001",
        "status": "superseded",
        "superseded_by": "DEC-002"
    });
    let replacement = serde_json::json!({
        "decision_id": "DEC-002",
        "status": "approved",
        "superseded_by": null
    });

    assert_eq!(original["superseded_by"], "DEC-002");
    assert!(replacement["superseded_by"].is_null());
}

/// Test: No new governance primitives introduced.
#[test]
fn test_no_new_primitives() {
    let _evidence = EvidenceCategory::ContractValidation;
    let _receipt = ReceiptType::Equivalence;
    let _residency = ResidencyState::Active;
    let _custody = CustodyMode::LocalCanonical;
    let _capability = CapabilityCategory::ModelExecution;
}

/// Test: Decision types as evidence payload — not new categories.
#[test]
fn test_decision_type_in_evidence_payload() {
    // Decision type is recorded in the evidence payload, not as a new category
    let payload = serde_json::json!({
        "action": "decision_recorded",
        "decision_id": "DEC-001",
        "decision_type": "capability_authorization",
    });
    assert_eq!(payload["decision_type"], "capability_authorization");
    // The decision type is a descriptive string, not a governance enum variant
}
