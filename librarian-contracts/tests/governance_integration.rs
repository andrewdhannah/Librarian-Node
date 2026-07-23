//! Integration test: Governance module via contract types.
//! Tests that the contract types + governance engine work together.
//! Does not depend on the librarian-core scaffold.

use librarian_contracts::prelude::*;

#[test]
fn test_cursor_initialization_and_advancement() {
    // Verify lifecycle states and transitions
    let cursor = LifecycleCursor {
        project_id: "test-project".into(),
        current_state: LifecycleState::Install,
        cycle: 1,
        cursor_position: 1,
        last_transition_at: "2026-07-23T00:00:00Z".into(),
        last_reconciled_at: None,
        reason: Some("Initialized".into()),
        schema_version: LIFECYCLE_CONTRACT_VERSION.into(),
    };
    assert_eq!(cursor.current_state, LifecycleState::Install);
    assert!(LifecycleState::Install.can_transition_to(&LifecycleState::Initialize));
    assert!(!LifecycleState::Install.can_transition_to(&LifecycleState::Operational));
}

#[test]
fn test_custody_claim_and_release() {
    let event = CustodyEvent {
        event_id: "ce-001".into(),
        project_id: "test".into(),
        mcp_session_id: "session-1".into(),
        node_id: "node-1".into(),
        window_id: None,
        work_packet_id: None,
        tool_name: "test".into(),
        authority_role: CustodyAuthorityRole::Agent,
        document_reference: "doc://contract".into(),
        custody_action: CustodyAction::Claim,
        previous_custody_mode: None,
        resulting_custody_mode: Some(CustodyMode::LocalWorkingCopy),
        mutation_allowance: Some(MutationAllowance::WorkingCopyOnly),
        decision_reference: None,
        provenance_receipt: None,
        refusal_reason: None,
        target_project_id: None,
        target_session_id: None,
        target_node_id: None,
        timestamp: "2026-07-23T00:00:00Z".into(),
    };
    assert_eq!(event.custody_action, CustodyAction::Claim);
    assert_eq!(event.resulting_custody_mode, Some(CustodyMode::LocalWorkingCopy));
}

#[test]
fn test_evidence_generation() {
    let evidence = ExecutionEvidence {
        id: "evt-001".into(),
        session_id: "session-1".into(),
        work_packet_id: "wp-001".into(),
        tool: "governance_test".into(),
        target: "test".into(),
        risk_class: RiskClass::R0,
        authority_decision: AuthorityDecision::Permitted,
        success: true,
        output_summary: "Test evidence".into(),
        error_detail: None,
        duration_ms: 0,
        timestamp: "2026-07-23T00:00:00Z".into(),
        schema_version: EVIDENCE_CONTRACT_VERSION.into(),
    };
    assert!(evidence.success);
    assert_eq!(evidence.authority_decision, AuthorityDecision::Permitted);
}

#[test]
fn test_receipt_types() {
    let auth_receipt = SprintAuthorizationReceipt {
        receipt_id: "AR-TEST-001".into(),
        receipt_type: ReceiptType::SprintAuthorization,
        work_order_id: "WO-004".into(),
        authorized_at: "2026-07-23T00:00:00Z".into(),
        authorized_by: "Owner".into(),
        repository: "Librarian-Node".into(),
        scope: "Rust Core Governance Port".into(),
        capability_expansion: None,
        migration_code: false,
        schema_version: RECEIPT_CONTRACT_VERSION.into(),
    };
    assert_eq!(auth_receipt.work_order_id, "WO-004");

    let eq_receipt = EquivalenceReceipt {
        receipt_id: "EQ-TEST-001".into(),
        equivalence_run_id: "eq-run-test".into(),
        baseline_id: "swift-core".into(),
        candidate_id: "rust-core".into(),
        result: "PASS".into(),
        checks_passed: 7,
        checks_failed: 0,
        completed_at: "2026-07-23T00:00:00Z".into(),
        evidence_path: "evidence/equivalence/test/".into(),
        schema_version: RECEIPT_CONTRACT_VERSION.into(),
    };
    assert_eq!(eq_receipt.result, "PASS");
    assert_eq!(eq_receipt.checks_passed, 7);
}

#[test]
fn test_serialization_round_trip() {
    use librarian_contracts::serialization::*;

    let receipt = SprintAuthorizationReceipt {
        receipt_id: "RT-TEST-001".into(),
        receipt_type: ReceiptType::SprintAuthorization,
        work_order_id: "WO-004".into(),
        authorized_at: "2026-07-23T00:00:00Z".into(),
        authorized_by: "Owner".into(),
        repository: "Librarian-Node".into(),
        scope: "Round-trip test".into(),
        capability_expansion: None,
        migration_code: false,
        schema_version: RECEIPT_CONTRACT_VERSION.into(),
    };

    let json_a = to_canonical_json(&receipt).unwrap();
    let json_b = to_canonical_json(&receipt).unwrap();
    assert_eq!(json_a, json_b, "Canonical JSON must be deterministic");

    let hash_a = hash_canonical(&receipt).unwrap();
    let hash_b = hash_canonical(&receipt).unwrap();
    assert_eq!(hash_a, hash_b, "SHA-256 must be deterministic");
    assert_eq!(hash_a.len(), 64, "SHA-256 hex must be 64 chars");
}

#[test]
fn test_all_contract_versions_present() {
    assert!(!IDENTITY_CONTRACT_VERSION.is_empty());
    assert!(!LIFECYCLE_CONTRACT_VERSION.is_empty());
    assert!(!EVIDENCE_CONTRACT_VERSION.is_empty());
    assert!(!RECEIPT_CONTRACT_VERSION.is_empty());
    assert!(!CUSTODY_CONTRACT_VERSION.is_empty());
    assert!(!CAPABILITY_CONTRACT_VERSION.is_empty());
    assert!(!ERROR_CONTRACT_VERSION.is_empty());
    assert!(!SERIALIZATION_CONTRACT_VERSION.is_empty());
}

#[test]
fn test_custody_envelope_operations() {
    let envelope = CustodyEnvelope {
        document_reference: "doc://governance/contract".into(),
        mode: CustodyMode::OwnerHeld,
        held_by: "owner".into(),
        acquired_at: "2026-07-23T00:00:00Z".into(),
        expires_at: None,
        establishing_event_id: "ce-001".into(),
        content_hash: "abc123".into(),
        schema_version: CUSTODY_CONTRACT_VERSION.into(),
    };
    assert_eq!(envelope.mode, CustodyMode::OwnerHeld);
    assert!(CustodyMode::OwnerHeld.allows_mutation());
}

#[test]
fn test_lifecycle_transition_table() {
    let all = LifecycleState::ALL;
    assert_eq!(all.len(), 11);

    // Verify each state can transition to something (except Retired)
    for state in all {
        if *state == LifecycleState::Retired {
            assert!(state.valid_transitions().is_empty());
            assert!(state.is_terminal());
        } else {
            assert!(!state.valid_transitions().is_empty(),
                "State {:?} has no valid transitions", state);
        }
    }
}
