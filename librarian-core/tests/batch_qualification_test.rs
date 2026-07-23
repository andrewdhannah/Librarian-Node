//! MQR-H6: Local Batch Qualification + Closure Validation — integration tests.
//!
//! Phase 1: Batch qualification behavior (H6-1 through H6-5)
//! Phase 2: Negative proofs and closure validation (H6-6 through H6-10)

use librarian_contracts::common::{
    PacketConstraints, PacketExecutionConfig, PacketModelIdentity,
};
use librarian_contracts::qualification_request::QualificationRequest;
use librarian_core::qualification::batch::{
    BatchQualificationInput, BatchQualificationResult, BatchQualificationRunner, BatchTarget,
};
use librarian_core::qualification::runner::{ExecutionResponse, QualificationRunner, RuntimeExecutor};

// ============================================================================
// Test helpers
// ============================================================================

struct BatchMockExecutor {
    responses: Vec<MockResponse>,
}

struct MockResponse {
    output: &'static str,
    token_count: u32,
    duration_ms: u64,
    fail: bool,
}

impl RuntimeExecutor for BatchMockExecutor {
    fn execute(
        &self,
        _port: u16,
        _prompt: &str,
        _max_tokens: u32,
        _temperature: f64,
        _timeout_secs: u64,
    ) -> Result<ExecutionResponse, anyhow::Error> {
        let idx = (_port as usize - 9000) % self.responses.len();
        let resp = &self.responses[idx];
        if resp.fail {
            anyhow::bail!("Mock failure for model at port {}", _port);
        }
        Ok(ExecutionResponse {
            output: resp.output.to_string(),
            input_tokens: Some(10),
            output_tokens: Some(resp.token_count),
            generation_duration_ms: resp.duration_ms,
            http_status: Some(200),
            error: None,
        })
    }
}

fn make_request(model_id: &str) -> QualificationRequest {
    QualificationRequest::new(
        format!("req-{}", model_id),
        PacketModelIdentity {
            model_id: model_id.to_string(),
            sha256: format!("sha256-{}", model_id),
            filename: format!("{}.gguf", model_id),
            quantization: Some("Q4_K_M".to_string()),
        },
        PacketExecutionConfig {
            runtime_profile_id: format!("prof-{}", model_id),
            task_description: format!("Task for {}", model_id),
            max_tokens: Some(256),
            temperature: Some(0.0),
            timeout_seconds: Some(120),
        },
        PacketConstraints {
            require_release_proof: true,
            max_vram_mb: Some(4096),
        },
    )
}

fn make_target(position: usize, model_id: &str) -> BatchTarget {
    BatchTarget {
        position,
        model_id: model_id.to_string(),
        request: make_request(model_id),
        custom_rules: vec![],
    }
}

fn make_batch_runner() -> BatchQualificationRunner {
    let inner = QualificationRunner::new("/tmp/fixtures");
    BatchQualificationRunner::new(inner)
}

// ============================================================================
// H6-T1: Batch runner accepts multiple qualification targets (H6-1)
// ============================================================================

#[test]
fn test_h6_t1_batch_accepts_multiple_targets() {
    let runner = make_batch_runner();
    let executor = BatchMockExecutor {
        responses: vec![MockResponse {
            output: "model-a output",
            token_count: 50,
            duration_ms: 100,
            fail: false,
        }],
    };

    let input = BatchQualificationInput {
        batch_id: "batch-h6t1".to_string(),
        targets: vec![
            make_target(0, "model-a"),
            make_target(1, "model-b"),
            make_target(2, "model-c"),
        ],
        global_custom_rules: vec![],
        created_at: "2026-01-01T00:00:00Z".to_string(),
    };

    let result = runner.run_batch(&input, &executor, 9000);
    assert_eq!(result.individual_results.len(), 3);
    assert_eq!(result.model_order, vec!["model-a", "model-b", "model-c"]);
    assert_eq!(result.aggregate.total_targets, 3);
}

// ============================================================================
// H6-T2: Models execute sequentially (not in parallel) (H6-2)
// ============================================================================

#[test]
fn test_h6_t2_sequential_execution() {
    let runner = make_batch_runner();
    let executor = BatchMockExecutor {
        responses: vec![MockResponse {
            output: "output",
            token_count: 50,
            duration_ms: 100,
            fail: false,
        }],
    };

    let input = BatchQualificationInput {
        batch_id: "batch-seq".to_string(),
        targets: vec![
            make_target(0, "model-first"),
            make_target(1, "model-second"),
            make_target(2, "model-third"),
        ],
        global_custom_rules: vec![],
        created_at: "2026-01-01".to_string(),
    };

    let result = runner.run_batch(&input, &executor, 9000);

    // Results should be in the same order as targets
    assert_eq!(result.individual_results[0].position, 0);
    assert_eq!(result.individual_results[0].model_id, "model-first");
    assert_eq!(result.individual_results[1].model_id, "model-second");
    assert_eq!(result.individual_results[2].model_id, "model-third");

    // Each model gets its own port (sequential ports)
    // This proves sequential execution by port assignment
    assert_eq!(result.individual_results[0].result.telemetry.port, Some(9000));
    assert_eq!(result.individual_results[1].result.telemetry.port, Some(9001));
    assert_eq!(result.individual_results[2].result.telemetry.port, Some(9002));
}

// ============================================================================
// H6-T3: Individual results remain independently addressable (H6-3)
// ============================================================================

#[test]
fn test_h6_t3_independent_results_addressable() {
    let runner = make_batch_runner();
    let executor = BatchMockExecutor {
        responses: vec![
            MockResponse { output: "alpha output", token_count: 50, duration_ms: 100, fail: false },
            MockResponse { output: "beta output", token_count: 100, duration_ms: 200, fail: false },
            MockResponse { output: "gamma output", token_count: 150, duration_ms: 300, fail: false },
        ],
    };

    let input = BatchQualificationInput {
        batch_id: "batch-ind".to_string(),
        targets: vec![
            make_target(0, "model-alpha"),
            make_target(1, "model-beta"),
            make_target(2, "model-gamma"),
        ],
        global_custom_rules: vec![],
        created_at: "2026-01-01".to_string(),
    };

    let result = runner.run_batch(&input, &executor, 9000);

    // Each result has its own model identity
    assert_eq!(result.individual_results[0].model_id, "model-alpha");
    assert_eq!(result.individual_results[1].model_id, "model-beta");
    assert_eq!(result.individual_results[2].model_id, "model-gamma");

    // Each result has its own run_id
    let ids: Vec<&str> = result.individual_results.iter().map(|r| r.result.run_id.as_str()).collect();
    assert_ne!(ids[0], ids[1]);
    assert_ne!(ids[1], ids[2]);

    // Each result has its own raw_output
    assert_eq!(result.individual_results[0].result.raw_output.as_deref(), Some("alpha output"));
    assert_eq!(result.individual_results[1].result.raw_output.as_deref(), Some("beta output"));
    assert_eq!(result.individual_results[2].result.raw_output.as_deref(), Some("gamma output"));
}

// ============================================================================
// H6-T4: Aggregate results preserve evidence provenance (H6-4)
// ============================================================================

#[test]
fn test_h6_t4_aggregate_preserves_evidence_provenance() {
    let runner = make_batch_runner();
    let executor = BatchMockExecutor {
        responses: vec![MockResponse {
            output: "output", token_count: 50, duration_ms: 100, fail: false,
        }],
    };

    let input = BatchQualificationInput {
        batch_id: "batch-prov".to_string(),
        targets: vec![
            make_target(0, "model-a"),
            make_target(1, "model-b"),
        ],
        global_custom_rules: vec![],
        created_at: "2026-01-01".to_string(),
    };

    let result = runner.run_batch(&input, &executor, 9000);

    assert_eq!(result.aggregate.total_targets, 2);
    assert_eq!(result.aggregate.completed, 2);
    assert_eq!(result.aggregate.failed, 0);
    assert!(result.aggregate.total_duration_ms > 0);

    // Evidence references exist for each completed model
    assert_eq!(result.aggregate.evidence_references.len(), 2);
    assert!(result.aggregate.evidence_references[0].contains("model-a"));
    assert!(result.aggregate.evidence_references[1].contains("model-b"));
}

// ============================================================================
// H6-T5: One failing model does not corrupt unrelated results (H6-5)
// ============================================================================

#[test]
fn test_h6_t5_failure_isolation() {
    let runner = make_batch_runner();
    // Model at index 1 fails
    let executor = BatchMockExecutor {
        responses: vec![
            MockResponse { output: "good output", token_count: 50, duration_ms: 100, fail: false },
            MockResponse { output: "bad output", token_count: 0, duration_ms: 0, fail: true },
            MockResponse { output: "good output 2", token_count: 75, duration_ms: 150, fail: false },
        ],
    };

    let input = BatchQualificationInput {
        batch_id: "batch-fail".to_string(),
        targets: vec![
            make_target(0, "model-good"),
            make_target(1, "model-bad"),
            make_target(2, "model-good-2"),
        ],
        global_custom_rules: vec![],
        created_at: "2026-01-01".to_string(),
    };

    let result = runner.run_batch(&input, &executor, 9000);

    assert_eq!(result.individual_results.len(), 3);
    assert_eq!(result.aggregate.total_targets, 3);

    // Model 0 succeeded
    assert_eq!(result.individual_results[0].model_id, "model-good");
    assert!(result.individual_results[0].state.is_success());

    // Model 1 failed
    assert_eq!(result.individual_results[1].model_id, "model-bad");
    assert!(result.individual_results[1].state.is_failure());

    // Model 2 still succeeded (failure isolation)
    assert_eq!(result.individual_results[2].model_id, "model-good-2");
    assert!(result.individual_results[2].state.is_success());

    // Aggregate counts
    assert_eq!(result.aggregate.completed, 2);
    assert_eq!(result.aggregate.failed, 1);

    // Evidence references only from successful models
    assert_eq!(result.aggregate.evidence_references.len(), 2);
}

// ============================================================================
// H6-T6: Batch qualification cannot approve capabilities (NEGATIVE PROOF)
// ============================================================================

#[test]
fn test_h6_t6_batch_cannot_approve_capabilities() {
    let runner = make_batch_runner();
    let executor = BatchMockExecutor {
        responses: vec![MockResponse {
            output: "output", token_count: 50, duration_ms: 100, fail: false,
        }],
    };

    let input = BatchQualificationInput {
        batch_id: "batch-np1".to_string(),
        targets: vec![make_target(0, "model-a")],
        global_custom_rules: vec![],
        created_at: "2026-01-01".to_string(),
    };

    let result = runner.run_batch(&input, &executor, 9000);

    // Structural proof: no capability fields exist
    assert!(result.assert_no_capability_data().is_ok());

    // JSON structural proof
    let json = serde_json::to_value(&result).unwrap();
    assert!(json.get("manifest_id").is_none(), "Batch result must not contain manifest_id");
    assert!(json.get("manifest_status").is_none(), "Batch result must not contain manifest_status");
    assert!(json.get("approved_roles").is_none(), "Batch result must not contain approved_roles");
}

// ============================================================================
// H6-T7: Batch failures cannot mutate router policy (NEGATIVE PROOF)
// ============================================================================

#[test]
fn test_h6_t7_batch_failures_cannot_mutate_router() {
    let runner = make_batch_runner();
    let executor = BatchMockExecutor {
        responses: vec![MockResponse {
            output: "", token_count: 0, duration_ms: 0, fail: true,
        }],
    };

    // All models fail
    let input = BatchQualificationInput {
        batch_id: "batch-np2".to_string(),
        targets: vec![
            make_target(0, "model-fail-a"),
            make_target(1, "model-fail-b"),
        ],
        global_custom_rules: vec![],
        created_at: "2026-01-01".to_string(),
    };

    let result = runner.run_batch(&input, &executor, 9000);

    assert_eq!(result.aggregate.failed, 2);
    assert_eq!(result.aggregate.completed, 0);

    // Structural proof: no router fields
    let json = serde_json::to_value(&result).unwrap();
    assert!(json.get("projections").is_none(), "Batch failures must not create projections");
    assert!(json.get("router_eligible").is_none(), "Batch failures must not affect router eligibility");
    assert!(json.get("routing_status").is_none(), "Batch failures must not change routing status");
}

// ============================================================================
// H6-T8: Aggregated evidence cannot become a decision (NEGATIVE PROOF)
// ============================================================================

#[test]
fn test_h6_t8_aggregated_evidence_not_decision() {
    let runner = make_batch_runner();
    let executor = BatchMockExecutor {
        responses: vec![MockResponse {
            output: "output", token_count: 50, duration_ms: 100, fail: false,
        }],
    };

    let input = BatchQualificationInput {
        batch_id: "batch-np3".to_string(),
        targets: vec![make_target(0, "model-a")],
        global_custom_rules: vec![],
        created_at: "2026-01-01".to_string(),
    };

    let result = runner.run_batch(&input, &executor, 9000);

    // Aggregate summary is evidence metadata — not a decision
    assert_eq!(result.aggregate.completed, 1);
    assert_eq!(result.aggregate.failed, 0);

    // JSON structural proof: no decision fields
    let json = serde_json::to_value(&result).unwrap();
    assert!(json.get("decision_id").is_none(), "Aggregated evidence must not contain decision_id");
    assert!(json.get("decision_type").is_none(), "Aggregated evidence must not contain decision_type");
}

// ============================================================================
// H6-T9: Batch output cannot bypass Owner authority (NEGATIVE PROOF)
// ============================================================================

#[test]
fn test_h6_t9_batch_cannot_bypass_owner_authority() {
    let runner = make_batch_runner();
    let executor = BatchMockExecutor {
        responses: vec![MockResponse {
            output: "perfect output", token_count: 1000, duration_ms: 50, fail: false,
        }],
    };

    // Even with perfect execution, batch result has no authority
    let input = BatchQualificationInput {
        batch_id: "batch-np4".to_string(),
        targets: vec![make_target(0, "model-perfect")],
        global_custom_rules: vec![],
        created_at: "2026-01-01".to_string(),
    };

    let result = runner.run_batch(&input, &executor, 9000);

    assert!(result.individual_results[0].state.is_success());
    assert!(result.assert_no_capability_data().is_ok());

    // No authority fields regardless of success
    let json = serde_json::to_value(&result).unwrap();
    assert!(json.get("approved").is_none(), "Batch success must not create approval");
    assert!(json.get("rejected").is_none(), "Batch success must not create rejection");
    assert!(json.get("owner_decision").is_none(), "Batch must not create Owner decision");
}

// ============================================================================
// H6-T10: Batch content hash is deterministic
// ============================================================================

#[test]
fn test_h6_t10_batch_content_hash_deterministic() {
    let result = BatchQualificationResult {
        batch_id: "batch-hash".to_string(),
        model_order: vec!["model-a".to_string(), "model-b".to_string()],
        individual_results: vec![],
        aggregate: librarian_core::qualification::batch::AggregateBatchSummary {
            total_targets: 2,
            completed: 2,
            failed: 0,
            total_duration_ms: 200,
            evidence_references: vec!["run:abc:model-a".to_string(), "run:def:model-b".to_string()],
            content_hash: String::new(),
        },
        started_at: "2026-01-01T00:00:00Z".to_string(),
        completed_at: "2026-01-01T00:00:01Z".to_string(),
        content_hash: String::new(),
    };

    let h1 = result.compute_content_hash().unwrap();
    let h2 = result.compute_content_hash().unwrap();
    assert_eq!(h1, h2);
}

// ============================================================================
// H6-T11: Batch input serialization round-trip
// ============================================================================

#[test]
fn test_h6_t11_batch_input_serialization_roundtrip() {
    let input = BatchQualificationInput {
        batch_id: "batch-ser".to_string(),
        targets: vec![
            make_target(0, "model-a"),
            make_target(1, "model-b"),
        ],
        global_custom_rules: vec![],
        created_at: "2026-01-01".to_string(),
    };

    let json = serde_json::to_string(&input).unwrap();
    let parsed: BatchQualificationInput = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.batch_id, "batch-ser");
    assert_eq!(parsed.targets.len(), 2);
}

// ============================================================================
// H6-T12: Batch result serialization round-trip
// ============================================================================

#[test]
fn test_h6_t12_batch_result_serialization_roundtrip() {
    let runner = make_batch_runner();
    let executor = BatchMockExecutor {
        responses: vec![MockResponse {
            output: "output", token_count: 50, duration_ms: 100, fail: false,
        }],
    };

    let input = BatchQualificationInput {
        batch_id: "batch-ser2".to_string(),
        targets: vec![make_target(0, "model-a")],
        global_custom_rules: vec![],
        created_at: "2026-01-01".to_string(),
    };

    let result = runner.run_batch(&input, &executor, 9000);
    let json = serde_json::to_string(&result).unwrap();
    let parsed: BatchQualificationResult = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.batch_id, "batch-ser2");
    assert_eq!(parsed.individual_results.len(), 1);
}

// ============================================================================
// H6-T13: Empty batch is valid (no targets = no results)
// ============================================================================

#[test]
fn test_h6_t13_empty_batch_is_valid() {
    let runner = make_batch_runner();
    let executor = BatchMockExecutor { responses: vec![] };

    let input = BatchQualificationInput {
        batch_id: "batch-empty".to_string(),
        targets: vec![],
        global_custom_rules: vec![],
        created_at: "2026-01-01".to_string(),
    };

    let result = runner.run_batch(&input, &executor, 9000);
    assert_eq!(result.individual_results.len(), 0);
    assert_eq!(result.aggregate.total_targets, 0);
    assert_eq!(result.aggregate.completed, 0);
    assert_eq!(result.aggregate.failed, 0);
}

// ============================================================================
// H6-T14: Explicit model ordering preserved in batch result
// ============================================================================

#[test]
fn test_h6_t14_explicit_model_ordering_preserved() {
    let runner = make_batch_runner();
    let executor = BatchMockExecutor {
        responses: vec![MockResponse {
            output: "output", token_count: 50, duration_ms: 100, fail: false,
        }],
    };

    // Out-of-position order (position 0 has model-c, position 1 has model-a, etc.)
    let input = BatchQualificationInput {
        batch_id: "batch-order".to_string(),
        targets: vec![
            BatchTarget { position: 0, model_id: "model-c".to_string(), request: make_request("model-c"), custom_rules: vec![] },
            BatchTarget { position: 1, model_id: "model-a".to_string(), request: make_request("model-a"), custom_rules: vec![] },
            BatchTarget { position: 2, model_id: "model-b".to_string(), request: make_request("model-b"), custom_rules: vec![] },
        ],
        global_custom_rules: vec![],
        created_at: "2026-01-01".to_string(),
    };

    let result = runner.run_batch(&input, &executor, 9000);

    // model_order should match target order (explicit ordering)
    assert_eq!(result.model_order, vec!["model-c", "model-a", "model-b"]);
    assert_eq!(result.individual_results[0].model_id, "model-c");
    assert_eq!(result.individual_results[1].model_id, "model-a");
    assert_eq!(result.individual_results[2].model_id, "model-b");
}

// ============================================================================
// H6-T15: Multiple batches with same input produce same structure
// ============================================================================

#[test]
fn test_h6_t15_batch_deterministic_structure_across_runs() {
    let runner = make_batch_runner();
    let executor = BatchMockExecutor {
        responses: vec![MockResponse {
            output: "deterministic output", token_count: 50, duration_ms: 100, fail: false,
        }],
    };

    let input = BatchQualificationInput {
        batch_id: "batch-det".to_string(),
        targets: vec![
            make_target(0, "model-a"),
            make_target(1, "model-b"),
        ],
        global_custom_rules: vec![],
        created_at: "2026-01-01".to_string(),
    };

    let r1 = runner.run_batch(&input, &executor, 9000);
    let r2 = runner.run_batch(&input, &executor, 9000);

    // Same count and ordering
    assert_eq!(r1.individual_results.len(), r2.individual_results.len());
    assert_eq!(r1.model_order, r2.model_order);

    // Same model IDs in each result
    for i in 0..r1.individual_results.len() {
        assert_eq!(r1.individual_results[i].model_id, r2.individual_results[i].model_id);
    }

    // Same aggregate totals
    assert_eq!(r1.aggregate.total_targets, r2.aggregate.total_targets);
    assert_eq!(r1.aggregate.completed, r2.aggregate.completed);
    assert_eq!(r1.aggregate.failed, r2.aggregate.failed);
}
