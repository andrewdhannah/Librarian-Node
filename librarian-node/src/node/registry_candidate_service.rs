use std::path::PathBuf;

use librarian_contracts::node::{NodeIdentity, RegistrationReceipt};
use librarian_contracts::registry::{
    CandidateEvidence, CandidateReviewReceipt, CandidateStatus, DiscoveryMethod, EvidenceType,
    NodeCandidate, ReviewDecision,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const DEFAULT_EVIDENCE_RETENTION_DAYS: u32 = 90;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupSummary {
    pub expired_candidates: usize,
    pub flagged_stale_review: usize,
    pub flagged_candidate_ids: Vec<String>,
    pub expired_candidate_ids: Vec<String>,
    pub evidence_before_count: usize,
    pub evidence_after_count: usize,
    pub evidence_purged: usize,
}

use super::capability_evidence::CapabilityEvidenceBridge;
use super::custody_service::CustodyService;
use super::fleet_service::FleetService;
use super::identity_service::NodeIdentityService;
use super::registration_service::RegistrationService;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    candidates: Vec<NodeCandidate>,
    evidence: Vec<CandidateEvidence>,
    review_receipts: Vec<CandidateReviewReceipt>,
}

pub struct RegistryCandidateService {
    candidates: Vec<NodeCandidate>,
    evidence: Vec<CandidateEvidence>,
    review_receipts: Vec<CandidateReviewReceipt>,
    persistence_path: PathBuf,
}

impl RegistryCandidateService {
    pub fn new(persistence_path: impl Into<PathBuf>) -> Self {
        let persistence_path = persistence_path.into();
        let (candidates, evidence, review_receipts) = if persistence_path.exists() {
            match std::fs::read_to_string(&persistence_path) {
                Ok(content) => match serde_json::from_str::<PersistedState>(&content) {
                    Ok(state) => (state.candidates, state.evidence, state.review_receipts),
                    Err(_) => (Vec::new(), Vec::new(), Vec::new()),
                },
                Err(_) => (Vec::new(), Vec::new(), Vec::new()),
            }
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };

        let mut svc = RegistryCandidateService {
            candidates,
            evidence,
            review_receipts,
            persistence_path,
        };
        svc.recover_stale();
        svc
    }

    fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState {
            candidates: self.candidates.clone(),
            evidence: self.evidence.clone(),
            review_receipts: self.review_receipts.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.persistence_path, json);
        }
    }

    fn next_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    fn find_candidate_mut(&mut self, candidate_id: &str) -> Option<&mut NodeCandidate> {
        self.candidates.iter_mut().find(|c| c.candidate_id == candidate_id)
    }

    /// Recover stale candidates on startup: candidates stuck in "under_review"
    /// with no updates for >24h revert to candidate state.
    pub fn recover_stale(&mut self) -> Vec<NodeCandidate> {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(24);
        let mut recovered = Vec::new();

        for candidate in &mut self.candidates {
            if candidate.status == CandidateStatus::UnderReview {
                if let Ok(last) = chrono::DateTime::parse_from_rfc3339(&candidate.last_updated_at) {
                    if last < cutoff {
                        candidate.status = CandidateStatus::Candidate;
                        candidate.last_updated_at = chrono::Utc::now().to_rfc3339();
                        recovered.push(candidate.clone());
                    }
                }
            }
        }

        if !recovered.is_empty() {
            self.persist();
        }
        recovered
    }

    /// Verify the persisted data file is loadable (uncorrupted).
    pub fn verify_file_integrity(&self) -> bool {
        if !self.persistence_path.exists() {
            return true;
        }
        match std::fs::read_to_string(&self.persistence_path) {
            Ok(content) => serde_json::from_str::<PersistedState>(&content).is_ok(),
            Err(_) => false,
        }
    }

    /// Regenerate data file from in-memory state if corrupted.
    pub fn regenerate_if_corrupted(&mut self) -> bool {
        if !self.verify_file_integrity() {
            self.persist();
            true
        } else {
            false
        }
    }

    /// Evidence count.
    pub fn evidence_count(&self) -> usize {
        self.evidence.len()
    }

    /// Candidate count.
    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    /// Perform stale candidate cleanup.
    /// - Candidates in "discovered" or "candidate" state older than 7 days → auto-expire
    /// - Candidates in "under_review" state older than 48 hours → flagged for notification
    pub fn cleanup_stale(&mut self) -> CleanupSummary {
        let now = chrono::Utc::now();
        let seven_days_ago = now - chrono::Duration::days(7);
        let forty_eight_hours_ago = now - chrono::Duration::hours(48);

        let mut expired_candidates = Vec::new();
        let mut flagged_review = Vec::new();

        self.candidates.retain(|candidate| {
            let last = match chrono::DateTime::parse_from_rfc3339(&candidate.last_updated_at) {
                Ok(dt) => dt,
                Err(_) => return true,
            };

            match candidate.status {
                CandidateStatus::Discovered | CandidateStatus::Candidate => {
                    if last < seven_days_ago {
                        expired_candidates.push(candidate.clone());
                        return false;
                    }
                    true
                }
                CandidateStatus::UnderReview => {
                    if last < forty_eight_hours_ago {
                        flagged_review.push(candidate.clone());
                    }
                    true
                }
                _ => true,
            }
        });

        let expired_evidence = self.cleanup_evidence_by_retention();
        let evidence_before = self.evidence.len() + expired_evidence.len();
        let evidence_after = self.evidence.len();

        let summary = CleanupSummary {
            expired_candidates: expired_candidates.len(),
            flagged_stale_review: flagged_review.len(),
            flagged_candidate_ids: flagged_review.iter().map(|c| c.candidate_id.clone()).collect(),
            expired_candidate_ids: expired_candidates.iter().map(|c| c.candidate_id.clone()).collect(),
            evidence_before_count: evidence_before,
            evidence_after_count: evidence_after,
            evidence_purged: expired_evidence.len(),
        };

        if expired_candidates.len() > 0 || expired_evidence.len() > 0 {
            self.persist();
        }

        summary
    }

    /// Purge evidence older than its retention_days.
    pub fn cleanup_evidence_by_retention(&mut self) -> Vec<CandidateEvidence> {
        let now = chrono::Utc::now();
        let mut purged = Vec::new();

        self.evidence.retain(|ev| {
            let collected = match chrono::DateTime::parse_from_rfc3339(&ev.collected_at) {
                Ok(dt) => dt,
                Err(_) => return true,
            };
            let retention = chrono::Duration::days(ev.retention_days as i64);
            if collected + retention < now {
                purged.push(ev.clone());
                return false;
            }
            true
        });

        purged
    }

    /// Get the persistence path for health checks.
    pub fn persistence_path(&self) -> &std::path::Path {
        &self.persistence_path
    }

    pub fn discover(
        &mut self,
        node_id: &str,
        display_name: &str,
        discovery_method: DiscoveryMethod,
    ) -> NodeCandidate {
        if let Some(existing) = self.candidates.iter().find(|c| c.node_id == node_id) {
            return existing.clone();
        }

        let now = chrono::Utc::now().to_rfc3339();
        let candidate = NodeCandidate {
            candidate_id: self.next_id(),
            node_id: node_id.to_string(),
            display_name: display_name.to_string(),
            status: CandidateStatus::Discovered,
            first_seen_at: now.clone(),
            last_updated_at: now,
            discovery_method,
        };
        self.candidates.push(candidate.clone());
        self.persist();
        candidate
    }

    pub fn collect_evidence(
        &mut self,
        candidate_id: &str,
        identity_service: &NodeIdentityService,
        capability_bridge: &CapabilityEvidenceBridge,
        custody_service: &CustodyService,
        fleet_service: &FleetService,
    ) -> Vec<CandidateEvidence> {
        let has_candidate = self.candidates.iter().any(|c| c.candidate_id == candidate_id);
        if !has_candidate {
            return Vec::new();
        }

        let candidate_node_id = self.candidates.iter()
            .find(|c| c.candidate_id == candidate_id)
            .map(|c| c.node_id.clone())
            .unwrap_or_default();

        let now = chrono::Utc::now().to_rfc3339();
        let mut collected = Vec::new();

        let identity = identity_service.get_identity();
        collected.push(CandidateEvidence {
            evidence_id: self.next_id(),
            candidate_id: candidate_id.to_string(),
            evidence_type: EvidenceType::Identity,
            payload: serde_json::to_value(identity).unwrap_or_default(),
            collected_at: now.clone(),
            retention_days: DEFAULT_EVIDENCE_RETENTION_DAYS,
        });

        collected.push(CandidateEvidence {
            evidence_id: self.next_id(),
            candidate_id: candidate_id.to_string(),
            evidence_type: EvidenceType::Capability,
            payload: serde_json::to_value(
                capability_bridge.get_verification_state(&candidate_node_id),
            )
            .unwrap_or_default(),
            collected_at: now.clone(),
            retention_days: DEFAULT_EVIDENCE_RETENTION_DAYS,
        });

        collected.push(CandidateEvidence {
            evidence_id: self.next_id(),
            candidate_id: candidate_id.to_string(),
            evidence_type: EvidenceType::Custody,
            payload: serde_json::to_value(custody_service.get_chain()).unwrap_or_default(),
            collected_at: now.clone(),
            retention_days: DEFAULT_EVIDENCE_RETENTION_DAYS,
        });

        let fleet_entry = fleet_service.get_node(&candidate_node_id);
        collected.push(CandidateEvidence {
            evidence_id: self.next_id(),
            candidate_id: candidate_id.to_string(),
            evidence_type: EvidenceType::Health,
            payload: serde_json::to_value(&fleet_entry).unwrap_or_default(),
            collected_at: now.clone(),
            retention_days: DEFAULT_EVIDENCE_RETENTION_DAYS,
        });

        for ev in &collected {
            self.evidence.push(ev.clone());
        }

        if let Some(c) = self.find_candidate_mut(candidate_id) {
            c.status = CandidateStatus::EvidenceCollection;
            c.last_updated_at = chrono::Utc::now().to_rfc3339();
        }

        self.persist();
        collected
    }

    pub fn collect_evidence_simple(
        &mut self,
        candidate_id: &str,
        identity: &NodeIdentity,
        capability_state: &librarian_contracts::capability_evidence::CapabilityVerificationState,
        custody_chain: &Option<librarian_contracts::custody::CustodyChain>,
        fleet_entry: &Option<librarian_contracts::fleet::NodeInventoryEntry>,
    ) -> Vec<CandidateEvidence> {
        let has_candidate = self.candidates.iter().any(|c| c.candidate_id == candidate_id);
        if !has_candidate {
            return Vec::new();
        }

        let now = chrono::Utc::now().to_rfc3339();
        let mut collected = Vec::new();

        collected.push(CandidateEvidence {
            evidence_id: self.next_id(),
            candidate_id: candidate_id.to_string(),
            evidence_type: EvidenceType::Identity,
            payload: serde_json::to_value(identity).unwrap_or_default(),
            collected_at: now.clone(),
            retention_days: DEFAULT_EVIDENCE_RETENTION_DAYS,
        });

        collected.push(CandidateEvidence {
            evidence_id: self.next_id(),
            candidate_id: candidate_id.to_string(),
            evidence_type: EvidenceType::Capability,
            payload: serde_json::to_value(capability_state).unwrap_or_default(),
            collected_at: now.clone(),
            retention_days: DEFAULT_EVIDENCE_RETENTION_DAYS,
        });

        collected.push(CandidateEvidence {
            evidence_id: self.next_id(),
            candidate_id: candidate_id.to_string(),
            evidence_type: EvidenceType::Custody,
            payload: serde_json::to_value(custody_chain).unwrap_or_default(),
            collected_at: now.clone(),
            retention_days: DEFAULT_EVIDENCE_RETENTION_DAYS,
        });

        collected.push(CandidateEvidence {
            evidence_id: self.next_id(),
            candidate_id: candidate_id.to_string(),
            evidence_type: EvidenceType::Health,
            payload: serde_json::to_value(fleet_entry).unwrap_or_default(),
            collected_at: now.clone(),
            retention_days: DEFAULT_EVIDENCE_RETENTION_DAYS,
        });

        for ev in &collected {
            self.evidence.push(ev.clone());
        }

        if let Some(c) = self.find_candidate_mut(candidate_id) {
            c.status = CandidateStatus::EvidenceCollection;
            c.last_updated_at = chrono::Utc::now().to_rfc3339();
        }

        self.persist();
        collected
    }

    pub fn submit_for_review(&mut self, candidate_id: &str) -> Option<NodeCandidate> {
        let candidate = self.find_candidate_mut(candidate_id)?;
        candidate.status = CandidateStatus::UnderReview;
        candidate.last_updated_at = chrono::Utc::now().to_rfc3339();
        let result = candidate.clone();
        self.persist();
        Some(result)
    }

    pub fn review(
        &mut self,
        candidate_id: &str,
        decision: ReviewDecision,
        reviewer: &str,
        reason: &str,
        registration_service: &mut RegistrationService,
    ) -> Option<CandidateReviewReceipt> {
        let candidate = self.find_candidate_mut(candidate_id)?;
        let previous_status = candidate.status.to_string();
        let candidate_node_id = candidate.node_id.clone();

        let new_status = match decision {
            ReviewDecision::Approve => CandidateStatus::Admitted,
            ReviewDecision::Reject => CandidateStatus::Rejected,
            ReviewDecision::RequestInfo => CandidateStatus::EvidenceCollection,
        };

        candidate.status = new_status;
        candidate.last_updated_at = chrono::Utc::now().to_rfc3339();

        let now = chrono::Utc::now().to_rfc3339();
        let new_status_str = candidate.status.to_string();

        let receipt = CandidateReviewReceipt {
            receipt_id: self.next_id(),
            candidate_id: candidate_id.to_string(),
            decision: decision.clone(),
            reviewer: reviewer.to_string(),
            reason: reason.to_string(),
            decided_at: now.clone(),
            previous_status: previous_status.clone(),
            new_status: new_status_str,
        };
        self.review_receipts.push(receipt.clone());

        if matches!(decision, ReviewDecision::Approve) {
            let reg_receipt = RegistrationReceipt {
                registration_id: self.next_id(),
                node_id: candidate_node_id,
                status: "admitted_via_candidate".to_string(),
                registered_at: now.clone(),
                previous_state: Some(previous_status),
            };
            registration_service.confirm_registration(&reg_receipt);
        }

        self.persist();
        Some(receipt)
    }

    pub fn get_candidate(&self, candidate_id: &str) -> Option<NodeCandidate> {
        self.candidates.iter().find(|c| c.candidate_id == candidate_id).cloned()
    }

    pub fn get_candidates(&self, status_filter: Option<&str>) -> Vec<NodeCandidate> {
        match status_filter {
            Some(filter) => {
                let target: CandidateStatus = filter.into();
                self.candidates.iter().filter(|c| c.status == target).cloned().collect()
            }
            None => self.candidates.clone(),
        }
    }

    pub fn get_evidence(&self, candidate_id: &str) -> Vec<CandidateEvidence> {
        self.evidence
            .iter()
            .filter(|e| e.candidate_id == candidate_id)
            .cloned()
            .collect()
    }

    pub fn expire_stale(&mut self, days: u32) -> Vec<NodeCandidate> {
        let cutoff = chrono::Utc::now()
            - chrono::Duration::days(days as i64);
        let mut expired = Vec::new();

        self.candidates.retain(|candidate| {
            match candidate.status {
                CandidateStatus::Discovered
                | CandidateStatus::Candidate
                | CandidateStatus::EvidenceCollection => {
                    if let Ok(last) = chrono::DateTime::parse_from_rfc3339(&candidate.last_updated_at) {
                        if last < cutoff {
                            expired.push(candidate.clone());
                            return false;
                        }
                    }
                    true
                }
                _ => true,
            }
        });

        if !expired.is_empty() {
            self.persist();
        }
        expired
    }

    pub fn get_all_candidates(&self) -> &[NodeCandidate] {
        &self.candidates
    }

    pub fn get_all_evidence(&self) -> &[CandidateEvidence] {
        &self.evidence
    }

    pub fn get_all_evidence_mut(&mut self) -> &mut Vec<CandidateEvidence> {
        &mut self.evidence
    }

    pub fn get_all_receipts(&self) -> &[CandidateReviewReceipt] {
        &self.review_receipts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_contracts::capability_evidence::CapabilityVerificationState;
    use librarian_contracts::fleet::NodeInventoryEntry;
    use librarian_contracts::node::NodeIdentity;
    use tempfile::tempdir;

    fn test_identity() -> NodeIdentity {
        NodeIdentity {
            node_id: "test-node-uuid".to_string(),
            display_name: "test-host".to_string(),
            platform: "test".to_string(),
            runtime_version: "0.1.0".to_string(),
            contract_version: "1".to_string(),
            first_seen_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn setup_registration(dir: &tempfile::TempDir) -> RegistrationService {
        let path = dir.path().join("node-registration.json");
        RegistrationService::new(&path)
    }

    fn make_candidate(svc: &mut RegistryCandidateService, node_id: &str, name: &str) -> NodeCandidate {
        svc.discover(node_id, name, DiscoveryMethod::ApiDiscovery)
    }

    // --- Test: Discover creates candidate with Discovered status ---
    #[test]
    fn test_discover_creates_candidate() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry-candidates.json");
        let mut svc = RegistryCandidateService::new(&path);

        let candidate = svc.discover("node-1", "Node One", DiscoveryMethod::ApiDiscovery);
        assert_eq!(candidate.status, CandidateStatus::Discovered);
        assert_eq!(candidate.node_id, "node-1");
        assert_eq!(candidate.display_name, "Node One");
    }

    #[test]
    fn test_discover_returns_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry-candidates.json");
        let mut svc = RegistryCandidateService::new(&path);

        let c1 = svc.discover("node-1", "Node One", DiscoveryMethod::ApiDiscovery);
        let c2 = svc.discover("node-1", "Node One Updated", DiscoveryMethod::Manual);
        // Should return existing without updating
        assert_eq!(c1.candidate_id, c2.candidate_id);
        assert_eq!(c1.display_name, "Node One");
        assert_eq!(c1.discovery_method, DiscoveryMethod::ApiDiscovery);
    }

    // --- Test: Collect evidence gathers all 4 types ---
    #[test]
    fn test_collect_evidence_simple_gathers_all_types() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry-candidates.json");
        let mut svc = RegistryCandidateService::new(&path);

        let candidate = make_candidate(&mut svc, "node-1", "Node One");
        let identity = test_identity();
        let cap_state = CapabilityVerificationState {
            node_id: "node-1".to_string(),
            capabilities: Vec::new(),
        };
        let custody_chain: Option<librarian_contracts::custody::CustodyChain> = None;
        let fleet_entry: Option<NodeInventoryEntry> = None;

        let evidence = svc.collect_evidence_simple(
            &candidate.candidate_id,
            &identity,
            &cap_state,
            &custody_chain,
            &fleet_entry,
        );

        assert_eq!(evidence.len(), 4);
        let types: Vec<String> = evidence.iter().map(|e| e.evidence_type.to_string()).collect();
        assert!(types.contains(&"identity".to_string()));
        assert!(types.contains(&"capability".to_string()));
        assert!(types.contains(&"custody".to_string()));
        assert!(types.contains(&"health".to_string()));
    }

    #[test]
    fn test_collect_evidence_transitions_status() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry-candidates.json");
        let mut svc = RegistryCandidateService::new(&path);

        let candidate = make_candidate(&mut svc, "node-1", "Node One");
        let identity = test_identity();
        let cap_state = CapabilityVerificationState {
            node_id: "node-1".to_string(),
            capabilities: Vec::new(),
        };
        let custody_chain = None;
        let fleet_entry = None;

        svc.collect_evidence_simple(&candidate.candidate_id, &identity, &cap_state, &custody_chain, &fleet_entry);

        let updated = svc.get_candidate(&candidate.candidate_id).unwrap();
        assert_eq!(updated.status, CandidateStatus::EvidenceCollection);
    }

    // --- Test: Submit transitions to UnderReview ---
    #[test]
    fn test_submit_for_review_transitions() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry-candidates.json");
        let mut svc = RegistryCandidateService::new(&path);

        let candidate = make_candidate(&mut svc, "node-1", "Node One");

        // Set to evidence collection first
        let identity = test_identity();
        let cap_state = CapabilityVerificationState {
            node_id: "node-1".to_string(),
            capabilities: Vec::new(),
        };
        let custody_chain = None;
        let fleet_entry = None;
        svc.collect_evidence_simple(&candidate.candidate_id, &identity, &cap_state, &custody_chain, &fleet_entry);

        let submitted = svc.submit_for_review(&candidate.candidate_id).unwrap();
        assert_eq!(submitted.status, CandidateStatus::UnderReview);

        let stored = svc.get_candidate(&candidate.candidate_id).unwrap();
        assert_eq!(stored.status, CandidateStatus::UnderReview);
    }

    // --- Test: Review approve transitions to Admitted, triggers registration ---
    #[test]
    fn test_review_approve_transitions_to_admitted() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry-candidates.json");
        let mut svc = RegistryCandidateService::new(&path);
        let mut reg = setup_registration(&dir);

        // Pre-register identity first since confirm_registration expects a valid record
        let identity = test_identity();
        reg.submit_registration(&identity, None);

        let candidate = make_candidate(&mut svc, "node-1", "Node One");

        let receipt = svc.review(
            &candidate.candidate_id,
            ReviewDecision::Approve,
            "owner",
            "Approved for admission",
            &mut reg,
        ).unwrap();

        assert_eq!(receipt.decision, ReviewDecision::Approve);
        assert_eq!(receipt.previous_status, "discovered");
        assert_eq!(receipt.new_status, "admitted");

        let stored = svc.get_candidate(&candidate.candidate_id).unwrap();
        assert_eq!(stored.status, CandidateStatus::Admitted);

        // Registration should have been triggered
        assert_eq!(reg.get_record().registration_status, "admitted_via_candidate");
    }

    // --- Test: Review reject transitions to Rejected ---
    #[test]
    fn test_review_reject_transitions_to_rejected() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry-candidates.json");
        let mut svc = RegistryCandidateService::new(&path);
        let mut reg = setup_registration(&dir);

        let candidate = make_candidate(&mut svc, "node-1", "Node One");

        let receipt = svc.review(
            &candidate.candidate_id,
            ReviewDecision::Reject,
            "owner",
            "Not eligible",
            &mut reg,
        ).unwrap();

        assert_eq!(receipt.decision, ReviewDecision::Reject);
        assert_eq!(receipt.new_status, "rejected");

        let stored = svc.get_candidate(&candidate.candidate_id).unwrap();
        assert_eq!(stored.status, CandidateStatus::Rejected);

        // Registration should NOT have been triggered
        assert_eq!(reg.get_record().registration_status, "unregistered");
    }

    // --- Test: Review request_info transitions back to EvidenceCollection ---
    #[test]
    fn test_review_request_info_transitions_back() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry-candidates.json");
        let mut svc = RegistryCandidateService::new(&path);
        let mut reg = setup_registration(&dir);

        let candidate = make_candidate(&mut svc, "node-1", "Node One");

        // First transition to evidence collection
        let identity = test_identity();
        let cap_state = CapabilityVerificationState {
            node_id: "node-1".to_string(),
            capabilities: Vec::new(),
        };
        let custody_chain = None;
        let fleet_entry = None;
        svc.collect_evidence_simple(&candidate.candidate_id, &identity, &cap_state, &custody_chain, &fleet_entry);

        // Submit for review
        svc.submit_for_review(&candidate.candidate_id);

        // Request more info
        let receipt = svc.review(
            &candidate.candidate_id,
            ReviewDecision::RequestInfo,
            "owner",
            "Need more evidence",
            &mut reg,
        ).unwrap();

        assert_eq!(receipt.decision, ReviewDecision::RequestInfo);
        assert_eq!(receipt.new_status, "evidence_collection");

        let stored = svc.get_candidate(&candidate.candidate_id).unwrap();
        assert_eq!(stored.status, CandidateStatus::EvidenceCollection);
    }

    // --- Test: Expire marks old candidates ---
    #[test]
    fn test_expire_stale_removes_old_candidates() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry-candidates.json");
        let mut svc = RegistryCandidateService::new(&path);

        let candidate = make_candidate(&mut svc, "node-1", "Node One");

        // Manually set last_updated_at to 10 days ago (inject through persistence)
        {
            let c = svc.find_candidate_mut(&candidate.candidate_id).unwrap();
            let old_time = (chrono::Utc::now() - chrono::Duration::days(10)).to_rfc3339();
            c.last_updated_at = old_time;
            svc.persist();
        }

        let expired = svc.expire_stale(5);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].candidate_id, candidate.candidate_id);
        assert!(svc.get_candidate(&candidate.candidate_id).is_none());
    }

    // --- Test: Candidates persist across restarts ---
    #[test]
    fn test_candidates_persist_across_restarts() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry-candidates.json");

        let candidate_id;
        {
            let mut svc1 = RegistryCandidateService::new(&path);
            let candidate = make_candidate(&mut svc1, "node-1", "Node One");
            candidate_id = candidate.candidate_id.clone();
        }

        {
            let svc2 = RegistryCandidateService::new(&path);
            let loaded = svc2.get_candidate(&candidate_id).unwrap();
            assert_eq!(loaded.node_id, "node-1");
            assert_eq!(loaded.display_name, "Node One");
            assert_eq!(loaded.status, CandidateStatus::Discovered);
        }
    }

    // --- Test: get_candidates with status filter ---
    #[test]
    fn test_get_candidates_with_status_filter() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry-candidates.json");
        let mut svc = RegistryCandidateService::new(&path);

        make_candidate(&mut svc, "node-1", "Node One");
        make_candidate(&mut svc, "node-2", "Node Two");

        let discovered = svc.get_candidates(Some("discovered"));
        assert_eq!(discovered.len(), 2);

        let admitted = svc.get_candidates(Some("admitted"));
        assert_eq!(admitted.len(), 0);
    }

    // --- Test: Evidence persists across restarts ---
    #[test]
    fn test_evidence_persists_across_restarts() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry-candidates.json");
        let candidate_id;

        {
            let mut svc1 = RegistryCandidateService::new(&path);
            let candidate = make_candidate(&mut svc1, "node-1", "Node One");
            candidate_id = candidate.candidate_id.clone();

            let identity = test_identity();
            let cap_state = CapabilityVerificationState {
            node_id: "node-1".to_string(),
            capabilities: Vec::new(),
        };
            let custody_chain = None;
            let fleet_entry = None;
            svc1.collect_evidence_simple(
                &candidate_id, &identity, &cap_state, &custody_chain, &fleet_entry,
            );
        }

        {
            let svc2 = RegistryCandidateService::new(&path);
            let evidence = svc2.get_evidence(&candidate_id);
            assert_eq!(evidence.len(), 4);

            let loaded = svc2.get_candidate(&candidate_id).unwrap();
            assert_eq!(loaded.status, CandidateStatus::EvidenceCollection);
        }
    }

    // --- Test: Expire does not affect approved/admitted candidates ---
    #[test]
    fn test_expire_does_not_affect_finalized() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry-candidates.json");
        let mut svc = RegistryCandidateService::new(&path);
        let mut reg = setup_registration(&dir);

        reg.submit_registration(&test_identity(), None);

        let candidate = make_candidate(&mut svc, "node-1", "Node One");

        svc.review(&candidate.candidate_id, ReviewDecision::Reject, "owner", "Not eligible", &mut reg);

        // Manually set old timestamp
        {
            let c = svc.find_candidate_mut(&candidate.candidate_id).unwrap();
            c.last_updated_at = (chrono::Utc::now() - chrono::Duration::days(10)).to_rfc3339();
            svc.persist();
        }

        let expired = svc.expire_stale(5);
        // Should NOT expire rejected candidates
        assert!(expired.is_empty());
        assert!(svc.get_candidate(&candidate.candidate_id).is_some());
    }
}
