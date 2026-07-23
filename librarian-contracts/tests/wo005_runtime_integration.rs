//! WO-005 integration test.
//! Validates that runtime execution can be governed using existing primitives
//! without introducing platform-specific governance types.

use librarian_contracts::prelude::*;

/// Test: Runtime start maps to ResidencyState transition.
#[test]
fn test_runtime_start_as_residency() {
    let residency = ResidencyRecord {
        record_id: "res-router-001".into(),
        component_id: "router-service".into(),
        current_state: ResidencyState::Active,
        last_transition_at: "2026-07-23T00:00:00Z".into(),
        host_node: "node-windows-1".into(),
        schema_version: RESIDENCY_CONTRACT_VERSION.into(),
    };

    assert_eq!(residency.current_state, ResidencyState::Active);
    assert!(residency.current_state.is_occupying_resources());
    // Same ResidencyState used by MQR — no runtime-specific state machine
}

/// Test: Runtime lifecycle through ResidencyState transitions.
#[test]
fn test_runtime_lifecycle_transitions() {
    // Full lifecycle through ResidencyState
    let requested = ResidencyState::Requested;
    let loading = ResidencyState::Loading;
    let loaded = ResidencyState::Loaded;
    let active = ResidencyState::Active;
    let releasing = ResidencyState::Releasing;
    let released = ResidencyState::Released;

    assert!(requested.can_transition_to(&loading));
    assert!(loading.can_transition_to(&loaded));
    assert!(loaded.can_transition_to(&active));
    assert!(active.can_transition_to(&releasing));
    assert!(releasing.can_transition_to(&released));
    assert!(!released.is_occupying_resources());
}

/// Test: Runtime crash maps to ResidencyState::Failed.
#[test]
fn test_runtime_crash_as_failed() {
    let failed = ResidencyState::Failed;
    assert!(!failed.is_occupying_resources());
    assert!(failed.is_terminal());
    // Can restart from Failed → Requested
    assert!(failed.can_transition_to(&ResidencyState::Requested));
}

/// Test: Runtime events produce evidence using existing types.
#[test]
fn test_runtime_evidence() {
    let evidence = EvidenceRecord {
        record_id: "rt-evt-router-start-001".into(),
        category: EvidenceCategory::ContractValidation,
        description: "Runtime event: Started — Released → Active".into(),
        payload: serde_json::json!({
            "component_id": "router-service",
            "event": "Started",
            "from_residency": "Released",
            "to_residency": "Active",
        }),
        payload_hash: "abc123".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        produced_by: "runtime-supervisor".into(),
        schema_version: EVIDENCE_CONTRACT_VERSION.into(),
    };

    assert_eq!(evidence.category, EvidenceCategory::ContractValidation);
    assert_eq!(evidence.produced_by, "runtime-supervisor");
    // Same EvidenceCategory used by MQR — no runtime-specific evidence type
}

/// Test: Runtime lifecycle receipts use existing types.
#[test]
fn test_runtime_receipt() {
    let receipt = EquivalenceReceipt {
        receipt_id: "RT-ROUTER-START-001".into(),
        equivalence_run_id: "rt-run-001".into(),
        baseline_id: "runtime-spec".into(),
        candidate_id: "router-service".into(),
        result: "PASS".into(),
        checks_passed: 1,
        checks_failed: 0,
        completed_at: "2026-07-23T00:00:00Z".into(),
        evidence_path: "rt-evt-router-start-001".into(),
        schema_version: RECEIPT_CONTRACT_VERSION.into(),
    };

    assert_eq!(receipt.result, "PASS");
    assert_eq!(receipt.candidate_id, "router-service");
    // Same EquivalenceReceipt used by MQR — no runtime-specific receipt type
}

/// Test: Receipt shape equivalence between MQR and WO-005.
#[test]
fn test_runtime_receipt_shape_matches_mqr() {
    let runtime_receipt = Receipt {
        receipt_id: "RT-001".into(),
        receipt_type: ReceiptType::Equivalence,
        receipt_version: "1.0".into(),
        action: "runtime_start".into(),
        initiated_by: "runtime-supervisor".into(),
        authorized_by: None,
        summary: "Runtime: router-service started".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        parent_receipt_ids: vec![],
        evidence_ids: vec!["rt-evt-001".into()],
        project_id: Some("runtime-supervision".into()),
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

    // Same envelope shape
    assert_eq!(runtime_receipt.receipt_type, mqr_receipt.receipt_type);
    assert_eq!(runtime_receipt.receipt_version, mqr_receipt.receipt_version);
    assert_eq!(runtime_receipt.schema_version, mqr_receipt.schema_version);

    // Different domain content
    assert_ne!(runtime_receipt.action, mqr_receipt.action);
    assert_ne!(runtime_receipt.project_id, mqr_receipt.project_id);
    assert_ne!(runtime_receipt.evidence_ids[0], mqr_receipt.evidence_ids[0]);
}

/// Test: No platform-specific governance primitives.
#[test]
fn test_no_platform_specific_primitives() {
    // Verify only existing types are used for runtime governance
    let _residency = ResidencyState::Active;
    let _evidence = EvidenceCategory::ContractValidation;
    let _receipt = ReceiptType::Equivalence;
    let _custody = CustodyMode::LocalCanonical;
    let _action = CustodyAction::Claim;

    // If these compile, no platform-specific types leaked in
}

/// Test: Runtime custody events.
#[test]
fn test_runtime_custody_events() {
    let claim = CustodyEvent {
        event_id: "rt-claim-router-001".into(),
        project_id: "runtime-supervision".into(),
        mcp_session_id: String::new(),
        node_id: "runtime-supervisor".into(),
        window_id: None,
        work_packet_id: None,
        tool_name: "runtime_supervisor".into(),
        authority_role: CustodyAuthorityRole::System,
        document_reference: "runtime://router-service".into(),
        custody_action: CustodyAction::Claim,
        previous_custody_mode: Some(CustodyMode::AdvisoryContextOnly),
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

    assert_eq!(claim.custody_action, CustodyAction::Claim);
    assert_eq!(claim.resulting_custody_mode, Some(CustodyMode::LocalCanonical));

    let release = CustodyEvent {
        event_id: "rt-release-router-001".into(),
        custody_action: CustodyAction::Release,
        resulting_custody_mode: Some(CustodyMode::AdvisoryContextOnly),
        ..claim
    };

    assert_eq!(release.custody_action, CustodyAction::Release);
    // Same CustodyEvent type used by MQR — no runtime-specific custody type
}
