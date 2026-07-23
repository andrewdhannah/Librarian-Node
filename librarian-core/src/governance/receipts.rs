//! # Receipt Generation
//!
//! Portable receipt generation using contract types.
//! Receipts form the governance spine — append-only, versioned, evidence-linked.

use anyhow::Result;
use librarian_contracts::prelude::*;
use uuid::Uuid;

use super::db::GovernanceDb;

/// Receipt generator for governance operations.
pub struct ReceiptGenerator {
    db: GovernanceDb,
}

impl ReceiptGenerator {
    /// Create a new receipt generator.
    pub fn new(db: GovernanceDb) -> Self {
        Self { db }
    }

    /// Generate a sprint authorization receipt.
    pub fn authorize_sprint(
        &self,
        work_order_id: &str,
        authorized_by: &str,
        scope: &str,
        repository: &str,
        parent_receipts: Vec<String>,
    ) -> Result<SprintAuthorizationReceipt> {
        let now = chrono::Utc::now().to_rfc3339();
        let receipt = SprintAuthorizationReceipt {
            receipt_id: format!("AR-{}-{}", work_order_id, chrono::Utc::now().format("%Y%m%d")),
            receipt_type: ReceiptType::SprintAuthorization,
            work_order_id: work_order_id.to_string(),
            authorized_at: now.clone(),
            authorized_by: authorized_by.to_string(),
            repository: repository.to_string(),
            scope: scope.to_string(),
            capability_expansion: None,
            migration_code: false,
            schema_version: RECEIPT_CONTRACT_VERSION.into(),
        };

        // Also store as a generic receipt
        let generic = Receipt {
            receipt_id: receipt.receipt_id.clone(),
            receipt_type: ReceiptType::SprintAuthorization,
            receipt_version: "1.0".into(),
            action: "authorize_sprint".into(),
            initiated_by: authorized_by.to_string(),
            authorized_by: Some(authorized_by.to_string()),
            summary: format!("Authorized {}: {}", work_order_id, scope),
            recorded_at: now,
            parent_receipt_ids: parent_receipts,
            evidence_ids: vec![],
            project_id: Some(repository.to_string()),
            schema_version: RECEIPT_CONTRACT_VERSION.into(),
        };
        self.db.store_receipt(&generic)?;

        Ok(receipt)
    }

    /// Generate a sprint seal receipt.
    pub fn seal_sprint(
        &self,
        work_order_id: &str,
        sealed_by: &str,
        summary: &str,
        evidence_ids: Vec<String>,
        parent_receipt_ids: Vec<String>,
    ) -> Result<EquivalenceReceipt> {
        let now = chrono::Utc::now().to_rfc3339();
        let receipt = EquivalenceReceipt {
            receipt_id: format!("SR-{}-{}", work_order_id, chrono::Utc::now().format("%Y%m%d")),
            equivalence_run_id: String::new(),
            baseline_id: String::new(),
            candidate_id: String::new(),
            result: "SEALED".into(),
            checks_passed: 0,
            checks_failed: 0,
            completed_at: now.clone(),
            evidence_path: String::new(),
            schema_version: RECEIPT_CONTRACT_VERSION.into(),
        };

        let generic = Receipt {
            receipt_id: receipt.receipt_id.clone(),
            receipt_type: ReceiptType::SprintSeal,
            receipt_version: "1.0".into(),
            action: "seal_sprint".into(),
            initiated_by: sealed_by.to_string(),
            authorized_by: Some(sealed_by.to_string()),
            summary: summary.to_string(),
            recorded_at: now,
            parent_receipt_ids: parent_receipt_ids,
            evidence_ids,
            project_id: None,
            schema_version: RECEIPT_CONTRACT_VERSION.into(),
        };
        self.db.store_receipt(&generic)?;

        Ok(receipt)
    }

    /// Generate an equivalence check receipt.
    pub fn equivalence_check(
        &self,
        baseline_id: &str,
        candidate_id: &str,
        result: &str,
        checks_passed: u32,
        checks_failed: u32,
        evidence_path: &str,
    ) -> Result<EquivalenceReceipt> {
        let now = chrono::Utc::now().to_rfc3339();
        let receipt = EquivalenceReceipt {
            receipt_id: format!("EQ-{}", Uuid::new_v4()),
            equivalence_run_id: format!("eq-run-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S")),
            baseline_id: baseline_id.to_string(),
            candidate_id: candidate_id.to_string(),
            result: result.to_string(),
            checks_passed,
            checks_failed,
            completed_at: now.clone(),
            evidence_path: evidence_path.to_string(),
            schema_version: RECEIPT_CONTRACT_VERSION.into(),
        };

        let generic = Receipt {
            receipt_id: receipt.receipt_id.clone(),
            receipt_type: ReceiptType::Equivalence,
            receipt_version: "1.0".into(),
            action: "equivalence_check".into(),
            initiated_by: "equivalence-harness".into(),
            authorized_by: None,
            summary: format!("Equivalence {}: {} vs {} ({} pass, {} fail)",
                result, baseline_id, candidate_id, checks_passed, checks_failed),
            recorded_at: now.clone(),
            parent_receipt_ids: vec![],
            evidence_ids: vec![evidence_path.to_string()],
            project_id: None,
            schema_version: RECEIPT_CONTRACT_VERSION.into(),
        };
        self.db.store_receipt(&generic)?;

        Ok(receipt)
    }

    /// Get the number of receipts stored.
    pub fn receipt_count(&self) -> Result<u64> {
        // In production this would query the governance DB.
        // Placeholder — returns 0.
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> ReceiptGenerator {
        let db = GovernanceDb::open_in_memory().unwrap();
        ReceiptGenerator::new(db)
    }

    #[test]
    fn test_authorize_sprint() {
        let gen = setup();
        let receipt = gen.authorize_sprint(
            "WO-004", "Owner", "Rust Core Governance Port", "Librarian-Node", vec![]
        ).unwrap();
        assert!(receipt.receipt_id.contains("AR-WO-004"));
        assert_eq!(receipt.authorized_by, "Owner");
        assert_eq!(receipt.repository, "Librarian-Node");
    }

    #[test]
    fn test_equivalence_receipt() {
        let gen = setup();
        let receipt = gen.equivalence_check(
            "swift-core-macos-v1.0",
            "rust-core-win-v0.1",
            "PASS",
            7,
            0,
            "evidence/equivalence/eq-run-001/",
        ).unwrap();
        assert_eq!(receipt.baseline_id, "swift-core-macos-v1.0");
        assert_eq!(receipt.result, "PASS");
        assert_eq!(receipt.checks_passed, 7);
    }

    #[test]
    fn test_seal_sprint() {
        let gen = setup();
        let receipt = gen.seal_sprint(
            "WO-004", "agent", "Rust Core implementation complete",
            vec!["evt-001".into()],
            vec![],
        ).unwrap();
        assert!(receipt.receipt_id.contains("SR-WO-004"));
    }
}
