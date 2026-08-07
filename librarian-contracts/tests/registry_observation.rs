//! M1-B registry observation projection type conformance (RUST-MIGRATION-M1-B).
//!
//! Verifies the observation surface against the locked M1-B read boundary
//! (`contracts/runtime-api/RUNTIME-REGISTRY-OBSERVATION-CONTRACT-001.md`,
//! Canonical `232edfb`, suite `REGISTRY-OBSERVATION-SCHEMA-001`):
//!
//! 1. Serialization conformance — serde names match the contract-enum names.
//! 2. Fail-closed enum parsing — unknown persisted values fail, never coerce
//!    (owner decision 3: an unknown value is schema/semantic drift).
//! 3. Payload discipline — `CapabilityObservation` embeds the locked
//!    `CapabilityIdentity` payload; `CapabilityVersionRecord` exposes
//!    `content_hash`, never `body` (owner decision 1).
//! 4. Naming guard — `capability_types` table rows (CapabilityTypeDefinition)
//!    are distinct from the `CapabilityType` enum (owner decision 4).
//! 5. Determinism — overview groups are fixed and zero-count-present
//!    (owner decision 6); the envelope's only variable field is
//!    `projection_observed_at` (contract §3.1).
//!
//! Authority checks are enforced by the drift guard
//! (`contract_surface_manifest.rs::m1b_serialized_surface_has_no_authority_keys`)
//! and by construction: these types expose no write/transition methods.

use librarian_contracts::node::capability_registry::{
    CapabilityId, CapabilityType, CapabilityVersion, QualificationAxis, QualificationState,
};
use librarian_contracts::node::registry_observation::{
    AuthorityAxis, AvailabilityAxis, CapabilityObservation, CapabilityTypeDefinition,
    CapabilityVersionRecord, RegistryIdentity, RegistryObservationEnvelope, RegistryOverview,
    TypeCategory,
};

// ---------------------------------------------------------------------------
// Serialization conformance
// ---------------------------------------------------------------------------

#[test]
fn enum_serde_names_match_contract_surface() {
    let cases: Vec<(&str, &str, &str)> = vec![
        ("AvailabilityAxis", "discovered", AvailabilityAxis::Discovered.as_str()),
        ("AvailabilityAxis", "registered", AvailabilityAxis::Registered.as_str()),
        ("AvailabilityAxis", "disabled", AvailabilityAxis::Disabled.as_str()),
        ("AvailabilityAxis", "removed", AvailabilityAxis::Removed.as_str()),
        ("AuthorityAxis", "not_submitted", AuthorityAxis::NotSubmitted.as_str()),
        ("AuthorityAxis", "pending_review", AuthorityAxis::PendingReview.as_str()),
        ("AuthorityAxis", "approved", AuthorityAxis::Approved.as_str()),
        ("AuthorityAxis", "rejected", AuthorityAxis::Rejected.as_str()),
        ("AuthorityAxis", "revoked", AuthorityAxis::Revoked.as_str()),
        ("TypeCategory", "standard", TypeCategory::Standard.as_str()),
        ("TypeCategory", "system", TypeCategory::System.as_str()),
        ("TypeCategory", "experimental", TypeCategory::Experimental.as_str()),
        ("TypeCategory", "external", TypeCategory::External.as_str()),
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
        // And deserialization must round-trip.
        let back: serde_json::Value =
            serde_json::from_str(&format!("\"{expected}\"")).expect("deserialize");
        assert!(back.as_str().is_some(), "{family}: {expected} not deserializable");
    }
}

// ---------------------------------------------------------------------------
// Fail-closed semantics (owner decision 3)
// ---------------------------------------------------------------------------

#[test]
fn unknown_enum_values_fail_closed() {
    // The axis columns carry no CHECK constraints in the governed schema, so a
    // persisted value outside the value set is schema/semantic drift. Serde
    // enum deserialization MUST fail (no silent default, no coercion) — the
    // projection module relies on this to fail the whole projection.
    let cases: Vec<(&str, &str)> = vec![
        ("AvailabilityAxis", "half_registered"),
        ("AvailabilityAxis", "AVAILABLE"),
        ("AuthorityAxis", "super_approved"),
        ("AuthorityAxis", ""),
        ("TypeCategory", "premium"),
    ];
    for (type_name, value) in &cases {
        let serialized = format!("\"{value}\"");
        match *type_name {
            "AvailabilityAxis" => {
                let r: Result<AvailabilityAxis, _> = serde_json::from_str(&serialized);
                assert!(r.is_err(), "{type_name}: '{value}' must fail closed");
            }
            "AuthorityAxis" => {
                let r: Result<AuthorityAxis, _> = serde_json::from_str(&serialized);
                assert!(r.is_err(), "{type_name}: '{value}' must fail closed");
            }
            "TypeCategory" => {
                let r: Result<TypeCategory, _> = serde_json::from_str(&serialized);
                assert!(r.is_err(), "{type_name}: '{value}' must fail closed");
            }
            _ => unreachable!(),
        }
    }
}

// ---------------------------------------------------------------------------
// Payload discipline
// ---------------------------------------------------------------------------

fn sample_observation() -> CapabilityObservation {
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
}

#[test]
fn capability_observation_embeds_identity_payload() {
    // CapabilityIdentity's five fields + the three assurance axes; nothing else.
    let json = serde_json::to_value(sample_observation()).unwrap();
    let obj = json.as_object().unwrap();
    assert_eq!(obj.len(), 8, "observation = identity(5) + axes(3)");
    for key in [
        "capability_id",
        "name",
        "capability_type",
        "version",
        "lifecycle_state",
        "availability",
        "qualification",
        "authority",
    ] {
        assert!(obj.contains_key(key), "missing field {key}");
    }
    // No content/policy/provenance material leaks into the projection.
    for key in obj.keys() {
        assert!(
            !["description", "summary", "execution_policy", "created_at", "updated_at"]
                .contains(&key.as_str()),
            "projection must not expose '{key}'"
        );
    }
    // The axes are plain enum values (column reads, not joins).
    assert_eq!(json["availability"].as_str().unwrap(), "registered");
    assert_eq!(json["qualification"].as_str().unwrap(), "passed");
    assert_eq!(json["authority"].as_str().unwrap(), "approved");
    assert_eq!(json["capability_type"].as_str().unwrap(), "skill");
    assert_eq!(json["version"].as_u64().unwrap(), 1);
}

#[test]
fn version_record_exposes_hash_never_body() {
    let record = CapabilityVersionRecord {
        capability_id: CapabilityId::new("alpha".to_string()).unwrap(),
        version: CapabilityVersion::new(1).unwrap(),
        content_hash: "a".repeat(64),
        changelog: Some("initial".to_string()),
        author: None,
        review_notes: None,
        qualification_evidence_id: Some("EV-00001".to_string()),
        profile_id: None,
        created_at: "2026-08-07T09:00:00Z".to_string(),
    };
    let json = serde_json::to_value(&record).unwrap();
    let obj = json.as_object().unwrap();
    assert_eq!(obj.len(), 9);
    assert!(
        !obj.contains_key("body"),
        "instruction body must never be exposed (owner decision 1)"
    );
    assert_eq!(json["content_hash"].as_str().unwrap(), "a".repeat(64));
    assert_eq!(json["qualification_evidence_id"].as_str().unwrap(), "EV-00001");

    // Round-trip preserves all fields.
    let back: CapabilityVersionRecord = serde_json::from_value(json).unwrap();
    assert_eq!(back, record);
}

#[test]
fn type_definition_rows_are_not_capability_type_enum() {
    // Naming guard (owner decision 4): the capability_types TABLE projects
    // taxonomy rows; the CapabilityType enum types a capability's single type.
    // The row payload must carry NO field that could be confused with the enum
    // variant surface (skill|workflow|policy|validator|template).
    let row = CapabilityTypeDefinition {
        capability_type_id: "design-review".to_string(),
        name: "Design Review".to_string(),
        description: "governed design review type".to_string(),
        category: TypeCategory::Standard,
        default_profile_id: Some("PROF-001".to_string()),
        default_policy_id: None,
    };
    let json = serde_json::to_value(&row).unwrap();
    let obj = json.as_object().unwrap();
    assert_eq!(obj.len(), 6);
    // The row id is a taxonomy key, NOT a CapabilityType variant value.
    assert_eq!(json["capability_type_id"].as_str().unwrap(), "design-review");
    assert!(!obj.contains_key("type"), "row payload must not use the field name 'type'");
    // Round-trip.
    let back: CapabilityTypeDefinition = serde_json::from_value(json).unwrap();
    assert_eq!(back, row);
}

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

fn sample_overview() -> RegistryOverview {
    RegistryOverview {
        capability_count: 1,
        by_status: vec![
            ("unreviewed".to_string(), 0),
            ("reviewed".to_string(), 1),
            ("qualified".to_string(), 0),
            ("deprecated".to_string(), 0),
            ("revoked".to_string(), 0),
        ],
        by_availability: vec![
            ("discovered".to_string(), 0),
            ("registered".to_string(), 1),
            ("disabled".to_string(), 0),
            ("removed".to_string(), 0),
        ],
        by_qualification: vec![
            ("not_tested".to_string(), 0),
            ("qualifying".to_string(), 0),
            ("passed".to_string(), 1),
            ("failed".to_string(), 0),
            ("stale".to_string(), 0),
            ("suspended".to_string(), 0),
        ],
        by_authority: vec![
            ("not_submitted".to_string(), 0),
            ("pending_review".to_string(), 0),
            ("approved".to_string(), 1),
            ("rejected".to_string(), 0),
            ("revoked".to_string(), 0),
        ],
        by_type: vec![
            ("skill".to_string(), 1),
            ("workflow".to_string(), 0),
            ("policy".to_string(), 0),
            ("validator".to_string(), 0),
            ("template".to_string(), 0),
        ],
        version_count: 1,
        dependency_count: 0,
        dependency_by_relationship: vec![
            ("requires".to_string(), 0),
            ("extends".to_string(), 0),
            ("refines".to_string(), 0),
            ("conflicts".to_string(), 0),
        ],
        type_count: 1,
        types_by_category: vec![
            ("standard".to_string(), 1),
            ("system".to_string(), 0),
            ("experimental".to_string(), 0),
            ("external".to_string(), 0),
        ],
    }
}

#[test]
fn overview_groups_are_fixed_and_zero_count_present() {
    let overview = sample_overview();
    let json = serde_json::to_value(&overview).unwrap();
    let obj = json.as_object().unwrap();
    assert_eq!(obj.len(), 11);

    // Every group field carries the FULL enum variant set, zero counts present
    // (owner decision 6: a fixed group set is the stronger observation contract).
    let expectations: Vec<(&str, usize, Vec<&str>)> = vec![
        ("by_status", 5, vec!["unreviewed", "reviewed", "qualified", "deprecated", "revoked"]),
        (
            "by_availability",
            4,
            vec!["discovered", "registered", "disabled", "removed"],
        ),
        (
            "by_qualification",
            6,
            vec!["not_tested", "qualifying", "passed", "failed", "stale", "suspended"],
        ),
        (
            "by_authority",
            5,
            vec!["not_submitted", "pending_review", "approved", "rejected", "revoked"],
        ),
        (
            "by_type",
            5,
            vec!["skill", "workflow", "policy", "validator", "template"],
        ),
        (
            "dependency_by_relationship",
            4,
            vec!["requires", "extends", "refines", "conflicts"],
        ),
        (
            "types_by_category",
            4,
            vec!["standard", "system", "experimental", "external"],
        ),
    ];
    for (field, expected_len, expected_order) in &expectations {
        let groups = obj
            .get(*field)
            .expect("group field present")
            .as_array()
            .expect("group array");
        assert_eq!(groups.len(), *expected_len, "{field}: group set must be fixed");
        let names: Vec<&str> = groups
            .iter()
            .map(|g| g[0].as_str().unwrap())
            .collect();
        assert_eq!(
            names, *expected_order,
            "{field}: group order must follow the enum ALL order"
        );
        for group in groups {
            assert_eq!(group.as_array().unwrap().len(), 2, "{field}: group = [name, count]");
            assert!(
                group[1].as_u64().is_some(),
                "{field}: count must be a number"
            );
        }
    }
    assert_eq!(json["capability_count"].as_u64().unwrap(), 1);
    assert_eq!(json["version_count"].as_u64().unwrap(), 1);
    assert_eq!(json["dependency_count"].as_u64().unwrap(), 0);
    assert_eq!(json["type_count"].as_u64().unwrap(), 1);
}

// ---------------------------------------------------------------------------
// Observation envelope
// ---------------------------------------------------------------------------

#[test]
fn envelope_round_trips_and_is_deterministic() {
    let identity = RegistryIdentity {
        value: "abcd1234".to_string(),
        source: "capability_registry_meta".to_string(),
        derivation: "sha256 over sorted capability_registry_meta key/value pairs".to_string(),
    };
    let envelope = RegistryObservationEnvelope {
        node_id: "WINPC-BIG-PICKLE".to_string(),
        registry_identity: identity.clone(),
        projection_observed_at: "2026-08-07T12:00:00Z".to_string(),
        projection: sample_overview(),
    };

    let json = serde_json::to_value(&envelope).unwrap();
    let obj = json.as_object().unwrap();
    assert_eq!(obj.len(), 4);
    // serde_json key order is alphabetical (no preserve_order); the field SET
    // is the equivalence surface (manifest serialization note).
    let keys: std::collections::BTreeSet<&str> = obj.keys().map(String::as_str).collect();
    let expected: std::collections::BTreeSet<&str> = [
        "node_id",
        "registry_identity",
        "projection_observed_at",
        "projection",
    ]
    .iter()
    .copied()
    .collect();
    assert_eq!(keys, expected);
    assert_eq!(json["registry_identity"]["source"].as_str().unwrap(), "capability_registry_meta");
    assert_eq!(json["registry_identity"]["value"].as_str().unwrap(), "abcd1234");

    // Round-trip.
    let back: RegistryObservationEnvelope<RegistryOverview> =
        serde_json::from_value(json).unwrap();
    assert_eq!(back, envelope);
    assert_eq!(back.projection.capability_count, 1);
}

#[test]
fn projection_observed_at_is_the_only_variable_field() {
    // Same payload, different observation time: the projection bytes MUST be
    // identical once the timestamp is removed (contract §3.1 determinism).
    let identity = RegistryIdentity {
        value: "v".to_string(),
        source: "capability_registry_meta".to_string(),
        derivation: "d".to_string(),
    };
    let payload = sample_overview();
    let a = RegistryObservationEnvelope {
        node_id: "NODE".to_string(),
        registry_identity: identity.clone(),
        projection_observed_at: "2026-08-07T10:00:00Z".to_string(),
        projection: payload.clone(),
    };
    let b = RegistryObservationEnvelope {
        node_id: "NODE".to_string(),
        registry_identity: identity,
        projection_observed_at: "2026-08-07T11:00:00Z".to_string(),
        projection: payload,
    };
    let strip = |e: &RegistryObservationEnvelope<RegistryOverview>| {
        let mut v = serde_json::to_value(e).unwrap();
        v.as_object_mut().unwrap().remove("projection_observed_at");
        v
    };
    assert_eq!(strip(&a), strip(&b), "projection payload must be fully deterministic");
    assert_ne!(a.projection_observed_at, b.projection_observed_at);
}

#[test]
fn registry_identity_carries_structured_provenance() {
    // Contract §3.1: registry_identity is { value, source, derivation } —
    // structured provenance, never an unbound field.
    let identity = RegistryIdentity {
        value: "sha256hex".to_string(),
        source: "capability_registry_meta".to_string(),
        derivation: "sha256 over sorted capability_registry_meta key/value pairs".to_string(),
    };
    let json = serde_json::to_value(&identity).unwrap();
    let obj = json.as_object().unwrap();
    assert_eq!(obj.len(), 3);
    assert!(obj.contains_key("value"));
    assert!(obj.contains_key("source"));
    assert!(obj.contains_key("derivation"));
    let back: RegistryIdentity = serde_json::from_value(json).unwrap();
    assert_eq!(back, identity);
}
