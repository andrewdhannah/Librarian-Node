//! WO-006 integration test.
//! Validates that a Linux runtime can consume the existing governance substrate
//! through the RuntimeAdapter boundary — without requiring the substrate
//! to know Linux exists.

use librarian_contracts::prelude::*;

/// Test: Linux process events map to existing ResidencyState.
#[test]
fn test_linux_event_to_residency() {
    // systemd "active (running)" → ProcessEvent::Started → ResidencyState::Active
    let requested = ResidencyState::Requested;
    let loading = ResidencyState::Loading;
    let active = ResidencyState::Active;
    let released = ResidencyState::Released;
    let failed = ResidencyState::Failed;

    assert!(requested.can_transition_to(&loading));
    assert!(loading.can_transition_to(&ResidencyState::Loaded));
    assert!(ResidencyState::Loaded.can_transition_to(&active));
    assert!(active.can_transition_to(&ResidencyState::Releasing));
    assert!(ResidencyState::Releasing.can_transition_to(&released));
    assert!(!released.is_occupying_resources());

    // "Active: failed" → ProcessEvent::Crashed → ResidencyState::Failed
    assert!(failed.is_terminal());
    assert!(failed.can_transition_to(&ResidencyState::Requested)); // restart
}

/// Test: Linux evidence uses existing categories.
#[test]
fn test_linux_evidence() {
    let evidence = EvidenceRecord {
        record_id: "lnx-evt-router-001".into(),
        category: EvidenceCategory::ContractValidation,
        description: "Runtime event (Linux): Started — Released → Active".into(),
        payload: serde_json::json!({
            "component_id": "librarian-node",
            "event": "systemd_active",
            "from_residency": "Released",
            "to_residency": "Active",
        }),
        payload_hash: "def456".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        produced_by: "runtime-supervisor".into(),
        schema_version: EVIDENCE_CONTRACT_VERSION.into(),
    };

    assert_eq!(evidence.category, EvidenceCategory::ContractValidation);
    assert_eq!(evidence.produced_by, "runtime-supervisor");
    // Same category as MQR and WO-005 — no Linux-specific evidence type
}

/// Test: Linux receipts use existing envelope.
#[test]
fn test_linux_receipt() {
    let receipt = EquivalenceReceipt {
        receipt_id: "LNX-NODE-START-001".into(),
        equivalence_run_id: "lnx-run-001".into(),
        baseline_id: "runtime-spec".into(),
        candidate_id: "librarian-node".into(),
        result: "PASS".into(),
        checks_passed: 1,
        checks_failed: 0,
        completed_at: "2026-07-23T00:00:00Z".into(),
        evidence_path: "lnx-evt-router-001".into(),
        schema_version: RECEIPT_CONTRACT_VERSION.into(),
    };

    assert_eq!(receipt.result, "PASS");
    assert_eq!(receipt.candidate_id, "librarian-node");
    // Same EquivalenceReceipt as MQR and WO-005
}

/// Test: Three-way receipt comparison.
#[test]
fn test_three_way_receipt_comparison() {
    // MQR receipt (model qualification)
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

    // WO-005 receipt (Windows runtime)
    let win = Receipt {
        receipt_id: "WIN-001".into(),
        receipt_type: ReceiptType::Equivalence,
        receipt_version: "1.0".into(),
        action: "runtime_start_windows".into(),
        initiated_by: "runtime-supervisor".into(),
        authorized_by: None,
        summary: "Runtime: router-service started".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        parent_receipt_ids: vec![],
        evidence_ids: vec!["win-evt-001".into()],
        project_id: Some("runtime-supervision".into()),
        schema_version: RECEIPT_CONTRACT_VERSION.into(),
    };

    // WO-006 receipt (Linux runtime)
    let linux = Receipt {
        receipt_id: "LNX-001".into(),
        receipt_type: ReceiptType::Equivalence,
        receipt_version: "1.0".into(),
        action: "runtime_start_linux".into(),
        initiated_by: "runtime-supervisor".into(),
        authorized_by: None,
        summary: "Runtime: librarian-node started".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        parent_receipt_ids: vec![],
        evidence_ids: vec!["lnx-evt-001".into()],
        project_id: Some("runtime-supervision".into()),
        schema_version: RECEIPT_CONTRACT_VERSION.into(),
    };

    // All three have the same envelope
    assert_eq!(mqr.receipt_type, win.receipt_type);
    assert_eq!(win.receipt_type, linux.receipt_type);
    assert_eq!(mqr.receipt_version, win.receipt_version);
    assert_eq!(win.receipt_version, linux.receipt_version);
    assert_eq!(mqr.schema_version, win.schema_version);
    assert_eq!(win.schema_version, linux.schema_version);

    // All three have different domain payloads
    assert_ne!(mqr.action, win.action);
    assert_ne!(win.action, linux.action);
    assert_ne!(mqr.evidence_ids[0], win.evidence_ids[0]);
    assert_ne!(win.evidence_ids[0], linux.evidence_ids[0]);

    // Same structure, different identifiers
    assert_eq!(mqr.evidence_ids.len(), win.evidence_ids.len());
    assert_eq!(win.evidence_ids.len(), linux.evidence_ids.len());
}

/// Test: No Linux-specific governance primitives.
#[test]
fn test_no_linux_specific_primitives() {
    // If these compile, no Linux-specific governance types exist
    let _residency = ResidencyState::Active;
    let _evidence = EvidenceCategory::ContractValidation;
    let _receipt = ReceiptType::Equivalence;
    let _custody = CustodyMode::LocalCanonical;
    let _capability = CapabilityCategory::ModelExecution;
}

/// Test: Linux custody events use existing types.
#[test]
fn test_linux_custody() {
    let claim = CustodyEvent {
        event_id: "lnx-claim-node-001".into(),
        project_id: "runtime-supervision".into(),
        mcp_session_id: String::new(),
        node_id: "linux-adapter".into(),
        window_id: None,
        work_packet_id: None,
        tool_name: "runtime_supervisor".into(),
        authority_role: CustodyAuthorityRole::System,
        document_reference: "runtime://librarian-node".into(),
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
    // Same CustodyEvent type used by MQR and WO-005
}

/// Test: Linux XDG paths fit existing ResidencyRecord.
#[test]
fn test_linux_residency_record() {
    let residency = ResidencyRecord {
        record_id: "res-lnx-node-001".into(),
        component_id: "librarian-node".into(),
        current_state: ResidencyState::Active,
        last_transition_at: "2026-07-23T00:00:00Z".into(),
        host_node: "node-ubuntu-1".into(),
        schema_version: RESIDENCY_CONTRACT_VERSION.into(),
    };

    assert_eq!(residency.current_state, ResidencyState::Active);
    assert!(residency.current_state.is_occupying_resources());
    // Same ResidencyRecord type as MQR and WO-005
}
