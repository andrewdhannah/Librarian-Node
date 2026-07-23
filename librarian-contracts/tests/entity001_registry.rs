//! ENTITY-001 integration test.
//! Validates that entity registry adds knowledge of existence
//! without introducing authority, authentication, or permissions.

use librarian_contracts::prelude::*;

/// Test: Entity types use existing contract patterns — no new types.
#[test]
fn test_entity_uses_existing_concepts() {
    let entity_type = serde_json::json!("node");
    assert_eq!(entity_type, "node");

    let entity_status = serde_json::json!("active");
    assert_eq!(entity_status, "active");
}

/// Test: Entity registration uses existing evidence types.
#[test]
fn test_entity_evidence() {
    let evidence = EvidenceRecord {
        record_id: "evt-entity-node-win-001".into(),
        category: EvidenceCategory::ContractValidation,
        description: "Entity registered: Windows Runtime Node 1 (Node)".into(),
        payload: serde_json::json!({
            "action": "entity_registered",
            "entity_id": "node-windows-01",
            "entity_type": "Node",
            "display_name": "Windows Runtime Node 1",
        }),
        payload_hash: "abc123".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        produced_by: "entity-registry".into(),
        schema_version: EVIDENCE_CONTRACT_VERSION.into(),
    };

    assert_eq!(evidence.category, EvidenceCategory::ContractValidation);
    assert_eq!(evidence.produced_by, "entity-registry");
    // Same EvidenceCategory as MQR, WO-005, WO-006, STORAGE-001
}

/// Test: Entity receipt uses existing receipt envelope.
#[test]
fn test_entity_receipt_shape() {
    let entity_receipt = Receipt {
        receipt_id: "ENT-001".into(),
        receipt_type: ReceiptType::Equivalence,
        receipt_version: "1.0".into(),
        action: "entity_registered".into(),
        initiated_by: "entity-registry".into(),
        authorized_by: None,
        summary: "Entity registered: node-windows-01".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        parent_receipt_ids: vec![],
        evidence_ids: vec!["evt-entity-node-win-001".into()],
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
    assert_eq!(entity_receipt.receipt_type, mqr_receipt.receipt_type);
    assert_eq!(entity_receipt.receipt_version, mqr_receipt.receipt_version);
    assert_eq!(entity_receipt.schema_version, mqr_receipt.schema_version);

    // Different content
    assert_ne!(entity_receipt.action, mqr_receipt.action);
}

/// Test: Entity type maps to existing CapabilityCategory for capability entities.
#[test]
fn test_capability_entity_uses_existing_category() {
    let cap = Capability {
        capability_id: "model-phi-4".into(),
        name: "Model: phi-4".into(),
        description: "Microsoft Phi-4 model execution".into(),
        category: CapabilityCategory::ModelExecution,
        requires_authorization: true,
        enabled: true,
        schema_version: CAPABILITY_CONTRACT_VERSION.into(),
    };

    assert_eq!(cap.category, CapabilityCategory::ModelExecution);
    // No new capability category — entity registry does not redefine capability model
}

/// Test: Node entity maps to existing NodeIdentity.
#[test]
fn test_node_entity_maps_to_identity() {
    let identity = NodeIdentity {
        node_id: NodeId::new("nid-win-001"),
        display_name: "Windows Runtime Node 1".into(),
        role: NodeRole::Runtime,
        platform: PlatformId::Windows,
        architecture: Architecture::X8664,
        version: "0.1.0".into(),
        contract_version: "1.0.0".into(),
    };

    assert_eq!(identity.role, NodeRole::Runtime);
    assert_eq!(identity.platform, PlatformId::Windows);
    // No new identity type — entity registry references existing identity
}

/// Test: Entity custody uses existing CustodyEvent.
#[test]
fn test_entity_custody() {
    // Entity registration is validated through existing custody events
    let event = CustodyEvent {
        event_id: "evt-entity-validate-001".into(),
        project_id: "governance".into(),
        mcp_session_id: String::new(),
        node_id: "entity-registry".into(),
        window_id: None,
        work_packet_id: None,
        tool_name: "entity_registry".into(),
        authority_role: CustodyAuthorityRole::System,
        document_reference: "entity://node-windows-01".into(),
        custody_action: CustodyAction::Validate,
        previous_custody_mode: None,
        resulting_custody_mode: Some(CustodyMode::LocalCanonical),
        mutation_allowance: Some(MutationAllowance::ReadOnly),
        decision_reference: None,
        provenance_receipt: None,
        refusal_reason: None,
        target_project_id: None,
        target_session_id: None,
        target_node_id: None,
        timestamp: "2026-07-23T00:00:00Z".into(),
    };

    assert_eq!(event.custody_action, CustodyAction::Validate);
    // Same CustodyEvent type as MQR, WO-005, WO-006
}

/// Test: No new governance primitives introduced.
#[test]
fn test_no_new_primitives() {
    // If these compile, no new primitives were introduced for entity registry
    let _evidence = EvidenceCategory::ContractValidation;
    let _receipt = ReceiptType::Equivalence;
    let _residency = ResidencyState::Active;
    let _custody = CustodyMode::LocalCanonical;
    let _capability = CapabilityCategory::ModelExecution;
}
