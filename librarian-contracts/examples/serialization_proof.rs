//! Phase 0 — Serialization Determinism Proof
//!
//! Run: cargo run --example serialization_proof -p librarian-contracts
//!
//! Proves that canonical serialization is deterministic, round-trips faithfully,
//! and produces wire-format-compatible output. Does NOT modify any contracts.

use librarian_contracts::prelude::*;
use std::collections::BTreeMap;

fn main() {
    println!("=== Phase 0: Serialization Determinism Proof ===\n");

    // 1. Canonical JSON stability
    println!("[1/4] Canonical JSON Stability");
    let mut data = BTreeMap::new();
    data.insert("z_key", "last");
    data.insert("a_key", "first");
    data.insert("m_key", "middle");

    let json_a = to_canonical_json(&data).unwrap();
    let json_b = to_canonical_json(&data).unwrap();
    assert_eq!(json_a, json_b, "Canonical JSON must be identical across runs");
    println!("  PASS: Same data → same JSON (sorted keys)");

    // 2. SHA-256 determinism
    println!("\n[2/4] SHA-256 Hash Determinism");
    let hash_a = hash_canonical(&data).unwrap();
    let hash_b = hash_canonical(&data).unwrap();
    assert_eq!(hash_a, hash_b, "SHA-256 hashes must be identical across runs");
    assert_eq!(hash_a.len(), 64, "SHA-256 hex must be 64 characters");
    println!("  PASS: Hash stable: {}", hash_a);

    // 3. Round-trip fidelity
    println!("\n[3/4] Round-Trip Fidelity");
    let receipt = SprintAuthorizationReceipt {
        receipt_id: "round-trip-test".into(),
        receipt_type: ReceiptType::SprintAuthorization,
        work_order_id: "WO-003".into(),
        authorized_at: "2026-07-23T00:00:00Z".into(),
        authorized_by: "phase0-evidence".into(),
        repository: "Librarian-Node".into(),
        scope: "Serialization proof".into(),
        capability_expansion: None,
        migration_code: false,
        schema_version: RECEIPT_CONTRACT_VERSION.into(),
    };
    let json_original = to_canonical_json(&receipt).unwrap();
    let deserialized: SprintAuthorizationReceipt = serde_json::from_str(&json_original).unwrap();
    let json_roundtrip = to_canonical_json(&deserialized).unwrap();
    assert_eq!(json_original, json_roundtrip, "Round-trip must produce identical JSON");
    println!("  PASS: Serialize → deserialize → serialize produces identical output");

    // 4. Wire format compatibility
    println!("\n[4/4] Wire Format Compatibility");
    let custody_mode = CustodyMode::OwnerHeld;
    let mode_json = serde_json::to_value(&custody_mode).unwrap();
    assert_eq!(mode_json, serde_json::json!("OWNER_HELD"),
        "CustodyMode must serialize as SCREAMING_SNAKE_CASE");

    let role = NodeRole::LibrarianAuthority;
    let role_json = serde_json::to_value(&role).unwrap();
    assert_eq!(role_json, serde_json::json!("librarian_authority"),
        "NodeRole must serialize as snake_case");

    let state = LifecycleState::Operational;
    let state_json = serde_json::to_value(&state).unwrap();
    assert_eq!(state_json, serde_json::json!("operational"),
        "LifecycleState must serialize as snake_case");
    println!("  PASS: All enums serialize to expected wire format");

    // Print the round-trip JSON as evidence
    println!("\n=== Round-trip payload ===");
    println!("{}", json_original);

    println!("\n=== All serialization proofs PASS ===");
}
