//! M1-B registry observation projection equivalence tests
//! (`RUNTIME-REGISTRY-OBSERVATION-CONTRACT-001`, Canonical `232edfb`).
//!
//! Evidence target: **Governed Registry Snapshot ≡ Rust Projection Types**.
//! These tests apply the canonical Phase 2 + Phase 3 schema
//! (`librarian_core::startup::canonical_schema`) to a temp SQLite database,
//! insert a deterministic fixture, and verify the module projections match the
//! fixture field-for-field with the source-map pinned ordering, identity, and
//! fail-closed behavior.
//!
//! Verification hooks (work order §4):
//! - consistent snapshot: two consecutive reads of a projection observe
//!   identical facts; identity read and payload read share the snapshot
//! - no write path: the registry is byte-identical after all projections
//! - fail-closed identity: absent/empty/missing-key meta fails the projection
//! - fail-closed enums: unknown persisted values fail, never coerce
//! - determinism: same registry state → same payload bytes (excluding the
//!   variable `projection_observed_at`)

use std::path::{Path, PathBuf};

use librarian_contracts::node::{
    AuthorityAxis, AvailabilityAxis, CapabilityDependency, CapabilityId, CapabilityObservation,
    CapabilityRelationshipType, CapabilityType, CapabilityTypeDefinition, CapabilityVersion,
    CapabilityVersionRecord, QualificationAxis, QualificationState, RegistryOverview, TypeCategory,
};
use librarian_node::registry_observation::RegistryObservationState;
use rusqlite::Connection;
use sha2::{Digest, Sha256};

/// Apply the canonical governed schema (Phase 2 + Phase 3) to `conn`.
fn apply_canonical_schema(conn: &Connection) {
    conn.execute_batch(&librarian_core::startup::canonical_schema())
        .expect("apply canonical capability-registry schema");
}

/// Deterministic fixture over the canonical schema.
///
/// 3 capabilities (alpha/beta/gamma), 4 versions (gamma has v1+v2), 2
/// dependencies, 2 taxonomy rows. `created_at` values are explicit so version
/// payloads are fully deterministic.
fn insert_fixture(conn: &Connection) {
    conn.execute_batch(
        "INSERT INTO capabilities
           (id, name, type, description, status, active_version,
            availability, qualification, authority)
         VALUES
           ('alpha', 'Alpha Capability', 'skill', 'Alpha description',
            'reviewed', 1, 'registered', 'passed', 'approved'),
           ('beta',  'Beta Workflow',     'workflow', 'Beta description',
            'unreviewed', NULL, 'discovered', 'not_tested', 'not_submitted'),
           ('gamma', 'Gamma Policy',      'policy', 'Gamma description',
            'qualified', 1, 'disabled', 'failed', 'rejected');

         INSERT INTO capability_versions
           (capability_id, version, body, content_hash, changelog, author,
            review_notes, qualification_evidence_id, profile_id, created_at)
         VALUES
           ('alpha', 1, 'alpha-body', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            'v1 init', 'owner', NULL, NULL, NULL, '2026-01-01T00:00:00Z'),
           ('beta',  1, 'beta-body',  'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
            NULL, NULL, 'beta reviewed', NULL, NULL, '2026-01-02T00:00:00Z'),
           ('gamma', 1, 'gamma-v1',   'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc',
            NULL, NULL, NULL, 'EV-00001', NULL, '2026-02-01T00:00:00Z'),
           ('gamma', 2, 'gamma-v2',   'dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd',
            'v2 changes', 'owner', NULL, NULL, 'PROFILE-1', '2026-02-02T00:00:00Z');

         INSERT INTO capability_dependencies
           (capability_id, dependency_id, required, relationship_type)
         VALUES
           ('alpha', 'beta', 1, 'requires'),
           ('gamma', 'alpha', 0, 'extends');

         INSERT INTO capability_types
           (capability_type_id, name, description, category,
            default_profile_id, default_policy_id)
         VALUES
           ('design-review', 'Design Review', 'Human design review workflow',
            'standard', NULL, NULL),
           ('system-audit', 'System Audit', 'Audits system invariants',
            'system', NULL, NULL);",
    )
    .expect("insert deterministic fixture");
}

/// Build a fixture registry DB; returns its path.
///
/// The fixture WRITER disables FK enforcement: rusqlite 0.31 defaults
/// `PRAGMA foreign_keys = ON`, and the frozen Phase 3 schema contains a
/// malformed FK — `capability_qualifications (capability_id, version_id)
/// REFERENCES capability_versions (capability_id, version_id)` — where
/// `capability_versions` has no `version_id` column (its version column is
/// `version`). SQLite reports this "foreign key mismatch" at DML time on the
/// affected FK path, blocking fixture rows that the governed registry may
/// legitimately hold. The projection module is read-only and unaffected; the
/// fixture simply mirrors the snapshot the governed writer produces.
fn fixture_db() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let db_path = dir.path().join("capability-registry.sqlite");
    let conn = Connection::open(&db_path).expect("open fixture db");
    conn.execute_batch("PRAGMA foreign_keys = OFF;").expect("disable FK enforcement for fixture writer");
    apply_canonical_schema(&conn);
    insert_fixture(&conn);
    (dir, db_path)
}

/// Open a separate connection to re-read fixture state (verification only).
fn open_reader(path: &Path) -> Connection {
    Connection::open(path).expect("open reader connection")
}

/// Independently recompute the registry identity the way the module must:
/// SHA-256 over the SORTED `(key, value)` pairs of `capability_registry_meta`,
/// serialized as `key=value\n` lines (source map §3, owner decision 5).
fn independent_identity(path: &Path) -> String {
    let conn = open_reader(path);
    let mut stmt = conn
        .prepare("SELECT key, value FROM capability_registry_meta")
        .expect("prepare meta read");
    let mut rows: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("query meta")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("collect meta");
    rows.sort();
    let mut hasher = Sha256::new();
    for (key, value) in &rows {
        hasher.update(key.as_bytes());
        hasher.update(b"=");
        hasher.update(value.as_bytes());
        hasher.update(b"\n");
    }
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn capability_projection_matches_fixture_field_for_field() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    let envelope = state.capability("alpha").expect("project alpha");

    // Envelope provenance.
    assert_eq!(envelope.node_id, "test-node");
    assert_eq!(envelope.registry_identity.source, "capability_registry_meta");
    assert_eq!(
        envelope.registry_identity.derivation,
        "sha256 over sorted capability_registry_meta key/value pairs"
    );
    assert_eq!(
        envelope.registry_identity.value,
        independent_identity(&db_path),
        "registry identity must be the deterministic content-derived SHA-256"
    );
    assert!(
        envelope.projection_observed_at.ends_with('Z'),
        "RFC 3339 UTC timestamp: {}",
        envelope.projection_observed_at
    );

    // Payload: identity (5 fields) + assurance axes (3 fields), exactly.
    assert_eq!(
        envelope.projection,
        CapabilityObservation {
            capability_id: CapabilityId::new("alpha".to_string()).unwrap(),
            name: "Alpha Capability".to_string(),
            capability_type: CapabilityType::Skill,
            version: Some(CapabilityVersion::new(1).unwrap()),
            lifecycle_state: QualificationState::Reviewed,
            availability: AvailabilityAxis::Registered,
            qualification: QualificationAxis::Passed,
            authority: AuthorityAxis::Approved,
        }
    );

    // NULL active_version projects as `version: null`.
    let beta = state.capability("beta").expect("project beta");
    assert_eq!(beta.projection.version, None);
}

#[test]
fn versions_projection_is_ascending_and_exposes_hash_never_body() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    let envelope = state.capability_versions("gamma").expect("project versions");

    // Owner decision 1: the serialized projection never contains `body`.
    let json = serde_json::to_value(&envelope).expect("serialize envelope");
    assert!(json["projection"][0].get("body").is_none(), "body must not be exposed");
    assert_eq!(json["projection"][0]["content_hash"].as_str().unwrap(), "c".repeat(64));

    // Ascending by integer version (CR-I-001, contract §3).
    let versions = envelope.projection;
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version.get(), 1);
    assert_eq!(versions[1].version.get(), 2);
    assert_eq!(
        versions,
        vec![
            CapabilityVersionRecord {
                capability_id: CapabilityId::new("gamma".to_string()).unwrap(),
                version: CapabilityVersion::new(1).unwrap(),
                content_hash: "c".repeat(64),
                changelog: None,
                author: None,
                review_notes: None,
                qualification_evidence_id: Some("EV-00001".to_string()),
                profile_id: None,
                created_at: "2026-02-01T00:00:00Z".to_string(),
            },
            CapabilityVersionRecord {
                capability_id: CapabilityId::new("gamma".to_string()).unwrap(),
                version: CapabilityVersion::new(2).unwrap(),
                content_hash: "d".repeat(64),
                changelog: Some("v2 changes".to_string()),
                author: Some("owner".to_string()),
                review_notes: None,
                qualification_evidence_id: None,
                profile_id: Some("PROFILE-1".to_string()),
                created_at: "2026-02-02T00:00:00Z".to_string(),
            },
        ]
    );
}

#[test]
fn dependencies_projection_is_locked_verbatim_and_ordered() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    let alpha_deps = state
        .capability_dependencies("alpha")
        .expect("project alpha dependencies")
        .projection;
    assert_eq!(
        alpha_deps,
        vec![CapabilityDependency {
            capability_id: CapabilityId::new("alpha".to_string()).unwrap(),
            dependency_id: CapabilityId::new("beta".to_string()).unwrap(),
            required: true,
            relationship_type: CapabilityRelationshipType::Requires,
        }]
    );

    // `created_at` is NOT carried (owner decision 2): locked surface, no growth.
    let json = serde_json::to_value(&state
        .capability_dependencies("alpha")
        .expect("serialize deps")
        .projection)
        .expect("serialize");
    assert!(json[0].get("created_at").is_none(), "created_at must not be carried");

    // Ordered by (capability_id, dependency_id).
    let gamma_deps = state
        .capability_dependencies("gamma")
        .expect("project gamma dependencies")
        .projection;
    assert_eq!(gamma_deps.len(), 1);
    assert_eq!(gamma_deps[0].dependency_id.as_str(), "alpha");
    assert!(!gamma_deps[0].required);
    assert_eq!(gamma_deps[0].relationship_type, CapabilityRelationshipType::Extends);
}

#[test]
fn type_taxonomy_projection_is_ordered_and_not_the_type_enum() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    let types = state.capability_types().expect("project types").projection;

    // Ordered by capability_type_id (contract §3).
    let ids: Vec<&str> = types.iter().map(|t| t.capability_type_id.as_str()).collect();
    assert_eq!(ids, vec!["design-review", "system-audit"]);

    assert_eq!(
        types,
        vec![
            CapabilityTypeDefinition {
                capability_type_id: "design-review".to_string(),
                name: "Design Review".to_string(),
                description: "Human design review workflow".to_string(),
                category: TypeCategory::Standard,
                default_profile_id: None,
                default_policy_id: None,
            },
            CapabilityTypeDefinition {
                capability_type_id: "system-audit".to_string(),
                name: "System Audit".to_string(),
                description: "Audits system invariants".to_string(),
                category: TypeCategory::System,
                default_profile_id: None,
                default_policy_id: None,
            },
        ]
    );

    // Naming guard: taxonomy rows are NOT CapabilityType enum values.
    assert_ne!(types[0].capability_type_id, CapabilityType::Skill.as_str());
    assert_ne!(types[1].capability_type_id, CapabilityType::Workflow.as_str());
}

#[test]
fn overview_counts_and_fixed_groups_match_fixture() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    let overview = state.registry_overview().expect("project overview").projection;

    assert_eq!(
        overview,
        RegistryOverview {
            capability_count: 3,
            by_status: vec![
                ("unreviewed".to_string(), 1),
                ("reviewed".to_string(), 1),
                ("qualified".to_string(), 1),
                ("deprecated".to_string(), 0),
                ("revoked".to_string(), 0),
            ],
            by_availability: vec![
                ("discovered".to_string(), 1),
                ("registered".to_string(), 1),
                ("disabled".to_string(), 1),
                ("removed".to_string(), 0),
            ],
            by_qualification: vec![
                ("not_tested".to_string(), 1),
                ("qualifying".to_string(), 0),
                ("passed".to_string(), 1),
                ("failed".to_string(), 1),
                ("stale".to_string(), 0),
                ("suspended".to_string(), 0),
            ],
            by_authority: vec![
                ("not_submitted".to_string(), 1),
                ("pending_review".to_string(), 0),
                ("approved".to_string(), 1),
                ("rejected".to_string(), 1),
                ("revoked".to_string(), 0),
            ],
            by_type: vec![
                ("skill".to_string(), 1),
                ("workflow".to_string(), 1),
                ("policy".to_string(), 1),
                ("validator".to_string(), 0),
                ("template".to_string(), 0),
            ],
            version_count: 4,
            dependency_count: 2,
            dependency_by_relationship: vec![
                ("requires".to_string(), 1),
                ("extends".to_string(), 1),
                ("refines".to_string(), 0),
                ("conflicts".to_string(), 0),
            ],
            type_count: 2,
            types_by_category: vec![
                ("standard".to_string(), 1),
                ("system".to_string(), 1),
                ("experimental".to_string(), 0),
                ("external".to_string(), 0),
            ],
        }
    );

    // No M1-C / M1-D / F-2 facts in the serialized overview (check top-level
    // keys only — group VALUES legitimately include enum names like "policy").
    let json = serde_json::to_value(&overview).expect("serialize overview");
    let keys = json
        .as_object()
        .expect("overview object")
        .keys()
        .map(|k| k.as_str().to_string())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        keys,
        vec![
            "by_authority",
            "by_availability",
            "by_qualification",
            "by_status",
            "by_type",
            "capability_count",
            "dependency_by_relationship",
            "dependency_count",
            "type_count",
            "types_by_category",
            "version_count",
        ]
        .into_iter()
        .map(|k| k.to_string())
        .collect::<std::collections::BTreeSet<_>>(),
        "overview key set is exactly the locked RegistryOverview surface"
    );
    for forbidden in [
        "qualification_count",
        "evidence_count",
        "policy_count",
        "operational_mode",
        "resolvability",
        "security_classification",
    ] {
        assert!(
            !keys.contains(forbidden),
            "overview must not carry fact kind {forbidden}"
        );
    }
}

#[test]
fn consecutive_reads_observe_identical_snapshot_facts() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    // Two consecutive reads of the same projection observe identical facts
    // (work order §4 verification hook).
    let a = state.registry_overview().expect("overview read 1");
    let b = state.registry_overview().expect("overview read 2");
    assert_eq!(a.registry_identity, b.registry_identity);
    assert_eq!(a.projection, b.projection, "same registry state -> same payload");

    // Identity and payload share the snapshot: after a governed change, BOTH
    // move together to the new state. Identity derives solely from
    // `capability_registry_meta`, so the governed change must touch that
    // table; the payload change comes from `capabilities`.
    let conn = open_reader(&db_path);
    conn.execute_batch(
        "UPDATE capabilities SET availability = 'removed' WHERE id = 'alpha';
         UPDATE capability_registry_meta SET value = '2026-02-02T00:00:00Z' WHERE key = 'created_at';",
    )
    .expect("governed change");
    let c = state.registry_overview().expect("overview read 3");
    assert_ne!(a.registry_identity, c.registry_identity, "identity tracks registry state");
    let removed = c
        .projection
        .by_availability
        .iter()
        .find(|(name, _)| name == "removed")
        .map(|(_, n)| *n)
        .unwrap();
    assert_eq!(removed, 1, "payload tracks the same new state");
}

#[test]
fn payload_is_deterministic_except_observed_at() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    let a = state.capability("alpha").expect("read 1");
    let b = state.capability("alpha").expect("read 2");
    assert_eq!(a.projection, b.projection);
    assert_eq!(a.registry_identity, b.registry_identity);

    // Serialized payload bytes identical across runs (determinism, §3.1).
    let mut json_a = serde_json::to_value(&a).expect("serialize a");
    let mut json_b = serde_json::to_value(&b).expect("serialize b");
    json_a["projection_observed_at"] = "X".into();
    json_b["projection_observed_at"] = "X".into();
    assert_eq!(json_a, json_b, "only projection_observed_at may vary");
}

#[test]
fn fail_closed_unknown_axis_value() {
    let (_dir, db_path) = fixture_db();
    // Schema has no CHECK on the axis columns — a drift value must fail closed
    // (owner decision 3), never coerce.
    let conn = open_reader(&db_path);
    conn.execute(
        "UPDATE capabilities SET availability = 'bogus' WHERE id = 'alpha'",
        [],
    )
    .expect("inject drift");

    let state = RegistryObservationState::new("test-node", &db_path);
    let err = state.capability("alpha").expect_err("must fail closed");
    assert!(
        err.to_string().contains("fail-closed invariant violation"),
        "explicit invariant-violation error: {err}"
    );

    // Overview group counts also fail closed on an unknown group value.
    let err = state.registry_overview().expect_err("overview must fail closed");
    assert!(err.to_string().contains("fail-closed invariant violation"), "{err}");
}

#[test]
fn fail_closed_unrepresentable_active_version() {
    let (_dir, db_path) = fixture_db();
    let conn = open_reader(&db_path);
    // 0 is unrepresentable (CHECK version > 0 makes it non-referenceable, CR-I-003).
    conn.execute(
        "UPDATE capabilities SET active_version = 0 WHERE id = 'alpha'",
        [],
    )
    .expect("inject active_version 0");

    let state = RegistryObservationState::new("test-node", &db_path);
    let err = state.capability("alpha").expect_err("must fail closed");
    assert!(err.to_string().contains("fail-closed invariant violation"), "{err}");
}

#[test]
fn fail_closed_missing_capability() {
    let (_dir, db_path) = fixture_db();
    let state = RegistryObservationState::new("test-node", &db_path);

    let err = state.capability("nope").expect_err("missing capability must fail");
    assert!(err.to_string().contains("not found in registry"), "{err}");
    let err = state
        .capability_versions("nope")
        .expect_err("missing capability must fail");
    assert!(err.to_string().contains("not found in registry"), "{err}");
    let err = state
        .capability_dependencies("nope")
        .expect_err("missing capability must fail");
    assert!(err.to_string().contains("not found in registry"), "{err}");
}

#[test]
fn fail_closed_registry_identity_absent_or_empty() {
    let (_dir, db_path) = fixture_db();
    let conn = open_reader(&db_path);

    // Absent: drop the meta table entirely.
    conn.execute_batch("DROP TABLE capability_registry_meta;").expect("drop meta");
    let state = RegistryObservationState::new("test-node", &db_path);
    let err = state.capability("alpha").expect_err("absent meta must fail closed");
    assert!(err.to_string().contains("registry identity fail-closed"), "{err}");

    // Empty: recreate without rows.
    conn.execute_batch(
        "CREATE TABLE capability_registry_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )
    .expect("recreate meta");
    let err = state.registry_overview().expect_err("empty meta must fail closed");
    assert!(err.to_string().contains("registry identity fail-closed"), "{err}");
}

#[test]
fn fail_closed_registry_identity_missing_required_key() {
    let (_dir, db_path) = fixture_db();
    let conn = open_reader(&db_path);
    conn.execute(
        "DELETE FROM capability_registry_meta WHERE key = 'schema_version'",
        [],
    )
    .expect("delete required key");

    let state = RegistryObservationState::new("test-node", &db_path);
    let err = state.capability("alpha").expect_err("missing key must fail closed");
    assert!(err.to_string().contains("required meta key 'schema_version'"), "{err}");
}

#[test]
fn no_write_path_registry_unchanged_after_all_projections() {
    let (_dir, db_path) = fixture_db();

    // Snapshot every row of the four projection tables before…
    let dump = |path: &Path| -> String {
        let conn = open_reader(path);
        let mut out = String::new();
        for table in [
            "capabilities",
            "capability_versions",
            "capability_dependencies",
            "capability_types",
            "capability_registry_meta",
        ] {
            let rows: String = {
                let mut stmt = conn
                    .prepare(&format!("SELECT * FROM {table} ORDER BY 1"))
                    .expect("prepare dump");
                let cols = stmt.column_count();
                let mut s = String::new();
                let mut rows = stmt.query([]).expect("query dump");
                while let Some(row) = rows.next().expect("row") {
                    for i in 0..cols {
                        // Mixed column types (TEXT, INTEGER, NULL): use
                        // ValueRef to avoid type-mismatch panics; dump as
                        // sentinel so the comparison is total.
                        use rusqlite::types::ValueRef;
                        let value = match row.get_ref(i).expect("col ref") {
                            ValueRef::Null => "<NULL>".to_string(),
                            ValueRef::Integer(n) => n.to_string(),
                            ValueRef::Real(f) => f.to_string(),
                            ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
                            ValueRef::Blob(b) => format!("<BLOB:{}>", b.len()),
                        };
                        s.push_str(&value);
                        s.push('|');
                    }
                    s.push('\n');
                }
                s
            };
            out.push_str(&format!("== {table} ==\n{rows}"));
        }
        out
    };

    let before = dump(&db_path);

    // Run every projection.
    let state = RegistryObservationState::new("test-node", &db_path);
    state.capability("alpha").expect("capability");
    state.capability_versions("gamma").expect("versions");
    state.capability_dependencies("alpha").expect("dependencies");
    state.capability_types().expect("types");
    state.registry_overview().expect("overview");

    // The registry is byte-identical — no write path exists.
    let after = dump(&db_path);
    assert_eq!(before, after, "projections must never modify the registry");
}
