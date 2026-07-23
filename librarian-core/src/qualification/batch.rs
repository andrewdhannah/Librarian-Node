//! Batch qualification runner — sequential multi-model qualification.
//!
//! Extends single-model qualification to batch qualification with:
//! - Sequential execution only (no parallelism)
//! - Explicit model ordering
//! - Independent model results
//! - No cross-model state leakage
//! - Deterministic aggregation
//! - Failure isolation between models
//!
//! Critical invariants:
//!   Batch qualification cannot approve capabilities.
//!   Batch failures cannot mutate router policy.
//!   Aggregated evidence cannot become a decision.
//!   Qualification output cannot bypass Owner authority.
//!   Custom rules remain evidence-only.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::custom_executor::{apply_custom_rules, CustomRuleDefinition, CustomRuleExecutor};
use super::run_result::QualificationRunResult;
use super::run_state::RunState;
use super::runner::{QualificationRunner, RuntimeExecutor};
use librarian_contracts::qualification_request::QualificationRequest;

/// A single target in a batch qualification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchTarget {
    /// Position in the batch (deterministic ordering).
    pub position: usize,

    /// Model ID for identification.
    pub model_id: String,

    /// The qualification request to execute.
    pub request: QualificationRequest,

    /// Optional custom rules to apply to this target's result.
    pub custom_rules: Vec<CustomRuleDefinition>,
}

/// Input for a batch qualification run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchQualificationInput {
    /// Unique batch identifier.
    pub batch_id: String,

    /// Ordered list of qualification targets.
    /// Models execute in this order, sequentially.
    pub targets: Vec<BatchTarget>,

    /// Global custom rules applied to ALL targets after execution.
    pub global_custom_rules: Vec<CustomRuleDefinition>,

    /// When the batch was created.
    pub created_at: String,
}

/// Result of an individual model in the batch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndividualBatchResult {
    /// Position in the batch (matches BatchTarget.position).
    pub position: usize,

    /// Model ID.
    pub model_id: String,

    /// The full qualification run result.
    pub result: QualificationRunResult,

    /// Final run state (convenience).
    pub state: RunState,

    /// Error message if the model failed during batch execution.
    pub error_message: Option<String>,
}

/// Aggregate summary of all batch results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AggregateBatchSummary {
    /// Total number of targets.
    pub total_targets: usize,

    /// Number of completed (successful) targets.
    pub completed: usize,

    /// Number of failed targets.
    pub failed: usize,

    /// Total duration across all runs in milliseconds.
    pub total_duration_ms: u64,

    /// Evidence references from all completed runs.
    pub evidence_references: Vec<String>,

    /// Batch content hash (deterministic).
    pub content_hash: String,
}

/// Complete batch qualification result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchQualificationResult {
    /// Batch identifier.
    pub batch_id: String,

    /// Ordered list of target model IDs.
    pub model_order: Vec<String>,

    /// Individual results (in execution order).
    pub individual_results: Vec<IndividualBatchResult>,

    /// Aggregate summary.
    pub aggregate: AggregateBatchSummary,

    /// When the batch started.
    pub started_at: String,

    /// When the batch completed.
    pub completed_at: String,

    /// Content hash for the batch.
    pub content_hash: String,
}

impl BatchQualificationResult {
    /// Compute a deterministic batch ID from input.
    pub fn compute_batch_id(targets: &[BatchTarget], created_at: &str) -> String {
        let mut hasher = Sha256::new();
        for target in targets {
            hasher.update(target.position.to_be_bytes());
            hasher.update(target.model_id.as_bytes());
        }
        hasher.update(created_at.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compute a deterministic content hash for the batch result.
    pub fn compute_content_hash(&self) -> Result<String> {
        let content = serde_json::json!({
            "batch_id": self.batch_id,
            "model_order": self.model_order,
            "result_count": self.individual_results.len(),
            "completed": self.aggregate.completed,
            "failed": self.aggregate.failed,
            "evidence_references": self.aggregate.evidence_references,
        });
        let json = content.to_string();
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Assert that this batch result contains no capability authority data.
    pub fn assert_no_capability_data(&self) -> Result<()> {
        // Structural proof: the fields are:
        // - batch_id, model_order (identifiers)
        // - individual_results (collection of QualificationRunResult — each asserts no capability data)
        // - aggregate summary (counts and evidence refs — NOT capability)
        // - started_at, completed_at (timestamps)
        // - content_hash (integrity)
        //
        // There are no fields for:
        // - role assignments
        // - capability status
        // - qualification decisions
        // - router eligibility
        // - Owner decisions

        // Additionally, each individual result also asserts no capability data
        for individual in &self.individual_results {
            individual.result.assert_no_capability_data()?;
        }

        Ok(())
    }
}

/// Batch qualification runner — executes multiple qualification runs sequentially.
///
/// Wraps the single-model QualificationRunner, adding batch management:
/// - Sequential execution
/// - Failure isolation
/// - Deterministic aggregation
/// - Custom rule application
pub struct BatchQualificationRunner {
    /// Inner single-model runner.
    inner: QualificationRunner,
}

impl BatchQualificationRunner {
    /// Create a new batch runner wrapping the inner runner.
    pub fn new(inner: QualificationRunner) -> Self {
        Self { inner }
    }

    /// Run a batch of qualification targets sequentially.
    ///
    /// Design properties:
    /// - Each target runs independently via the inner runner
    /// - Failures are isolated — one failing model does not abort the batch
    /// - Custom rules (per-target and global) are applied after each run
    /// - Results are aggregated deterministically
    pub fn run_batch<E: RuntimeExecutor>(
        &self,
        input: &BatchQualificationInput,
        executor: &E,
        port_base: u16,
    ) -> BatchQualificationResult {
        let started_at = chrono::Utc::now().to_rfc3339();
        let custom_executor = CustomRuleExecutor::default();

        let mut total_duration_ms: u64 = 0;
        let mut evidence_references: Vec<String> = Vec::new();

        // Validate batch
        if input.targets.is_empty() {
            // Return empty batch result
            return BatchQualificationResult {
                batch_id: input.batch_id.clone(),
                model_order: vec![],
                individual_results: vec![],
                aggregate: AggregateBatchSummary {
                    total_targets: 0,
                    completed: 0,
                    failed: 0,
                    total_duration_ms: 0,
                    evidence_references: vec![],
                    content_hash: String::new(),
                },
                started_at: started_at.clone(),
                completed_at: chrono::Utc::now().to_rfc3339(),
                content_hash: String::new(),
            };
        }

        // Execute targets sequentially, collecting raw results
        let mut raw_results: Vec<(usize, String, QualificationRunResult, RunState, Option<String>)> = Vec::new();

        for (_idx, target) in input.targets.iter().enumerate() {
            let model_port = port_base + target.position as u16;
            let result = self.inner.run(&target.request, executor, model_port, None);

            // Accumulate duration
            if let Some(gen_ms) = result.telemetry.generation_duration_ms {
                total_duration_ms += gen_ms;
            }

            // Build evidence reference from successful runs
            if result.state == RunState::Completed {
                evidence_references.push(format!(
                    "run:{}:{}",
                    result.run_id, result.model_id
                ));
            }

            let state = result.state.clone();
            let error_message = result.error_message.clone();
            let model_id = result.model_id.clone();

            raw_results.push((target.position, model_id, result, state, error_message));
        }

        // Apply custom rules to each result and build final results
        let mut final_results = Vec::new();
        for (_idx, target) in input.targets.iter().enumerate() {
            let (pos, model_id, mut result, state, error_message) = raw_results.remove(0);

            // Apply per-target custom rules
            if !target.custom_rules.is_empty() {
                apply_custom_rules(&mut result, &custom_executor, &target.custom_rules);
            }

            // Apply global custom rules
            if !input.global_custom_rules.is_empty() {
                apply_custom_rules(&mut result, &custom_executor, &input.global_custom_rules);
            }

            final_results.push(IndividualBatchResult {
                position: pos,
                model_id,
                result,
                state,
                error_message,
            });
        }

        let completed = final_results
            .iter()
            .filter(|r| r.state.is_success())
            .count();
        let failed = final_results
            .iter()
            .filter(|r| r.state.is_failure())
            .count();

        let model_order: Vec<String> = input
            .targets
            .iter()
            .map(|t| t.model_id.clone())
            .collect();

        let mut batch_result = BatchQualificationResult {
            batch_id: input.batch_id.clone(),
            model_order,
            individual_results: final_results,
            aggregate: AggregateBatchSummary {
                total_targets: input.targets.len(),
                completed,
                failed,
                total_duration_ms,
                evidence_references,
                content_hash: String::new(),
            },
            started_at: started_at.clone(),
            completed_at: chrono::Utc::now().to_rfc3339(),
            content_hash: String::new(),
        };

        // Compute content hash
        batch_result.aggregate.content_hash = batch_result
            .compute_content_hash()
            .unwrap_or_default();
        batch_result.content_hash = batch_result
            .compute_content_hash()
            .unwrap_or_default();

        batch_result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_contracts::common::{
        PacketConstraints, PacketExecutionConfig, PacketModelIdentity,
    };

    struct MockExecutor {
        responses: Vec<MockResponse>,
    }

    struct MockResponse {
        output: &'static str,
        token_count: u32,
        duration_ms: u64,
        error: Option<&'static str>,
    }

    impl RuntimeExecutor for MockExecutor {
        fn execute(
            &self,
            _port: u16,
            _prompt: &str,
            _max_tokens: u32,
            _temperature: f64,
            _timeout_secs: u64,
        ) -> Result<crate::qualification::runner::ExecutionResponse> {
            // Round-robin through responses
            let idx = (_port as usize - 9000) % self.responses.len();
            let resp = &self.responses[idx];
            if let Some(err) = resp.error {
                anyhow::bail!("{}", err);
            }
            Ok(crate::qualification::runner::ExecutionResponse {
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

    fn make_runner() -> BatchQualificationRunner {
        let inner = QualificationRunner::new("/tmp/fixtures");
        BatchQualificationRunner::new(inner)
    }

    // H6-U1: Batch ID is deterministic
    #[test]
    fn test_batch_id_deterministic() {
        let targets = vec![
            make_target(0, "model-a"),
            make_target(1, "model-b"),
        ];
        let id1 = BatchQualificationResult::compute_batch_id(&targets, "2026-01-01");
        let id2 = BatchQualificationResult::compute_batch_id(&targets, "2026-01-01");
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 64);
    }

    // H6-U2: Batch content hash function is deterministic (same state → same hash)
    #[test]
    fn test_batch_content_hash_deterministic() {
        let result = BatchQualificationResult {
            batch_id: "test-batch".to_string(),
            model_order: vec!["model-a".to_string()],
            individual_results: vec![],
            aggregate: AggregateBatchSummary {
                total_targets: 1,
                completed: 1,
                failed: 0,
                total_duration_ms: 100,
                evidence_references: vec!["run:abc123:model-a".to_string()],
                content_hash: String::new(),
            },
            started_at: "2026-01-01".to_string(),
            completed_at: "2026-01-01".to_string(),
            content_hash: String::new(),
        };

        let h1 = result.compute_content_hash().unwrap();
        let h2 = result.compute_content_hash().unwrap();
        assert_eq!(h1, h2);
    }

    // H6-U3: Batch result has no capability data
    #[test]
    fn test_no_capability_data() {
        let runner = make_runner();
        let executor = MockExecutor {
            responses: vec![MockResponse {
                output: "output",
                token_count: 50,
                duration_ms: 100,
                error: None,
            }],
        };
        let input = BatchQualificationInput {
            batch_id: "batch-002".to_string(),
            targets: vec![make_target(0, "model-a")],
            global_custom_rules: vec![],
            created_at: "2026-01-01".to_string(),
        };
        let result = runner.run_batch(&input, &executor, 9000);
        assert!(result.assert_no_capability_data().is_ok());
    }

    // H6-U4: Compute content hash is deterministic
    #[test]
    fn test_content_hash_deterministic() {
        let mut result = BatchQualificationResult {
            batch_id: "test".to_string(),
            model_order: vec!["model-a".to_string()],
            individual_results: vec![],
            aggregate: AggregateBatchSummary {
                total_targets: 0,
                completed: 0,
                failed: 0,
                total_duration_ms: 0,
                evidence_references: vec![],
                content_hash: String::new(),
            },
            started_at: "2026-01-01".to_string(),
            completed_at: "2026-01-01".to_string(),
            content_hash: String::new(),
        };
        let h1 = result.compute_content_hash().unwrap();
        let h2 = result.compute_content_hash().unwrap();
        assert_eq!(h1, h2);
        result.aggregate.completed = 1;
        let h3 = result.compute_content_hash().unwrap();
        assert_ne!(h1, h3);
    }

    // H6-U5: Empty batch produces empty result
    #[test]
    fn test_empty_batch() {
        let runner = make_runner();
        let executor = MockExecutor {
            responses: vec![],
        };
        let input = BatchQualificationInput {
            batch_id: "batch-empty".to_string(),
            targets: vec![],
            global_custom_rules: vec![],
            created_at: "2026-01-01".to_string(),
        };
        let result = runner.run_batch(&input, &executor, 9000);
        assert_eq!(result.individual_results.len(), 0);
        assert_eq!(result.aggregate.total_targets, 0);
    }
}
