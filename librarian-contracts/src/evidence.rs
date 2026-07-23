//! # Evidence Contract Types
//!
//! Evidence record types for the Librarian platform.
//! Maps to Swift `AgentExecutionEvidence`, evidence fixtures,
//! and evidence writer patterns.
//!
//! Evidence is append-only. State may change; evidence does not.

use serde::{Deserialize, Serialize};

/// Schema version for evidence contracts.
pub const EVIDENCE_CONTRACT_VERSION: &str = "1.0.0";

/// Evidence category — what kind of artifact this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceCategory {
    /// Tool execution evidence (from agent sessions).
    ToolExecution,
    /// Test result evidence.
    TestResult,
    /// Contract validation evidence.
    ContractValidation,
    /// Benchmark/performance evidence.
    Benchmark,
    /// Receipt evidence.
    Receipt,
    /// Audit evidence.
    Audit,
    /// Manual evidence (human-provided).
    Manual,
}

/// The authority decision for a tool execution.
/// Maps to Swift `AgentExecutionEvidence.authorityDecision`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityDecision {
    /// Execution was permitted.
    Permitted,
    /// Execution was blocked.
    Blocked,
    /// Execution was escalated for owner review.
    Escalated,
    /// An error occurred during authority check.
    Error,
}

/// Risk class for an action.
/// Maps to Swift `AgentExecutionEvidence.riskClass`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskClass {
    /// Read-only, no side effects.
    R0,
    /// Low risk.
    R1,
    /// Moderate risk.
    R2,
    /// High risk.
    R5,
}

/// A single evidence record for a tool execution or governance action.
/// Maps to Swift `AgentExecutionEvidence`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvidence {
    /// Unique evidence identifier.
    pub id: String,
    /// Agent session this evidence belongs to.
    pub session_id: String,
    /// Work packet associated with the session.
    pub work_packet_id: String,
    /// The tool or action name executed.
    pub tool: String,
    /// The target resource path or identifier.
    pub target: String,
    /// Risk class of the executed action.
    pub risk_class: RiskClass,
    /// Authority decision.
    pub authority_decision: AuthorityDecision,
    /// Whether execution succeeded.
    pub success: bool,
    /// Summary of output or result.
    pub output_summary: String,
    /// Error detail if execution failed or was blocked.
    pub error_detail: Option<String>,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Schema version.
    pub schema_version: String,
}

/// An evidence record that can be persisted as a JSON fixture.
/// Maps to the evidence-writer pattern from the Rust router.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRecord {
    /// Unique record identifier.
    pub record_id: String,
    /// Category of evidence.
    pub category: EvidenceCategory,
    /// Human-readable description.
    pub description: String,
    /// The evidence payload as JSON value.
    pub payload: serde_json::Value,
    /// SHA-256 hash of the payload for integrity verification.
    pub payload_hash: String,
    /// ISO 8601 timestamp.
    pub recorded_at: String,
    /// What produced this evidence.
    pub produced_by: String,
    /// Schema version.
    pub schema_version: String,
}

/// Evidence writer configuration.
/// Maps to the evidence path and deduplication pattern from the Rust router.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceWriterConfig {
    /// Directory path for evidence output.
    pub evidence_dir: String,
    /// Counter-based deduplication suffix separator.
    pub dedup_separator: String,
}

impl Default for EvidenceWriterConfig {
    fn default() -> Self {
        Self {
            evidence_dir: "evidence".into(),
            dedup_separator: "-".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_evidence_serde() {
        let evidence = ExecutionEvidence {
            id: "evt-001".into(),
            session_id: "session-001".into(),
            work_packet_id: "wp-001".into(),
            tool: "read_file".into(),
            target: "/path/to/file".into(),
            risk_class: RiskClass::R0,
            authority_decision: AuthorityDecision::Permitted,
            success: true,
            output_summary: "File read successfully".into(),
            error_detail: None,
            duration_ms: 42,
            timestamp: "2026-07-23T00:00:00Z".into(),
            schema_version: EVIDENCE_CONTRACT_VERSION.into(),
        };
        let json = serde_json::to_string(&evidence).unwrap();
        let deserialized: ExecutionEvidence = serde_json::from_str(&json).unwrap();
        assert_eq!(evidence.id, deserialized.id);
        assert_eq!(evidence.authority_decision, deserialized.authority_decision);
    }

    #[test]
    fn test_evidence_record_with_payload() {
        let record = EvidenceRecord {
            record_id: "rec-001".into(),
            category: EvidenceCategory::ContractValidation,
            description: "Contract equivalence check result".into(),
            payload: serde_json::json!({"passed": true, "checks": 7}),
            payload_hash: "abc123".into(),
            recorded_at: "2026-07-23T00:00:00Z".into(),
            produced_by: "equivalence-harness".into(),
            schema_version: EVIDENCE_CONTRACT_VERSION.into(),
        };
        let json = serde_json::to_string_pretty(&record).unwrap();
        assert!(json.contains("contract_validation"));
        assert!(json.contains("equivalence-harness"));
    }
}
