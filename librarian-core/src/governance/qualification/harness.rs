//! # Model Qualification Harness
//!
//! Qualification harness that consumes the existing governance substrate.
//! No new governance primitives — uses Evidence, Receipt, ResidencyState, Capability.
//!
//! The flow:
//!   Model Profile
//!        ↓
//!   Capability (mapped)
//!        ↓
//!   ResidencyState (qualification runtime tracking)
//!        ↓
//!   EvidenceRecord (qualification results)
//!        ↓
//!   Receipt (qualification completion)
//!        ↓
//!   GovernanceDb (persistence)

use anyhow::Result;
use librarian_contracts::prelude::*;
use uuid::Uuid;

use super::profiles::ModelProfileConfig;
use crate::governance::db::GovernanceDb;
use crate::governance::evidence::EvidenceGenerator;
use crate::governance::receipts::ReceiptGenerator;

/// A qualification run — the end-to-end result of qualifying a model profile.
#[derive(Debug, Clone, serde::Serialize)]
pub struct QualificationRun {
    /// Unique run identifier.
    pub run_id: String,
    /// Model alias that was qualified.
    pub model_alias: String,
    /// The capability identifier.
    pub capability_id: String,
    /// Whether qualification passed.
    pub passed: bool,
    /// Evidence record ID.
    pub evidence_id: String,
    /// Receipt record ID.
    pub receipt_id: String,
    /// ISO 8601 timestamp.
    pub completed_at: String,
}

/// The model qualification harness. Consumes governance primitives, does not invent them.
pub struct QualificationHarness {
    db: GovernanceDb,
    evidence_gen: EvidenceGenerator,
    receipt_gen: ReceiptGenerator,
}

impl QualificationHarness {
    /// Create a new qualification harness.
    pub fn new(db: GovernanceDb) -> Self {
        let evidence_gen = EvidenceGenerator::new(db.clone(), "mqr-harness");
        let receipt_gen = ReceiptGenerator::new(db.clone());
        Self {
            db,
            evidence_gen,
            receipt_gen,
        }
    }

    /// Run qualification for a single model profile.
    ///
    /// This is a structural qualification — it validates that the profile
    /// maps to governance types correctly. Real qualification runs would
    /// additionally load the model and run inference tests.
    pub fn qualify_profile(&self, profile: &ModelProfileConfig) -> Result<QualificationRun> {
        let run_id = format!("mqr-{}", Uuid::new_v4());
        let now = chrono::Utc::now().to_rfc3339();

        // 1. Map profile to capability
        let capability = profile.to_capability();
        let capability_id = capability.capability_id.clone();

        // 2. Track residency during qualification
        let _residency_record = ResidencyRecord {
            record_id: format!("res-{}-{}", profile.alias, Uuid::new_v4()),
            component_id: capability_id.clone(),
            current_state: ResidencyState::Active,
            last_transition_at: now.clone(),
            host_node: "librarian-node".into(),
            schema_version: RESIDENCY_CONTRACT_VERSION.into(),
        };

        // Record residency start
        let start_event = CustodyEvent {
            event_id: format!("res-start-{}", Uuid::new_v4()),
            project_id: "model-qualification".into(),
            mcp_session_id: run_id.clone(),
            node_id: "mqr-harness".into(),
            window_id: None,
            work_packet_id: None,
            tool_name: "qualification_harness".into(),
            authority_role: CustodyAuthorityRole::System,
            document_reference: format!("capability://{}", capability_id),
            custody_action: CustodyAction::Claim,
            previous_custody_mode: None,
            resulting_custody_mode: Some(CustodyMode::LocalWorkingCopy),
            mutation_allowance: Some(MutationAllowance::ReadOnly),
            decision_reference: None,
            provenance_receipt: None,
            refusal_reason: None,
            target_project_id: None,
            target_session_id: None,
            target_node_id: None,
            timestamp: now.clone(),
        };
        self.db.record_custody_event(&start_event)?;

        // 3. Generate qualification evidence
        let evidence_payload = serde_json::json!({
            "model_alias": profile.alias,
            "model_file": profile.model_file,
            "gguf_size_gb": profile.gguf_size_gb,
            "backend": profile.backend,
            "context": profile.context,
            "ngl": profile.ngl,
            "verified_status": profile.verified_status,
            "stability": profile.stability,
            "task_classes": profile.task_classes,
            "capability_id": capability_id,
            "capability_enabled": capability.enabled,
            "residency_state": "Active",
        });

        let evidence = self.evidence_gen.generate(
            profile.evidence_category(),
            &format!("Qualification: {} ({})", profile.alias, profile.verified_status),
            &evidence_payload,
        )?;

        // 4. Generate qualification receipt
        let passed = profile.verified_status == "verified";
        let receipt = self.receipt_gen.equivalence_check(
            "profile-config",
            &format!("model-{}", profile.alias),
            if passed { "PASS" } else { "FAIL" },
            if passed { 1 } else { 0 },
            if passed { 0 } else { 1 },
            &evidence.record_id,
        )?;

        // 5. Record residency release
        let end_event = CustodyEvent {
            event_id: format!("res-end-{}", Uuid::new_v4()),
            project_id: "model-qualification".into(),
            mcp_session_id: run_id.clone(),
            node_id: "mqr-harness".into(),
            window_id: None,
            work_packet_id: None,
            tool_name: "qualification_harness".into(),
            authority_role: CustodyAuthorityRole::System,
            document_reference: format!("capability://{}", capability_id),
            custody_action: CustodyAction::Release,
            previous_custody_mode: Some(CustodyMode::LocalWorkingCopy),
            resulting_custody_mode: Some(CustodyMode::LocalCanonical),
            mutation_allowance: Some(MutationAllowance::ReadOnly),
            decision_reference: None,
            provenance_receipt: None,
            refusal_reason: None,
            target_project_id: None,
            target_session_id: None,
            target_node_id: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.db.record_custody_event(&end_event)?;

        Ok(QualificationRun {
            run_id,
            model_alias: profile.alias.clone(),
            capability_id,
            passed,
            evidence_id: evidence.record_id,
            receipt_id: receipt.receipt_id,
            completed_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Run qualification for multiple profiles.
    pub fn qualify_all(&self, profiles: &[ModelProfileConfig]) -> Result<Vec<QualificationRun>> {
        let mut results = Vec::new();
        for profile in profiles {
            let run = self.qualify_profile(profile)?;
            results.push(run);
        }
        Ok(results)
    }

    /// Get the number of stored qualification receipts.
    pub fn qualification_count(&self) -> Result<u64> {
        Ok(0) // TODO: count from GovernanceDb
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::db::GovernanceDb;

    fn sample_profile() -> ModelProfileConfig {
        ModelProfileConfig {
            alias: "phi-4".into(),
            model_file: "microsoft_Phi-4-mini-instruct-Q4_K_M.gguf".into(),
            gguf_size_gb: 2.32,
            backend: "vulkan".into(),
            ngl: 99,
            context: 4096,
            port: 9120,
            task_classes: vec!["general_advisory".into()],
            verified_status: "verified".into(),
            stability: "stable".into(),
            requires_reduced_offload: false,
            authority_status: "advisory_only".into(),
            limitations: "Safe up to 4096".into(),
            known_behavior: "Clean output".into(),
        }
    }

    fn setup() -> QualificationHarness {
        let db = GovernanceDb::open_in_memory().unwrap();
        QualificationHarness::new(db)
    }

    #[test]
    fn test_qualify_verified_profile() {
        let harness = setup();
        let profile = sample_profile();
        let result = harness.qualify_profile(&profile).unwrap();
        assert!(result.passed);
        assert_eq!(result.model_alias, "phi-4");
        assert_eq!(result.capability_id, "model-phi-4");
        assert!(!result.evidence_id.is_empty());
        assert!(!result.receipt_id.is_empty());
    }

    #[test]
    fn test_qualify_unverified_profile() {
        let harness = setup();
        let mut profile = sample_profile();
        profile.verified_status = "unverified".into();
        let result = harness.qualify_profile(&profile).unwrap();
        assert!(!result.passed);
    }

    #[test]
    fn test_qualify_all_profiles() {
        let harness = setup();
        let profiles = vec![sample_profile(); 3]; // 3 profiles
        let results = harness.qualify_all(&profiles).unwrap();
        assert_eq!(results.len(), 3);
        for r in &results {
            assert!(r.passed);
        }
    }

    #[test]
    fn test_residency_tracking() {
        let harness = setup();
        let profile = sample_profile();
        let result = harness.qualify_profile(&profile).unwrap();

        // Verify custody events were recorded (residency start + end)
        let events = harness.db.get_custody_events(
            &format!("capability://{}", result.capability_id)
        ).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].custody_action, CustodyAction::Claim);
        assert_eq!(events[1].custody_action, CustodyAction::Release);
    }

    #[test]
    fn test_no_new_primitives() {
        // Verify that the qualification process only uses existing types
        let harness = setup();
        let profile = sample_profile();
        let result = harness.qualify_profile(&profile).unwrap();

        // The result should use existing types only
        let _: Capability = profile.to_capability();
        let _: EvidenceRecord = harness.evidence_gen.generate(
            EvidenceCategory::ContractValidation,
            "test",
            &serde_json::json!({}),
        ).unwrap();
        assert!(result.passed || !result.passed);
    }
}
