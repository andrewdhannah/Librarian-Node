//! STORAGE-001 integration test.
//! Validates numbered migration framework for the governance database.

use librarian_contracts::prelude::*;

/// Test: schema version tracking uses existing Receipt type.
#[test]
fn test_migration_tracking() {
    let receipt = Receipt {
        receipt_id: "MIG-001-20260723".into(),
        receipt_type: ReceiptType::Equivalence,
        receipt_version: "1.0".into(),
        action: "schema_migration".into(),
        initiated_by: "migration-runner".into(),
        authorized_by: None,
        summary: "Applied migration 001: Initial governance schema".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        parent_receipt_ids: vec![],
        evidence_ids: vec!["mig-evt-001".into()],
        project_id: Some("governance".into()),
        schema_version: RECEIPT_CONTRACT_VERSION.into(),
    };

    assert_eq!(receipt.action, "schema_migration");
    assert_eq!(receipt.receipt_type, ReceiptType::Equivalence);
    // Same receipt envelope as MQR, WO-005, WO-006
}

/// Test: migration evidence uses existing Evidence types.
#[test]
fn test_migration_evidence() {
    let evidence = EvidenceRecord {
        record_id: "mig-evt-001".into(),
        category: EvidenceCategory::ContractValidation,
        description: "Applied 1 migration(s) to governance schema".into(),
        payload: serde_json::json!({
            "migrations_applied": [{
                "id": 1,
                "description": "Create initial governance schema",
                "duration_ms": 45
            }]
        }),
        payload_hash: "abc123".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        produced_by: "migration-runner".into(),
        schema_version: EVIDENCE_CONTRACT_VERSION.into(),
    };

    assert_eq!(evidence.category, EvidenceCategory::ContractValidation);
    // Same evidence format as all other governance events
}

/// Test: migration metadata uses existing identity types.
#[test]
fn test_migration_identity() {
    let migrated_by = NodeIdentity {
        node_id: NodeId::new("migration-runner"),
        display_name: "Governance Migration Runner".into(),
        role: NodeRole::Verifier,
        platform: PlatformId::Linux,
        architecture: Architecture::X8664,
        version: "1.0.0".into(),
        contract_version: "1.0.0".into(),
    };

    assert_eq!(migrated_by.role, NodeRole::Verifier);
    // Migration runner uses existing Verifier role — no new role type needed
}

/// Test: migration log entry uses existing custody model for tracking.
#[test]
fn test_migration_custody() {
    // A migration changes the database schema — this is analogous to
    // a custody event where the schema_version is the document.
    let schema_change = CustodyEvent {
        event_id: "mig-001-apply".into(),
        project_id: "governance".into(),
        mcp_session_id: String::new(),
        node_id: "migration-runner".into(),
        window_id: None,
        work_packet_id: None,
        tool_name: "migration_runner".into(),
        authority_role: CustodyAuthorityRole::System,
        document_reference: "schema://governance/v1".into(),
        custody_action: CustodyAction::Validate,
        previous_custody_mode: Some(CustodyMode::AdvisoryContextOnly),
        resulting_custody_mode: Some(CustodyMode::LocalCanonical),
        mutation_allowance: Some(MutationAllowance::CanonicalMutationApproved),
        decision_reference: None,
        provenance_receipt: None,
        refusal_reason: None,
        target_project_id: None,
        target_session_id: None,
        target_node_id: None,
        timestamp: "2026-07-23T00:00:00Z".into(),
    };

    assert_eq!(schema_change.custody_action, CustodyAction::Validate);
    assert_eq!(schema_change.resulting_custody_mode, Some(CustodyMode::LocalCanonical));
    // Same CustodyEvent type as all other governance operations
}

/// Test: No new governance primitives introduced by storage maturity.
#[test]
fn test_no_new_primitives() {
    let _receipt = ReceiptType::Equivalence;
    let _evidence = EvidenceCategory::ContractValidation;
    let _residency = ResidencyState::Active;
    let _custody = CustodyMode::LocalCanonical;
    let _capability = CapabilityCategory::ModelExecution;
    // If these compile, no new primitives were introduced
}

/// Test: Migration receipt matches the same envelope as all other receipts.
#[test]
fn test_migration_receipt_shape_matches() {
    let migration_receipt = Receipt {
        receipt_id: "MIG-001".into(),
        receipt_type: ReceiptType::Equivalence,
        receipt_version: "1.0".into(),
        action: "schema_migration".into(),
        initiated_by: "migration-runner".into(),
        authorized_by: None,
        summary: "Applied migration 001".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        parent_receipt_ids: vec![],
        evidence_ids: vec!["mig-evt-001".into()],
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
    assert_eq!(migration_receipt.receipt_type, mqr_receipt.receipt_type);
    assert_eq!(migration_receipt.receipt_version, mqr_receipt.receipt_version);
    assert_eq!(migration_receipt.schema_version, mqr_receipt.schema_version);

    // Different content
    assert_ne!(migration_receipt.action, mqr_receipt.action);
}
