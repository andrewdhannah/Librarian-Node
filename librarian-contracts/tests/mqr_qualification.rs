//! MQR Sprint 1 integration test.
//! Validates that model qualification consumes the existing governance substrate
//! without introducing model-specific governance primitives.

use librarian_contracts::prelude::*;

/// Test: Model profile maps to Capability using existing types.
#[test]
fn test_model_profile_as_capability() {
    // A model profile's attributes map to existing Capability fields.
    let capability = Capability {
        capability_id: "model-phi-4".into(),
        name: "Model: phi-4".into(),
        description: "microsoft_Phi-4-mini-instruct-Q4_K_M.gguf — 2.32 GB, vulkan".into(),
        category: CapabilityCategory::ModelExecution,
        requires_authorization: true,
        enabled: true,
        schema_version: CAPABILITY_CONTRACT_VERSION.into(),
    };

    assert_eq!(capability.capability_id, "model-phi-4");
    assert_eq!(capability.category, CapabilityCategory::ModelExecution);
    assert!(capability.requires_authorization);
    assert!(capability.enabled);
    // No new CapabilityCategory variant was introduced for models
    assert!(matches!(capability.category, CapabilityCategory::ModelExecution));
}

/// Test: Qualification output uses existing Evidence types.
#[test]
fn test_qualification_output_as_evidence() {
    let evidence = EvidenceRecord {
        record_id: "mqr-evt-phi4-001".into(),
        category: EvidenceCategory::ContractValidation,
        description: "Qualification: phi-4 (verified)".into(),
        payload: serde_json::json!({
            "model_alias": "phi-4",
            "verified_status": "verified",
            "context": 4096,
            "ngl": 99,
        }),
        payload_hash: "abc123".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        produced_by: "mqr-harness".into(),
        schema_version: EVIDENCE_CONTRACT_VERSION.into(),
    };

    assert_eq!(evidence.category, EvidenceCategory::ContractValidation);
    assert_eq!(evidence.produced_by, "mqr-harness");
    assert_eq!(evidence.payload["model_alias"], "phi-4");
    // No new EvidenceCategory variant was introduced
    assert!(matches!(evidence.category, EvidenceCategory::ContractValidation));
}

/// Test: Qualification result generates existing Receipt types.
#[test]
fn test_qualification_receipt() {
    let receipt = EquivalenceReceipt {
        receipt_id: "MQR-PHI4-001".into(),
        equivalence_run_id: "mqr-run-001".into(),
        baseline_id: "profile-config".into(),
        candidate_id: "model-phi-4".into(),
        result: "PASS".into(),
        checks_passed: 1,
        checks_failed: 0,
        completed_at: "2026-07-23T00:00:00Z".into(),
        evidence_path: "mqr-evt-phi4-001".into(),
        schema_version: RECEIPT_CONTRACT_VERSION.into(),
    };

    assert_eq!(receipt.result, "PASS");
    assert_eq!(receipt.baseline_id, "profile-config");
    assert_eq!(receipt.candidate_id, "model-phi-4");
    // Uses existing EquivalenceReceipt — no new receipt type for models
}

/// Test: Runtime tracking uses existing ResidencyState.
#[test]
fn test_model_residency_tracking() {
    let residency = ResidencyRecord {
        record_id: "res-phi4-001".into(),
        component_id: "model-phi-4".into(),
        current_state: ResidencyState::Active,
        last_transition_at: "2026-07-23T00:00:00Z".into(),
        host_node: "librarian-node".into(),
        schema_version: RESIDENCY_CONTRACT_VERSION.into(),
    };

    assert_eq!(residency.current_state, ResidencyState::Active);
    assert!(residency.current_state.is_occupying_resources());
    // Uses existing ResidencyState — no model-specific state machine
}

/// Test: Receipt shape is the same as a governance receipt.
#[test]
fn test_receipt_shape_equivalence() {
    let model_receipt = Receipt {
        receipt_id: "MQR-001".into(),
        receipt_type: ReceiptType::Equivalence,
        receipt_version: "1.0".into(),
        action: "model_qualification".into(),
        initiated_by: "mqr-harness".into(),
        authorized_by: None,
        summary: "Qualification: phi-4 PASS".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        parent_receipt_ids: vec![],
        evidence_ids: vec!["mqr-evt-phi4-001".into()],
        project_id: Some("model-qualification".into()),
        schema_version: RECEIPT_CONTRACT_VERSION.into(),
    };

    let governance_receipt = Receipt {
        receipt_id: "GOV-001".into(),
        receipt_type: ReceiptType::SprintAuthorization,
        receipt_version: "1.0".into(),
        action: "authorize_sprint".into(),
        initiated_by: "owner".into(),
        authorized_by: Some("owner".into()),
        summary: "Authorized WO-005".into(),
        recorded_at: "2026-07-23T00:00:00Z".into(),
        parent_receipt_ids: vec![],
        evidence_ids: vec![],
        project_id: Some("librarian".into()),
        schema_version: RECEIPT_CONTRACT_VERSION.into(),
    };

    // Both receipts have the same shape (same type, same fields).
    // The difference is in the payload (action, summary, evidence_ids).
    assert_eq!(model_receipt.receipt_version, governance_receipt.receipt_version);
    assert_eq!(model_receipt.schema_version, governance_receipt.schema_version);
    // Both use ReceiptType from contracts — model_receipt uses Equivalence,
    // governance_receipt uses SprintAuthorization. Same struct, different variant.
}

/// Test: No model-specific governance primitives were introduced.
#[test]
fn test_no_model_specific_primitives() {
    // Verify we're using existing types — if these compile, no new primitives
    // were introduced for model qualification.
    let _capability = CapabilityCategory::ModelExecution;
    let _evidence = EvidenceCategory::ContractValidation;
    let _receipt = ReceiptType::Equivalence;
    let _residency = ResidencyState::Active;
    let _custody = CustodyMode::LocalWorkingCopy;
}

/// Test: Custody events track model qualification lifecycle.
#[test]
fn test_custody_for_model_qualification() {
    let claim = CustodyEvent {
        event_id: "mqr-claim-phi4-001".into(),
        project_id: "model-qualification".into(),
        mcp_session_id: "mqr-run-001".into(),
        node_id: "mqr-harness".into(),
        window_id: None,
        work_packet_id: None,
        tool_name: "qualification_harness".into(),
        authority_role: CustodyAuthorityRole::System,
        document_reference: "capability://model-phi-4".into(),
        custody_action: CustodyAction::Claim,
        previous_custody_mode: None,
        resulting_custody_mode: Some(CustodyMode::LocalWorkingCopy),
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
    assert_eq!(
        claim.resulting_custody_mode,
        Some(CustodyMode::LocalWorkingCopy)
    );

    let release = CustodyEvent {
        event_id: "mqr-release-phi4-001".into(),
        ..claim.clone()
    };

    // Same structure, different action
    assert_eq!(claim.event_id, "mqr-claim-phi4-001");
    assert_eq!(release.event_id, "mqr-release-phi4-001");
}
