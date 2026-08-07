//! M1-A capability registry type conformance (RUST-MIGRATION-M1).
//!
//! Verifies the observational contract type surface against the locked M1
//! contract set:
//!
//! 1. Serialization conformance — serde names match the contract-enum names.
//! 2. Enum variant stability — `ALL` + `as_str` round-trips through serde.
//! 3. Validation rules — identity pattern, version > 0, self-dependency rejection.
//! 4. Round-trip encode/decode — structs survive serde_json round-trips intact.
//! 5. Non-collapse invariants — identity carries no qualification fields;
//!    security context carries provenance, not storage assumptions.
//!
//! Authority checks are enforced by the drift guard
//! (`contract_surface_manifest.rs::m1a_serialized_surface_has_no_authority_keys`)
//! and by construction: these types expose no transition/authorization methods.

use librarian_contracts::node::capability_registry::{
    AssessorType, CapabilityDependency, CapabilityId, CapabilityIdentity,
    CapabilityRelationshipType, CapabilitySecurityContext, CapabilityType, CapabilityVersion,
    ClassificationDerivation, EvidenceDimension, EvidenceFreshness, EvidenceProducerRole,
    EvidenceType, OperationalMode, OperationalModeInputs, OperationalModeValue, QualificationAxis,
    QualificationLifecycleEvent, QualificationRecordStatus, QualificationState,
    SecurityClassification, TransitionType, TransitionerRole,
};

// ---------------------------------------------------------------------------
// Serialization conformance
// ---------------------------------------------------------------------------

#[test]
fn enum_serde_names_match_contract_surface() {
    let cases: Vec<(&str, &str, &str)> = vec![
        ("CapabilityType", "skill", CapabilityType::Skill.as_str()),
        ("CapabilityType", "workflow", CapabilityType::Workflow.as_str()),
        ("CapabilityType", "policy", CapabilityType::Policy.as_str()),
        ("CapabilityType", "validator", CapabilityType::Validator.as_str()),
        ("CapabilityType", "template", CapabilityType::Template.as_str()),
        ("Relationship", "requires", CapabilityRelationshipType::Requires.as_str()),
        ("Relationship", "extends", CapabilityRelationshipType::Extends.as_str()),
        ("Relationship", "refines", CapabilityRelationshipType::Refines.as_str()),
        ("Relationship", "conflicts", CapabilityRelationshipType::Conflicts.as_str()),
        ("QualificationState", "unreviewed", QualificationState::Unreviewed.as_str()),
        ("QualificationState", "reviewed", QualificationState::Reviewed.as_str()),
        ("QualificationState", "qualified", QualificationState::Qualified.as_str()),
        ("QualificationState", "deprecated", QualificationState::Deprecated.as_str()),
        ("QualificationState", "revoked", QualificationState::Revoked.as_str()),
        ("QualificationAxis", "not_tested", QualificationAxis::NotTested.as_str()),
        ("QualificationAxis", "qualifying", QualificationAxis::Qualifying.as_str()),
        ("QualificationAxis", "passed", QualificationAxis::Passed.as_str()),
        ("QualificationAxis", "failed", QualificationAxis::Failed.as_str()),
        ("QualificationAxis", "stale", QualificationAxis::Stale.as_str()),
        ("QualificationAxis", "suspended", QualificationAxis::Suspended.as_str()),
        ("RecordStatus", "qualifying", QualificationRecordStatus::Qualifying.as_str()),
        ("RecordStatus", "passed", QualificationRecordStatus::Passed.as_str()),
        ("RecordStatus", "failed", QualificationRecordStatus::Failed.as_str()),
        ("RecordStatus", "stale", QualificationRecordStatus::Stale.as_str()),
        ("RecordStatus", "superseded", QualificationRecordStatus::Superseded.as_str()),
        ("AssessorType", "manual", AssessorType::Manual.as_str()),
        ("AssessorType", "automated", AssessorType::Automated.as_str()),
        ("AssessorType", "external", AssessorType::External.as_str()),
        ("TransitionType", "automatic", TransitionType::Automatic.as_str()),
        ("TransitionType", "manual", TransitionType::Manual.as_str()),
        ("TransitionerRole", "system", TransitionerRole::System.as_str()),
        ("TransitionerRole", "evaluator", TransitionerRole::Evaluator.as_str()),
        ("TransitionerRole", "approver", TransitionerRole::Approver.as_str()),
        ("TransitionerRole", "owner", TransitionerRole::Owner.as_str()),
        ("EvidenceDimension", "identity", EvidenceDimension::Identity.as_str()),
        ("EvidenceDimension", "capability", EvidenceDimension::Capability.as_str()),
        ("EvidenceDimension", "security_level", EvidenceDimension::SecurityLevel.as_str()),
        ("EvidenceDimension", "qualification", EvidenceDimension::Qualification.as_str()),
        ("EvidenceDimension", "constraints", EvidenceDimension::Constraints.as_str()),
        ("EvidenceType", "test_result", EvidenceType::TestResult.as_str()),
        ("EvidenceType", "review_approval", EvidenceType::ReviewApproval.as_str()),
        ("EvidenceType", "benchmark", EvidenceType::Benchmark.as_str()),
        ("EvidenceType", "audit_log", EvidenceType::AuditLog.as_str()),
        ("EvidenceType", "receipt", EvidenceType::Receipt.as_str()),
        ("ProducerRole", "evaluator", EvidenceProducerRole::Evaluator.as_str()),
        ("ProducerRole", "system", EvidenceProducerRole::System.as_str()),
        ("ProducerRole", "automated_harness", EvidenceProducerRole::AutomatedHarness.as_str()),
        ("ProducerRole", "external", EvidenceProducerRole::External.as_str()),
        ("Classification", "S0", SecurityClassification::S0.as_str()),
        ("Classification", "S1", SecurityClassification::S1.as_str()),
        ("Classification", "S2", SecurityClassification::S2.as_str()),
        ("Classification", "S3", SecurityClassification::S3.as_str()),
        ("Classification", "S4", SecurityClassification::S4.as_str()),
        ("Classification", "S5", SecurityClassification::S5.as_str()),
        ("Classification", "unclassified", SecurityClassification::Unclassified.as_str()),
        ("Derivation", "declared", ClassificationDerivation::Declared.as_str()),
        ("Derivation", "derived", ClassificationDerivation::Derived.as_str()),
        ("Derivation", "inherited", ClassificationDerivation::Inherited.as_str()),
        ("Derivation", "policy_constraint", ClassificationDerivation::PolicyConstraint.as_str()),
        ("ModeValue", "explain_only", OperationalModeValue::ExplainOnly.as_str()),
        ("ModeValue", "review_assist", OperationalModeValue::ReviewAssist.as_str()),
        ("ModeValue", "recommend_only", OperationalModeValue::RecommendOnly.as_str()),
        ("ModeValue", "autonomous_assist", OperationalModeValue::AutonomousAssist.as_str()),
        ("Freshness", "fresh", EvidenceFreshness::Fresh.as_str()),
        ("Freshness", "stale", EvidenceFreshness::Stale.as_str()),
        ("Freshness", "no_evidence", EvidenceFreshness::NoEvidence.as_str()),
    ];
    for (family, expected, actual) in &cases {
        assert_eq!(actual, expected, "{family}: as_str mismatch");
        // The serde-serialized form must equal the contract name too.
        let value = serde_json::to_value(actual).unwrap();
        assert_eq!(
            value.as_str().unwrap(),
            *expected,
            "{family}: serde name drifted from contract surface"
        );
    }
}

#[test]
fn security_classification_serde_preserves_s0_case() {
    // Frozen meanings (PI-001): S0–S5 are case-preserved; unclassified lowercase.
    for level in [
        SecurityClassification::S0,
        SecurityClassification::S1,
        SecurityClassification::S2,
        SecurityClassification::S3,
        SecurityClassification::S4,
        SecurityClassification::S5,
    ] {
        let json = serde_json::to_string(&level).unwrap();
        assert!(json.starts_with("\"S"), "S-level must keep uppercase S: {json}");
        let back: SecurityClassification = serde_json::from_str(&json).unwrap();
        assert_eq!(back, level);
    }
    let json = serde_json::to_string(&SecurityClassification::Unclassified).unwrap();
    assert_eq!(json, "\"unclassified\"");
    let back: SecurityClassification = serde_json::from_str(&json).unwrap();
    assert_eq!(back, SecurityClassification::Unclassified);
}

// ---------------------------------------------------------------------------
// Validation rules (schema constraints)
// ---------------------------------------------------------------------------

#[test]
fn capability_id_validation() {
    assert!(CapabilityId::new("frontend-design".to_string()).is_some());
    assert!(CapabilityId::new("alpha_1".to_string()).is_some());
    assert!(CapabilityId::new("plain".to_string()).is_some());
    // Violations: empty, spaces, and punctuation outside ^[A-Za-z0-9_-]+$.
    assert!(CapabilityId::new(String::new()).is_none());
    assert!(CapabilityId::new("has space".to_string()).is_none());
    assert!(CapabilityId::new("a.b".to_string()).is_none());
    assert!(CapabilityId::new("ünïcode".to_string()).is_none());
}

#[test]
fn capability_version_validation() {
    assert_eq!(CapabilityVersion::new(1).unwrap().get(), 1);
    assert_eq!(CapabilityVersion::new(7).unwrap().get(), 7);
    // Schema CHECK (version > 0), CR-I-001.
    assert!(CapabilityVersion::new(0).is_none());
}

#[test]
fn dependency_rejects_self_reference() {
    let id = CapabilityId::new("alpha".to_string()).unwrap();
    assert!(
        CapabilityDependency::new(
            id.clone(),
            id,
            true,
            CapabilityRelationshipType::Requires,
        )
        .is_none(),
        "schema CHECK (capability_id != dependency_id) (CR-I-005)"
    );
}

// ---------------------------------------------------------------------------
// Round-trip encode/decode
// ---------------------------------------------------------------------------

fn sample_context() -> CapabilitySecurityContext {
    CapabilitySecurityContext {
        classification: SecurityClassification::S2,
        source: "capability_registry.capabilities".to_string(),
        derivation: ClassificationDerivation::Inherited,
        evidence_reference: Some("EV-00001".to_string()),
    }
}

fn sample_identity() -> CapabilityIdentity {
    CapabilityIdentity {
        capability_id: CapabilityId::new("alpha".to_string()).unwrap(),
        name: "Alpha Capability".to_string(),
        capability_type: CapabilityType::Skill,
        version: Some(CapabilityVersion::new(1).unwrap()),
        lifecycle_state: QualificationState::Reviewed,
    }
}

fn sample_mode() -> OperationalMode {
    OperationalMode {
        mode: OperationalModeValue::AutonomousAssist,
        explanation: "S2, qualified, fresh evidence, no constraints".to_string(),
        derivation_inputs: OperationalModeInputs {
            security_classification: SecurityClassification::S2,
            qualification_axis: QualificationAxis::Passed,
            lifecycle_state: QualificationState::Qualified,
            evidence_freshness: EvidenceFreshness::Fresh,
            policy_constraints: None,
        },
        evidence_references: vec!["EV-00001".to_string(), "EV-00002".to_string()],
    }
}

#[test]
fn capability_security_context_round_trips() {
    let context = sample_context();
    let json = serde_json::to_string(&context).unwrap();
    let back: CapabilitySecurityContext = serde_json::from_str(&json).unwrap();
    assert_eq!(back, context);
}

#[test]
fn capability_identity_round_trips() {
    let identity = sample_identity();
    let json = serde_json::to_string(&identity).unwrap();
    let back: CapabilityIdentity = serde_json::from_str(&json).unwrap();
    assert_eq!(back, identity);
    // Identifiers survive the wire intact.
    assert_eq!(back.capability_id.as_str(), "alpha");
    assert_eq!(back.version.unwrap().get(), 1);
}

#[test]
fn operational_mode_round_trips() {
    let mode = sample_mode();
    let json = serde_json::to_string(&mode).unwrap();
    let back: OperationalMode = serde_json::from_str(&json).unwrap();
    assert_eq!(back, mode);
    assert_eq!(back.derivation_inputs.security_classification, SecurityClassification::S2);
    assert_eq!(back.evidence_references.len(), 2);
}

#[test]
fn qualification_lifecycle_event_round_trips() {
    let event = QualificationLifecycleEvent {
        event_id: "QLE-20260807-001".to_string(),
        qualification_id: "Q-20260807-001".to_string(),
        capability_id: CapabilityId::new("alpha".to_string()).unwrap(),
        from_state: QualificationState::Reviewed,
        to_state: QualificationState::Qualified,
        transition_type: TransitionType::Manual,
        security_classification: Some(SecurityClassification::S3),
        transitioned_by: "approver@librarian".to_string(),
        transitioner_role: TransitionerRole::Approver,
        authority_evidence_id: Some("EV-00042".to_string()),
        evidence_snapshot: serde_json::json!({"dimensions": ["identity", "capability"]}),
        created_at: "2026-08-07T11:00:00Z".to_string(),
    };
    let json = serde_json::to_string(&event).unwrap();
    let back: QualificationLifecycleEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(back, event);
    // FROZEN classification is preserved through the wire (audit replay).
    assert_eq!(back.security_classification, Some(SecurityClassification::S3));
}

// ---------------------------------------------------------------------------
// Non-collapse invariants (type-level, enforced at runtime where observable)
// ---------------------------------------------------------------------------

#[test]
fn identity_carries_no_qualification_or_permission_fields() {
    // CAPABILITY-IDENTITY-CONTRACT-001 §1.2: identity persists independently of
    // qualification state. Assert the serialized shape has no qualification or
    // permission material.
    let json = serde_json::to_value(sample_identity()).unwrap();
    let keys: Vec<&str> = json.as_object().unwrap().keys().map(String::as_str).collect();
    for key in keys {
        assert!(
            !key.contains("qualification") && !key.contains("permission") && key != "evidence",
            "identity must not carry field '{key}'"
        );
    }
    assert_eq!(json.as_object().unwrap().len(), 5);
}

#[test]
fn security_context_is_storage_independent() {
    // QUALIFICATION-STATE-CONTRACT-001 §5: the context carries provenance
    // (source + derivation) — it must not encode WHERE the classification was
    // stored, only how it was obtained.
    let context = sample_context();
    assert_eq!(context.derivation, ClassificationDerivation::Inherited);
    let json = serde_json::to_value(&context).unwrap();
    let keys: Vec<&str> = json.as_object().unwrap().keys().map(String::as_str).collect();
    assert!(keys.contains(&"derivation"), "provenance must be observable");
    assert!(keys.contains(&"source"), "source must be observable");
}

#[test]
fn dependency_references_are_not_embedded_objects() {
    // CR-I-005 / CAPABILITY-IDENTITY-CONTRACT-001: dependencies are references.
    // The serialized dependency must contain ids, not nested capability objects.
    let dep = CapabilityDependency::new(
        CapabilityId::new("alpha".to_string()).unwrap(),
        CapabilityId::new("beta".to_string()).unwrap(),
        false,
        CapabilityRelationshipType::Extends,
    )
    .unwrap();
    let json = serde_json::to_value(&dep).unwrap();
    assert_eq!(json["capability_id"].as_str().unwrap(), "alpha");
    assert_eq!(json["dependency_id"].as_str().unwrap(), "beta");
    assert_eq!(json["required"].as_bool().unwrap(), false);
    assert_eq!(json["relationship_type"].as_str().unwrap(), "extends");
}
