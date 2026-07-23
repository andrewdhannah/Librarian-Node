//! MQR-H5: Custom Validation Evidence Integration — integration tests.
//!
//! These tests prove that custom validation evidence flows through the
//! qualification pipeline without becoming authoritative — no capability
//! manifest creation, no routing mutation, no Owner decision alteration.
//!
//! Core invariants under test:
//!   CustomRuleEvidence is additive and inspectable — NOT authoritative.
//!   apply_custom_rules() collects evidence only — no side effects.
//!   Evidence lifecycle is deterministic and reproducible.
//!   Failure outcomes are represented as evidence, not decisions.
//!   Existing qualification behavior is unchanged when custom validation is absent.

use librarian_core::qualification::custom_executor::{
    apply_custom_rules, CustomRuleDefinition, CustomRuleExecutor,
};
use librarian_core::qualification::run_result::QualificationRunResult;
use librarian_core::qualification::run_state::RunState;
use librarian_core::qualification::validator_engine::RuleSeverity;

// ============================================================================
// Test helpers
// ============================================================================

fn test_run_result(output: &str, token_count: Option<u32>) -> QualificationRunResult {
    let run_id = QualificationRunResult::compute_run_id("qr-h5-001", "2026-07-11T12:00:00Z");
    QualificationRunResult {
        run_id,
        request_id: "qr-h5-001".to_string(),
        model_id: "minicpm5-1b-q4km".to_string(),
        model_sha256: "abc123".to_string(),
        model_filename: "test-model.gguf".to_string(),
        task_pack_id: "tp-h5-001".to_string(),
        fixture_hash: "def456".to_string(),
        state: RunState::Completed,
        raw_output: if output.is_empty() { None } else { Some(output.to_string()) },
        settings: librarian_core::qualification::run_result::GenerationSettings {
            runtime_profile_id: "prof-h5".to_string(),
            max_tokens: Some(256),
            temperature: Some(0.0),
            timeout_seconds: Some(120),
            task_description: "H5 test task".to_string(),
        },
        telemetry: librarian_core::qualification::run_result::RuntimeTelemetry {
            port: Some(9120),
            process_id: Some(12345),
            load_duration_ms: Some(1000),
            generation_duration_ms: Some(500),
            input_tokens: Some(10),
            output_tokens: token_count,
            http_status: Some(200),
            runtime_error: None,
        },
        lifecycle_events: vec![],
        error_message: None,
        custom_evidence: vec![],
        started_at: "2026-07-11T12:00:00Z".to_string(),
        ended_at: Some("2026-07-11T12:00:01Z".to_string()),
    }
}

fn make_rule(rule_id: &str, strategy: &str, severity: RuleSeverity) -> CustomRuleDefinition {
    CustomRuleDefinition {
        rule_id: rule_id.to_string(),
        version: "1.0.0".to_string(),
        description: format!("Custom rule {} with strategy {}", rule_id, strategy),
        severity,
        params: serde_json::json!({"strategy": strategy}),
    }
}

fn make_contains_rule(rule_id: &str, target: &str) -> CustomRuleDefinition {
    CustomRuleDefinition {
        rule_id: rule_id.to_string(),
        version: "1.0.0".to_string(),
        description: format!("Check contains '{}'", target),
        severity: RuleSeverity::Critical,
        params: serde_json::json!({"strategy": "contains", "target": target}),
    }
}

// ============================================================================
// H5-T1: Evidence flows through apply_custom_rules into run result
// ============================================================================

#[test]
fn test_h5_t1_evidence_collected_in_run_result() {
    let mut result = test_run_result("Hello world output", Some(50));
    let executor = CustomRuleExecutor::default();
    let rules = vec![make_rule("CR-FLOW", "pass", RuleSeverity::Critical)];

    apply_custom_rules(&mut result, &executor, &rules);

    assert_eq!(result.custom_evidence.len(), 1);
    assert_eq!(result.custom_evidence[0].rule_id, "CR-FLOW");
    assert!(result.custom_evidence[0].outcome.passed);
}

// ============================================================================
// H5-T2: Evidence is additive (multiple rules → multiple evidence records)
// ============================================================================

#[test]
fn test_h5_t2_evidence_is_additive() {
    let mut result = test_run_result("test output", Some(50));
    let executor = CustomRuleExecutor::default();
    let rules = vec![
        make_rule("CR-01", "pass", RuleSeverity::Critical),
        make_rule("CR-02", "pass", RuleSeverity::Warning),
        make_rule("CR-03", "pass", RuleSeverity::Info),
    ];

    apply_custom_rules(&mut result, &executor, &rules);

    assert_eq!(result.custom_evidence.len(), 3);
    // Same strategy → same outcome
    assert!(result.custom_evidence.iter().all(|e| e.outcome.passed));
}

// ============================================================================
// H5-T3: Evidence is sorted deterministically by rule_id
// ============================================================================

#[test]
fn test_h5_t3_evidence_sort_order_deterministic() {
    let executor = CustomRuleExecutor::default();
    let rules = vec![
        make_rule("CR-Z", "pass", RuleSeverity::Critical),
        make_rule("CR-A", "pass", RuleSeverity::Critical),
        make_rule("CR-M", "pass", RuleSeverity::Critical),
    ];

    // First application
    let mut result1 = test_run_result("output", Some(10));
    apply_custom_rules(&mut result1, &executor, &rules);

    // Second application
    let mut result2 = test_run_result("output", Some(10));
    apply_custom_rules(&mut result2, &executor, &rules);

    assert_eq!(result1.custom_evidence.len(), 3);
    assert_eq!(result2.custom_evidence.len(), 3);

    // Both should have the same order
    for i in 0..3 {
        assert_eq!(result1.custom_evidence[i].rule_id, result2.custom_evidence[i].rule_id);
    }

    // Order should be sorted: CR-A, CR-M, CR-Z
    assert_eq!(result1.custom_evidence[0].rule_id, "CR-A");
    assert_eq!(result1.custom_evidence[1].rule_id, "CR-M");
    assert_eq!(result1.custom_evidence[2].rule_id, "CR-Z");
}

// ============================================================================
// H5-T4: Evidence serialization round-trip preserves all fields
// ============================================================================

#[test]
fn test_h5_t4_evidence_serialization_roundtrip() {
    let mut result = test_run_result("Hello world", Some(50));
    let executor = CustomRuleExecutor::default();
    let rules = vec![make_contains_rule("CR-CHECK", "Hello")];

    apply_custom_rules(&mut result, &executor, &rules);

    // Serialize the whole run result
    let json = result.to_json().unwrap();
    let parsed = QualificationRunResult::from_json(&json).unwrap();

    assert_eq!(parsed.custom_evidence.len(), 1);
    assert_eq!(parsed.custom_evidence[0].rule_id, "CR-CHECK");
    assert_eq!(parsed.custom_evidence[0].version, "1.0.0");
    assert!(parsed.custom_evidence[0].outcome.passed);
    assert_eq!(parsed.custom_evidence[0].task_pack_id, "tp-h5-001");
}

// ============================================================================
// H5-T5: Evidence content hash is stable (same output → same content hash)
// ============================================================================

#[test]
fn test_h5_t5_evidence_content_hash_stable() {
    let executor = CustomRuleExecutor::default();
    let rules = vec![make_contains_rule("CR-STABLE", "test")];

    let mut result1 = test_run_result("test output", Some(50));
    apply_custom_rules(&mut result1, &executor, &rules);

    let mut result2 = test_run_result("test output", Some(50));
    apply_custom_rules(&mut result2, &executor, &rules);

    // Compare evidence content hashes (not full run result hash, which includes timestamps)
    let hash1 = result1.custom_evidence[0].compute_content_hash().unwrap();
    let hash2 = result2.custom_evidence[0].compute_content_hash().unwrap();
    assert_eq!(hash1, hash2, "Same output + same rules → same evidence content hash");
}

// ============================================================================
// H5-T6: Evidence hash changes with different output
// ============================================================================

#[test]
fn test_h5_t6_evidence_hash_changes_with_output() {
    let executor = CustomRuleExecutor::default();
    let rules = vec![make_contains_rule("CR-CONTENT", "Hello")];

    // Output contains "Hello" → pass
    let mut result_pass = test_run_result("Hello world", Some(50));
    apply_custom_rules(&mut result_pass, &executor, &rules);

    // Output does NOT contain "Hello" → fail
    let mut result_fail = test_run_result("Goodbye world", Some(50));
    apply_custom_rules(&mut result_fail, &executor, &rules);

    let hash_pass = result_pass.compute_hash().unwrap();
    let hash_fail = result_fail.compute_hash().unwrap();
    assert_ne!(hash_pass, hash_fail, "Different outcomes → different hashes");
}

// ============================================================================
// H5-T7: Empty output produces evidence (not an error)
// ============================================================================

#[test]
fn test_h5_t7_empty_output_evidence() {
    let mut result = test_run_result("", None);
    let executor = CustomRuleExecutor::default();
    let rules = vec![make_contains_rule("CR-EMPTY", "anything")];

    apply_custom_rules(&mut result, &executor, &rules);

    // Evidence is still collected — it just shows fail (empty output doesn't contain target)
    assert_eq!(result.custom_evidence.len(), 1);
    assert!(!result.custom_evidence[0].outcome.passed);
    assert!(!result.custom_evidence[0].outcome.timed_out);
    assert!(!result.custom_evidence[0].outcome.panicked);
}

// ============================================================================
// H5-T8: No custom rules → no custom evidence (existing behavior preserved)
// ============================================================================

#[test]
fn test_h5_t8_no_rules_no_evidence() {
    let mut result = test_run_result("output", Some(50));
    let executor = CustomRuleExecutor::default();

    apply_custom_rules(&mut result, &executor, &[]);

    assert!(result.custom_evidence.is_empty());
    assert!(result.raw_output.is_some());
    assert_eq!(result.state, RunState::Completed);
}

// ============================================================================
// H5-T9: Timeout evidence flows through run result
// ============================================================================

#[test]
fn test_h5_t9_timeout_evidence_flows_through() {
    let mut result = test_run_result("output", Some(50));
    let short_executor = CustomRuleExecutor::new(100);
    let rules = vec![CustomRuleDefinition {
        rule_id: "CR-HANG".to_string(),
        version: "1.0.0".to_string(),
        description: "Hangs intentionally".to_string(),
        severity: RuleSeverity::Critical,
        params: serde_json::json!({"strategy": "hang"}),
    }];

    apply_custom_rules(&mut result, &short_executor, &rules);

    assert_eq!(result.custom_evidence.len(), 1);
    assert!(result.custom_evidence[0].outcome.timed_out);
    assert!(!result.custom_evidence[0].outcome.passed);
    assert!(!result.custom_evidence[0].outcome.panicked);
}

// ============================================================================
// H5-T10: Panic evidence flows through run result
// ============================================================================

#[test]
fn test_h5_t10_panic_evidence_flows_through() {
    let mut result = test_run_result("output", Some(50));
    let executor = CustomRuleExecutor::default();
    let rules = vec![CustomRuleDefinition {
        rule_id: "CR-PANIC".to_string(),
        version: "1.0.0".to_string(),
        description: "Panics intentionally".to_string(),
        severity: RuleSeverity::Critical,
        params: serde_json::json!({"strategy": "panic"}),
    }];

    apply_custom_rules(&mut result, &executor, &rules);

    assert_eq!(result.custom_evidence.len(), 1);
    assert!(!result.custom_evidence[0].outcome.passed);
    assert!(result.custom_evidence[0].outcome.panicked);
    assert!(!result.custom_evidence[0].outcome.timed_out);
}

// ============================================================================
// H5-T11: Version mismatch recorded in evidence (not rejected)
// ============================================================================

#[test]
fn test_h5_t11_version_mismatch_in_evidence() {
    let mut result = test_run_result("version test", Some(50));
    let executor = CustomRuleExecutor::default();
    let rules = vec![CustomRuleDefinition {
        rule_id: "CR-VERSION".to_string(),
        version: "999.0.0".to_string(), // Arbitrary future version
        description: "Future version rule".to_string(),
        severity: RuleSeverity::Critical,
        params: serde_json::json!({"strategy": "pass"}),
    }];

    apply_custom_rules(&mut result, &executor, &rules);

    assert_eq!(result.custom_evidence.len(), 1);
    assert_eq!(result.custom_evidence[0].version, "999.0.0");
    assert!(result.custom_evidence[0].outcome.passed);
}

// ============================================================================
// H5-T12: Unknown rule identity recorded in evidence
// ============================================================================

#[test]
fn test_h5_t12_unknown_rule_identity_handled() {
    let mut result = test_run_result("output", Some(50));
    let executor = CustomRuleExecutor::default();
    let rules = vec![CustomRuleDefinition {
        rule_id: "CR-UNKNOWN-ID".to_string(),
        version: "0.0.1".to_string(),
        description: "Rule with unknown identity pattern".to_string(),
        severity: RuleSeverity::Warning,
        params: serde_json::json!({"strategy": "unknown_strategy_xyz"}),
    }];

    apply_custom_rules(&mut result, &executor, &rules);

    assert_eq!(result.custom_evidence.len(), 1);
    assert_eq!(result.custom_evidence[0].rule_id, "CR-UNKNOWN-ID");
    // Unknown strategy defaults to pass with message
    assert!(result.custom_evidence[0].outcome.passed);
    assert!(result.custom_evidence[0].outcome.message.is_some());
}

// ============================================================================
// H5-T13: Missing rule metadata uses defaults gracefully
// ============================================================================

#[test]
fn test_h5_t13_missing_metadata_uses_defaults() {
    let mut result = test_run_result("output", Some(50));
    let executor = CustomRuleExecutor::default();
    let rules = vec![CustomRuleDefinition {
        rule_id: "CR-NO-META".to_string(),
        version: "0.1.0".to_string(), // Default version
        description: String::new(),    // Empty description
        severity: RuleSeverity::Critical,
        params: serde_json::json!({}), // No strategy → defaults to pass
    }];

    apply_custom_rules(&mut result, &executor, &rules);

    assert_eq!(result.custom_evidence.len(), 1);
    assert!(result.custom_evidence[0].outcome.passed);
    // Empty params default to "pass" strategy — message may be absent
}

// ============================================================================
// H5-T14: Evidence does NOT create capability manifest
// ============================================================================

#[test]
fn test_h5_t14_evidence_does_not_create_manifest() {
    let mut result = test_run_result("output", Some(50));
    let executor = CustomRuleExecutor::default();
    let rules = vec![make_rule("CR-MANIFEST", "pass", RuleSeverity::Critical)];

    apply_custom_rules(&mut result, &executor, &rules);

    // QualificationRunResult has no fields for capability manifests
    assert!(result.custom_evidence.len() >= 1);

    // Structural proof: no manifest_id, no manifest_status
    // (Compile-time proof — these fields don't exist on QualificationRunResult)
    let json = result.to_json().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("manifest_id").is_none(), "Evidence should not create a manifest_id");
    assert!(parsed.get("manifest_status").is_none(), "Evidence should not create a manifest_status");
}

// ============================================================================
// H5-T15: Evidence does NOT mutate router state
// ============================================================================

#[test]
fn test_h5_t15_evidence_does_not_mutate_router() {
    let mut result = test_run_result("output", Some(50));
    let executor = CustomRuleExecutor::default();
    let rules = vec![
        make_rule("CR-ROUTER-1", "pass", RuleSeverity::Critical),
        make_rule("CR-ROUTER-2", "fail", RuleSeverity::Critical),
    ];

    apply_custom_rules(&mut result, &executor, &rules);

    // Structural proof: no projections, no router fields
    let json = result.to_json().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("projections").is_none(), "Evidence should not create router projections");
    assert!(parsed.get("router_eligible").is_none(), "Evidence should not create router eligibility");
    assert!(parsed.get("routing_status").is_none(), "Evidence should not create routing status");
}

// ============================================================================
// H5-T16: Evidence does NOT alter Owner decision
// ============================================================================

#[test]
fn test_h5_t16_evidence_does_not_alter_owner_decision() {
    let mut result = test_run_result("output", Some(50));
    let executor = CustomRuleExecutor::default();

    // Even with evidence suggesting pass or fail
    let rules = vec![
        make_rule("CR-OWNER-1", "pass", RuleSeverity::Critical),
        make_rule("CR-OWNER-2", "fail", RuleSeverity::Critical),
    ];

    apply_custom_rules(&mut result, &executor, &rules);

    // Structural proof: no decision fields
    let json = result.to_json().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("decision_id").is_none(), "Evidence should not create a decision_id");
    assert!(parsed.get("decision_type").is_none(), "Evidence should not create a decision_type");
    assert!(parsed.get("approved").is_none(), "Evidence should not create approval status");
}

// ============================================================================
// H5-T17: Evidence does NOT bypass qualification gates
// ============================================================================

#[test]
fn test_h5_t17_evidence_does_not_bypass_gates() {
    let mut result = test_run_result("output", Some(50));
    let executor = CustomRuleExecutor::default();

    // Even perfect evidence passes
    let rules = vec![make_rule("CR-BYPASS", "pass", RuleSeverity::Critical)];

    apply_custom_rules(&mut result, &executor, &rules);

    // The result state remains unchanged — evidence doesn't mutate run state
    assert_eq!(result.state, RunState::Completed);

    // Evidence doesn't auto-promote state to something it shouldn't be
    assert!(!result.state.is_terminal() || result.state == RunState::Completed);
}

// ============================================================================
// H5-T18: Evidence is add-only (not destructive)
// ============================================================================

#[test]
fn test_h5_t18_evidence_is_add_only() {
    let mut result = test_run_result("output", Some(50));
    let executor = CustomRuleExecutor::default();

    let rules1 = vec![make_rule("CR-ADD-1", "pass", RuleSeverity::Critical)];
    apply_custom_rules(&mut result, &executor, &rules1);
    assert_eq!(result.custom_evidence.len(), 1);

    // Second application adds more evidence
    let rules2 = vec![make_rule("CR-ADD-2", "pass", RuleSeverity::Critical)];
    apply_custom_rules(&mut result, &executor, &rules2);
    assert_eq!(result.custom_evidence.len(), 2);

    // First evidence still there
    assert_eq!(result.custom_evidence[0].rule_id, "CR-ADD-1");
    assert_eq!(result.custom_evidence[1].rule_id, "CR-ADD-2");
}

// ============================================================================
// H5-T19: Multiple evidence entries with different severities all captured
// ============================================================================

#[test]
fn test_h5_t19_different_severities_captured() {
    let mut result = test_run_result("output", Some(50));
    let executor = CustomRuleExecutor::default();
    let rules = vec![
        CustomRuleDefinition {
            rule_id: "CR-CRITICAL".to_string(),
            version: "1.0.0".to_string(),
            description: "Critical rule".to_string(),
            severity: RuleSeverity::Critical,
            params: serde_json::json!({"strategy": "pass"}),
        },
        CustomRuleDefinition {
            rule_id: "CR-WARNING".to_string(),
            version: "1.0.0".to_string(),
            description: "Warning rule".to_string(),
            severity: RuleSeverity::Warning,
            params: serde_json::json!({"strategy": "pass"}),
        },
        CustomRuleDefinition {
            rule_id: "CR-INFO".to_string(),
            version: "1.0.0".to_string(),
            description: "Info rule".to_string(),
            severity: RuleSeverity::Info,
            params: serde_json::json!({"strategy": "pass"}),
        },
    ];

    apply_custom_rules(&mut result, &executor, &rules);

    assert_eq!(result.custom_evidence.len(), 3);
    // All passed — evidence is collected regardless of severity
    assert!(result.custom_evidence.iter().all(|e| e.outcome.passed));
}

// ============================================================================
// H5-T20: apply_custom_rules with fail strategy records failure as evidence
// ============================================================================

#[test]
fn test_h5_t20_fail_strategy_recorded_as_evidence() {
    let mut result = test_run_result("output", Some(50));
    let executor = CustomRuleExecutor::default();
    let rules = vec![make_rule("CR-FAIL-EVIDENCE", "fail", RuleSeverity::Critical)];

    apply_custom_rules(&mut result, &executor, &rules);

    assert_eq!(result.custom_evidence.len(), 1);
    assert!(!result.custom_evidence[0].outcome.passed);
    // Failure is evidence — not an automatic rejection
    assert_eq!(result.state, RunState::Completed);
    assert!(result.raw_output.is_some());
}

// ============================================================================
// H5-T21: Evidence content hash validates across serialization
// ============================================================================

#[test]
fn test_h5_t21_evidence_content_hash_across_serialization() {
    let mut result = test_run_result("hash test output", Some(50));
    let executor = CustomRuleExecutor::default();
    let rules = vec![make_rule("CR-HASH", "pass", RuleSeverity::Critical)];

    apply_custom_rules(&mut result, &executor, &rules);

    // Capture original evidence hash
    let original_hash = result.custom_evidence[0].content_hash.clone();

    // Serialize and deserialize
    let json = result.to_json().unwrap();
    let parsed = QualificationRunResult::from_json(&json).unwrap();

    // Compute content hash from parsed evidence
    let parsed_hash = parsed.custom_evidence[0].compute_content_hash().unwrap();
    assert_eq!(original_hash, parsed_hash, "Content hash preserved across serialization");
}

// ============================================================================
// H5-T22: Token count is propagated to custom rule context
// ============================================================================

#[test]
fn test_h5_t22_token_count_passed_to_rules() {
    let mut result = test_run_result("output", Some(42));
    let executor = CustomRuleExecutor::default();
    let rules = vec![CustomRuleDefinition {
        rule_id: "CR-MINTOKENS".to_string(),
        version: "1.0.0".to_string(),
        description: "Min 40 tokens".to_string(),
        severity: RuleSeverity::Critical,
        params: serde_json::json!({"strategy": "min_tokens", "min": 40}),
    }];

    apply_custom_rules(&mut result, &executor, &rules);

    assert_eq!(result.custom_evidence.len(), 1);
    assert!(result.custom_evidence[0].outcome.passed, "Should pass with 42 >= 40");
}

// ============================================================================
// H5-T23: Token count below minimum produces fail evidence
// ============================================================================

#[test]
fn test_h5_t23_token_count_below_minimum() {
    let mut result = test_run_result("output", Some(5));
    let executor = CustomRuleExecutor::default();
    let rules = vec![CustomRuleDefinition {
        rule_id: "CR-MINTOKENS".to_string(),
        version: "1.0.0".to_string(),
        description: "Min 40 tokens".to_string(),
        severity: RuleSeverity::Critical,
        params: serde_json::json!({"strategy": "min_tokens", "min": 40}),
    }];

    apply_custom_rules(&mut result, &executor, &rules);

    assert_eq!(result.custom_evidence.len(), 1);
    assert!(!result.custom_evidence[0].outcome.passed, "Should fail with 5 < 40");
}

// ============================================================================
// H5-T24: Run with no token count — min_tokens evidence handles gracefully
// ============================================================================

#[test]
fn test_h5_t24_no_token_count_handled() {
    let mut result = test_run_result("output", None); // No token count
    let executor = CustomRuleExecutor::default();
    let rules = vec![CustomRuleDefinition {
        rule_id: "CR-NOTOKENS".to_string(),
        version: "1.0.0".to_string(),
        description: "Min 1 token".to_string(),
        severity: RuleSeverity::Critical,
        params: serde_json::json!({"strategy": "min_tokens", "min": 1}),
    }];

    apply_custom_rules(&mut result, &executor, &rules);

    assert_eq!(result.custom_evidence.len(), 1);
    // When token_count is None, min_tokens returns false
    assert!(!result.custom_evidence[0].outcome.passed);
}

// ============================================================================
// H5-T25: Multiple rule applications are idempotent (same input → same evidence)
// ============================================================================

#[test]
fn test_h5_t25_idempotent_application() {
    let executor = CustomRuleExecutor::default();
    let rules = vec![
        make_contains_rule("CR-A", "Hello"),
        make_contains_rule("CR-B", "world"),
    ];

    let mut result_a = test_run_result("Hello world", Some(10));
    apply_custom_rules(&mut result_a, &executor, &rules);

    let mut result_b = test_run_result("Hello world", Some(10));
    apply_custom_rules(&mut result_b, &executor, &rules);

    // Same output + same rules → same evidence results
    assert_eq!(result_a.custom_evidence.len(), result_b.custom_evidence.len());
    for i in 0..result_a.custom_evidence.len() {
        assert_eq!(
            result_a.custom_evidence[i].outcome.passed,
            result_b.custom_evidence[i].outcome.passed
        );
    }
}
