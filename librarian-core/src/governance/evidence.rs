//! # Evidence Generation
//!
//! Portable evidence generation using contract types.
//! Produces evidence records from governance operations that can be
//! verified by any platform implementation.

use anyhow::Result;
use librarian_contracts::evidence::*;
use librarian_contracts::serialization::hash_canonical;
use uuid::Uuid;

use super::db::GovernanceDb;

/// Evidence generator for governance operations.
pub struct EvidenceGenerator {
    db: GovernanceDb,
    produced_by: String,
}

impl EvidenceGenerator {
    /// Create a new evidence generator.
    pub fn new(db: GovernanceDb, produced_by: &str) -> Self {
        Self {
            db,
            produced_by: produced_by.to_string(),
        }
    }

    /// Generate an evidence record from any serializable payload.
    pub fn generate<T: serde::Serialize>(
        &self,
        category: EvidenceCategory,
        description: &str,
        payload: &T,
    ) -> Result<EvidenceRecord> {
        let now = chrono::Utc::now().to_rfc3339();
        let payload_value = serde_json::to_value(payload)?;
        let payload_hash = hash_canonical(payload)?;

        let record = EvidenceRecord {
            record_id: format!("evt-{}", Uuid::new_v4()),
            category,
            description: description.to_string(),
            payload: payload_value,
            payload_hash,
            recorded_at: now,
            produced_by: self.produced_by.clone(),
            schema_version: EVIDENCE_CONTRACT_VERSION.into(),
        };

        self.db.store_evidence(&record)?;
        Ok(record)
    }

    /// Generate evidence for a lifecycle cursor transition.
    pub fn generate_transition_evidence(
        &self,
        project_id: &str,
        from_state: &str,
        to_state: &str,
        reason: &str,
    ) -> Result<EvidenceRecord> {
        let payload = serde_json::json!({
            "project_id": project_id,
            "from_state": from_state,
            "to_state": to_state,
            "reason": reason,
        });
        self.generate(
            EvidenceCategory::ContractValidation,
            &format!("Lifecycle transition: {} → {}", from_state, to_state),
            &payload,
        )
    }

    /// Generate evidence for a custody operation.
    pub fn generate_custody_evidence(
        &self,
        document_reference: &str,
        action: &str,
        node_id: &str,
    ) -> Result<EvidenceRecord> {
        let payload = serde_json::json!({
            "document_reference": document_reference,
            "action": action,
            "node_id": node_id,
        });
        self.generate(
            EvidenceCategory::ContractValidation,
            &format!("Custody action: {} on {}", action, document_reference),
            &payload,
        )
    }

    /// Generate evidence for an equivalence check.
    pub fn generate_equivalence_evidence(
        &self,
        check_id: &str,
        baseline: &str,
        candidate: &str,
        passed: bool,
        details: &serde_json::Value,
    ) -> Result<EvidenceRecord> {
        let payload = serde_json::json!({
            "check_id": check_id,
            "baseline": baseline,
            "candidate": candidate,
            "passed": passed,
            "details": details,
        });
        self.generate(
            EvidenceCategory::ContractValidation,
            &format!("Equivalence check {}: {}", check_id, if passed { "PASS" } else { "FAIL" }),
            &payload,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> EvidenceGenerator {
        let db = GovernanceDb::open_in_memory().unwrap();
        EvidenceGenerator::new(db, "WO-004-test")
    }

    #[test]
    fn test_generate_evidence() {
        let gen = setup();
        let payload = serde_json::json!({"test": "data", "number": 42});
        let record = gen.generate(EvidenceCategory::TestResult, "Test evidence", &payload).unwrap();
        assert!(record.record_id.starts_with("evt-"));
        assert_eq!(record.category, EvidenceCategory::TestResult);
        assert_eq!(record.produced_by, "WO-004-test");
        assert_eq!(record.payload, payload);
    }

    #[test]
    fn test_transition_evidence() {
        let gen = setup();
        let record = gen.generate_transition_evidence("test-project", "Install", "Initialize", "Phase 1 complete").unwrap();
        assert_eq!(record.category, EvidenceCategory::ContractValidation);
        assert!(record.description.contains("Install"));
        assert!(record.description.contains("Initialize"));
    }

    #[test]
    fn test_custody_evidence() {
        let gen = setup();
        let record = gen.generate_custody_evidence("doc://test", "check-out", "node-1").unwrap();
        assert_eq!(record.category, EvidenceCategory::ContractValidation);
    }

    #[test]
    fn test_equivalence_evidence_pass() {
        let gen = setup();
        let details = serde_json::json!({"checks": 7, "passed": 7});
        let record = gen.generate_equivalence_evidence("EQ-001", "swift-core", "rust-core", true, &details).unwrap();
        assert!(record.description.contains("PASS"));
        assert_eq!(record.payload["check_id"], "EQ-001");
    }

    #[test]
    fn test_equivalence_evidence_fail() {
        let gen = setup();
        let details = serde_json::json!({"checks": 7, "passed": 6});
        let record = gen.generate_equivalence_evidence("EQ-001", "swift-core", "rust-core", false, &details).unwrap();
        assert!(record.description.contains("FAIL"));
    }
}
