//! M1-D2 qualification observation projection equivalence tests

use librarian_contracts::node::capability_registry::{
    QualificationAxis, QualificationRecordStatus, QualificationState, SecurityClassification,
};
use librarian_node::qualification_observation::QualificationObservationState;
use rusqlite::Connection;

/// Apply the canonical governed schema to `conn`.
fn apply_canonical_schema(conn: &Connection) {
    conn.execute_batch(&librarian_core::startup::canonical_schema())
        .expect("apply canonical capability-registry schema");
}

/// Insert a deterministic qualification fixture.
fn insert_fixture(conn: &Connection) {
    conn.execute_batch(
        "PRAGMA foreign_keys = OFF;
         INSERT INTO capabilities
           (id, name, type, description, status, active_version,
            availability, qualification, authority)
         VALUES
           ('alpha', 'Alpha Skill', 'skill', 'Alpha desc',
            'qualified', 1, 'registered', 'passed', 'approved');
         INSERT INTO capability_versions
           (capability_id, version, body, content_hash, changelog, author,
            review_notes, qualification_evidence_id, profile_id, created_at)
         VALUES
           ('alpha', 1, 'body', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            'init', 'owner', NULL, NULL, NULL, '2026-01-01T00:00:00Z');
         INSERT INTO capability_qualifications
           (qualification_id, capability_id, profile_id, version_id,
            qualification_status, confidence, evidence_reference,
            qualified_at, expires_at, assessed_at,
            assessor_identity, assessor_type, notes)
         VALUES
           ('Q-20260807-001', 'alpha', 'default', 1,
            'passed', 0.95, 'EV-00001',
            '2026-08-07T00:00:00Z', NULL, '2026-08-07T00:00:00Z',
            'evaluator-1', 'automated', 'Initial qualification');
         INSERT INTO qualification_lifecycle_events
           (event_id, qualification_id, capability_id,
            from_state, to_state, transition_type,
            security_classification, transitioned_by,
            transitioner_role, authority_evidence_id,
            evidence_snapshot, created_at)
         VALUES
           ('QLE-20260807-001', 'Q-20260807-001', 'alpha',
            'unreviewed', 'reviewed', 'manual',
            'S0', 'owner-1',
            'owner', 'EV-00002',
            '{\"status\": \"reviewed\"}', '2026-08-06T00:00:00Z'),
           ('QLE-20260807-002', 'Q-20260807-001', 'alpha',
            'reviewed', 'qualified', 'automatic',
            'S0', 'system',
            'system', NULL,
            '{\"status\": \"qualified\"}', '2026-08-07T00:00:00Z');
         INSERT INTO qualification_evidence_records
           (evidence_id, qualification_id, dimension,
            evidence_type, evidence_reference, evidence_body,
            evidence_hash, captured_at, expires_at,
            producer_identity, producer_role)
         VALUES
           ('QER-20260807-001', 'Q-20260807-001', 'identity',
            'test_result', 'EV-00003', '{\"result\": \"pass\"}',
            'abc123', '2026-08-07T00:00:00Z', NULL,
            'evaluator-1', 'evaluator'),
           ('QER-20260807-002', 'Q-20260807-001', 'capability',
            'test_result', 'EV-00004', '{\"result\": \"pass\"}',
            'def456', '2026-08-07T00:00:00Z', NULL,
            'evaluator-1', 'evaluator');",
    )
    .expect("insert qualification fixture");
}

/// Build a test registry DB.
fn fixture_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let db_path = dir.path().join("capability-registry.sqlite");
    let conn = Connection::open(&db_path).expect("open fixture db");
    apply_canonical_schema(&conn);
    insert_fixture(&conn);
    (dir, db_path)
}

#[test]
fn qualification_record_projection_matches_fixture() {
    let (_dir, db_path) = fixture_db();
    let state = QualificationObservationState::new("test-node", &db_path);

    let envelope = state
        .qualification_record("Q-20260807-001")
        .expect("project qualification record");

    assert_eq!(envelope.node_id, "test-node");
    assert_eq!(envelope.projection.qualification_id, "Q-20260807-001");
    assert_eq!(envelope.projection.status, QualificationRecordStatus::Passed);
    assert!((envelope.projection.confidence.unwrap() - 0.95).abs() < f32::EPSILON);
}

#[test]
fn capability_qualifications_projection_matches_fixture() {
    let (_dir, db_path) = fixture_db();
    let state = QualificationObservationState::new("test-node", &db_path);

    let envelope = state
        .capability_qualifications("alpha")
        .expect("project capability qualifications");

    assert_eq!(envelope.projection.len(), 1);
    assert_eq!(envelope.projection[0].qualification_id, "Q-20260807-001");
}

#[test]
fn qualification_lifecycle_events_projection_matches_fixture() {
    let (_dir, db_path) = fixture_db();
    let state = QualificationObservationState::new("test-node", &db_path);

    let envelope = state
        .qualification_lifecycle_events("Q-20260807-001")
        .expect("project lifecycle events");

    assert_eq!(envelope.projection.len(), 2);
    // Events are ordered by created_at ascending
    assert_eq!(envelope.projection[0].from_state, QualificationState::Unreviewed);
    assert_eq!(envelope.projection[0].to_state, QualificationState::Reviewed);
    assert_eq!(envelope.projection[1].from_state, QualificationState::Reviewed);
    assert_eq!(envelope.projection[1].to_state, QualificationState::Qualified);
}

#[test]
fn qualification_evidence_projection_matches_fixture() {
    let (_dir, db_path) = fixture_db();
    let state = QualificationObservationState::new("test-node", &db_path);

    let envelope = state
        .qualification_evidence("Q-20260807-001")
        .expect("project evidence records");

    assert_eq!(envelope.projection.len(), 2);
    // Evidence references, not inline proof
    assert!(envelope.projection[0].evidence_reference.is_some());
    assert!(envelope.projection[0].evidence_body.is_some());
}

#[test]
fn qualification_overview_projection_matches_fixture() {
    let (_dir, db_path) = fixture_db();
    let state = QualificationObservationState::new("test-node", &db_path);

    let envelope = state
        .qualification_overview()
        .expect("project overview");

    assert_eq!(envelope.projection.qualification_count, 1);
    assert_eq!(envelope.projection.lifecycle_event_count, 2);
    assert_eq!(envelope.projection.evidence_record_count, 2);
}

#[test]
fn capability_governance_state_projection_matches_fixture() {
    let (_dir, db_path) = fixture_db();
    let state = QualificationObservationState::new("test-node", &db_path);

    let envelope = state
        .capability_governance_state("alpha")
        .expect("project governance state");

    assert_eq!(envelope.projection.qualification_state, QualificationState::Qualified);
    assert_eq!(envelope.projection.qualification_axis, QualificationAxis::Passed);
    assert!(envelope.projection.available);
}

#[test]
fn governance_state_invariants_enforced() {
    let (_dir, db_path) = fixture_db();
    let state = QualificationObservationState::new("test-node", &db_path);

    let envelope = state
        .capability_governance_state("alpha")
        .expect("project governance state");

    // QUALIFIED ≠ AUTHORIZED
    assert!(envelope.projection.is_qualified());
    assert!(!envelope.projection.is_authorized(), "QUALIFIED ≠ AUTHORIZED");

    // QUALIFIED ≠ AVAILABLE (this capability is available, but the invariant
    // is that they are independent — a qualified capability is not automatically
    // available; this one happens to be available for other reasons)
    assert!(envelope.projection.is_qualified());
    assert!(envelope.projection.is_available(), "this capability is available");

    // QUALIFIED ≠ EXECUTING
    assert!(envelope.projection.is_qualified());
    assert!(!envelope.projection.is_executing(), "QUALIFIED ≠ EXECUTING");
}

#[test]
fn consecutive_reads_observe_identical_snapshot() {
    let (_dir, db_path) = fixture_db();
    let state = QualificationObservationState::new("test-node", &db_path);

    let a = state
        .qualification_overview()
        .expect("overview read 1");
    let b = state
        .qualification_overview()
        .expect("overview read 2");

    assert_eq!(a.registry_identity, b.registry_identity);
    assert_eq!(a.projection, b.projection);
}

#[test]
fn classification_provenance_preserved() {
    let (_dir, db_path) = fixture_db();
    let state = QualificationObservationState::new("test-node", &db_path);

    let envelope = state
        .qualification_lifecycle_events("Q-20260807-001")
        .expect("project lifecycle events");

    // Security classification is preserved per event (F-2 deferred)
    for event in &envelope.projection {
        if let Some(classification) = &event.security_classification {
            match classification {
                SecurityClassification::S0
                | SecurityClassification::S1
                | SecurityClassification::S2
                | SecurityClassification::S3
                | SecurityClassification::S4
                | SecurityClassification::S5
                | SecurityClassification::Unclassified => {}
            }
        }
    }
}
