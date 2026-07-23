use std::path::PathBuf;

use librarian_contracts::allocation::AllocationRecommendation;
use librarian_contracts::owner_allocation::{
    AllocationActionReceipt, AllocationDecision, AllocationDecisionReceipt,
    AllocationRecommendationSummary, AllocationReviewResult, PendingAllocationQueue,
};
use uuid::Uuid;

use super::AllocationService;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PersistedState {
    decisions: Vec<AllocationDecision>,
    decision_receipts: Vec<AllocationDecisionReceipt>,
    action_receipts: Vec<AllocationActionReceipt>,
}

pub struct OwnerAllocationService {
    decisions: Vec<AllocationDecision>,
    decision_receipts: Vec<AllocationDecisionReceipt>,
    action_receipts: Vec<AllocationActionReceipt>,
    persistence_path: PathBuf,
}

impl OwnerAllocationService {
    pub fn new(persistence_path: impl Into<PathBuf>) -> Self {
        let persistence_path = persistence_path.into();
        let (decisions, decision_receipts, action_receipts) = if persistence_path.exists() {
            match std::fs::read_to_string(&persistence_path) {
                Ok(content) => match serde_json::from_str::<PersistedState>(&content) {
                    Ok(state) => (state.decisions, state.decision_receipts, state.action_receipts),
                    Err(_) => (Vec::new(), Vec::new(), Vec::new()),
                },
                Err(_) => (Vec::new(), Vec::new(), Vec::new()),
            }
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };

        OwnerAllocationService {
            decisions,
            decision_receipts,
            action_receipts,
            persistence_path,
        }
    }

    fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState {
            decisions: self.decisions.clone(),
            decision_receipts: self.decision_receipts.clone(),
            action_receipts: self.action_receipts.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.persistence_path, json);
        }
    }

    fn build_summary(rec: &AllocationRecommendation) -> AllocationRecommendationSummary {
        AllocationRecommendationSummary {
            recommendation_id: rec.recommendation_id.clone(),
            workload_description: rec.workload_id.clone(),
            recommended_node_id: rec.node_id.clone(),
            recommended_node_name: format!("node-{}", &rec.node_id.chars().take(8).collect::<String>()),
            score: rec.score.score,
            evidence_verified: rec.score.evidence_verified,
            key_reasoning: rec.reasoning.clone(),
            status: rec.status.clone(),
            generated_at: rec.generated_at.clone(),
        }
    }

    pub fn get_pending_recommendations(
        &self,
        allocation_service: &AllocationService,
    ) -> PendingAllocationQueue {
        let proposed = allocation_service.get_recommendations(Some("proposed"));
        let items: Vec<AllocationRecommendationSummary> =
            proposed.iter().map(Self::build_summary).collect();
        let total_pending = items.len() as u32;

        PendingAllocationQueue {
            total_pending,
            items,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn review_recommendations(
        &self,
        allocation_service: &AllocationService,
        filter: Option<&str>,
    ) -> AllocationReviewResult {
        let all = allocation_service.get_recommendations(filter);
        let pending_count = all.iter().filter(|r| r.status == "proposed").count() as u32;
        let total_count = all.len() as u32;
        let recommendations: Vec<AllocationRecommendationSummary> =
            all.iter().map(Self::build_summary).collect();

        AllocationReviewResult {
            result_id: Uuid::new_v4().to_string(),
            request_id: String::new(),
            pending_count,
            total_count,
            recommendations,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn get_recommendation_detail(
        &self,
        allocation_service: &AllocationService,
        recommendation_id: &str,
    ) -> Option<AllocationRecommendationSummary> {
        let all = allocation_service.get_recommendations(None);
        all.iter()
            .find(|r| r.recommendation_id == recommendation_id)
            .map(Self::build_summary)
    }

    pub fn submit_decision(
        &mut self,
        allocation_service: &mut AllocationService,
        decision: AllocationDecision,
    ) -> AllocationDecisionReceipt {
        let rec_id = &decision.recommendation_id;

        let workload_description;
        let selected_node_id: Option<String>;

        match decision.decision.as_str() {
            "approved" => {
                let receipt = allocation_service
                    .accept_recommendation(rec_id, Some(decision.session_id.clone()));
                if let Some(ref r) = receipt {
                    workload_description = r.workload_id.clone();
                    selected_node_id = Some(r.node_id.clone());
                } else {
                    workload_description = String::new();
                    selected_node_id = None;
                }
            }
            "rejected" => {
                let receipt =
                    allocation_service.reject_recommendation(rec_id, decision.reason.clone());
                if let Some(ref r) = receipt {
                    workload_description = r.workload_id.clone();
                    selected_node_id = Some(r.node_id.clone());
                } else {
                    workload_description = String::new();
                    selected_node_id = None;
                }
            }
            _ => {
                workload_description = String::new();
                selected_node_id = decision.alternative_node_id.clone();
            }
        }

        let decision_receipt = AllocationDecisionReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            decision_id: decision.decision_id.clone(),
            recommendation_id: decision.recommendation_id.clone(),
            decision: decision.decision.clone(),
            workload_description,
            selected_node_id,
            decided_at: decision.decided_at.clone(),
            session_id: decision.session_id.clone(),
            custody_envelope_id: None,
        };

        self.decisions.push(decision);
        self.decision_receipts.push(decision_receipt.clone());
        self.persist();

        decision_receipt
    }

    pub fn log_action(
        &mut self,
        decision_id: &str,
        action: &str,
        session_id: Option<String>,
        node_id: &str,
    ) -> AllocationActionReceipt {
        let rec = AllocationActionReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            decision_id: decision_id.to_string(),
            recommendation_id: String::new(),
            action: action.to_string(),
            session_id,
            node_id: node_id.to_string(),
            acted_at: chrono::Utc::now().to_rfc3339(),
        };

        self.action_receipts.push(rec.clone());
        self.persist();

        rec
    }

    pub fn get_decision_history(&self) -> Vec<AllocationDecisionReceipt> {
        self.decision_receipts.clone()
    }

    pub fn get_action_receipts(&self) -> Vec<AllocationActionReceipt> {
        self.action_receipts.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::FleetService;
    use librarian_contracts::allocation::AllocationRequest;
    use tempfile::tempdir;

    fn test_allocation_service(dir: &tempfile::TempDir) -> AllocationService {
        let path = dir.path().join("allocation.json");
        AllocationService::new(path)
    }

    fn test_owner_service(dir: &tempfile::TempDir) -> OwnerAllocationService {
        let path = dir.path().join("owner_allocation.json");
        OwnerAllocationService::new(path)
    }

    fn test_fleet_service(dir: &tempfile::TempDir) -> FleetService {
        let path = dir.path().join("fleet.json");
        let mut fleet = FleetService::new(path);
        fleet.add_or_update_node(librarian_contracts::fleet::NodeInventoryEntry {
            node_id: "node-001".to_string(),
            display_name: "test-node".to_string(),
            status: "online".to_string(),
            last_seen_at: Some(chrono::Utc::now().to_rfc3339()),
            runtime_version: "0.1.0".to_string(),
            platform: "test".to_string(),
            capability_count: 5,
            verified_capability_count: 5,
            session_count: 2,
            custody_envelope_count: 1,
            registered: true,
            bootstrap_completed: true,
            last_health_status: Some("healthy".to_string()),
        });
        fleet
    }

    fn make_request() -> AllocationRequest {
        AllocationRequest {
            request_id: "wl-001".to_string(),
            workload_description: "Test workload".to_string(),
            requirements: vec![librarian_contracts::allocation::CapabilityRequirement {
                requirement_id: "req-1".to_string(),
                capability_type: "inference".to_string(),
                required: true,
                constraints: None,
            }],
            preferred_nodes: None,
            requested_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn seed_proposed_recommendation(
        alloc: &mut AllocationService,
        fleet: &FleetService,
    ) -> AllocationRecommendation {
        alloc.generate_recommendation(make_request(), fleet)
    }

    #[test]
    fn test_pending_queue_returns_only_proposed() {
        let dir = tempdir().unwrap();
        let fleet = test_fleet_service(&dir);
        let mut alloc = test_allocation_service(&dir);
        let owner = test_owner_service(&dir);

        let rec1 = seed_proposed_recommendation(&mut alloc, &fleet);
        let rec2 = seed_proposed_recommendation(&mut alloc, &fleet);
        alloc.accept_recommendation(&rec1.recommendation_id, None);

        let queue = owner.get_pending_recommendations(&alloc);
        assert_eq!(queue.total_pending, 1);
        assert_eq!(queue.items.len(), 1);
        assert_eq!(queue.items[0].recommendation_id, rec2.recommendation_id);
        assert_eq!(queue.items[0].status, "proposed");
    }

    #[test]
    fn test_review_returns_recommendations_with_summaries() {
        let dir = tempdir().unwrap();
        let fleet = test_fleet_service(&dir);
        let mut alloc = test_allocation_service(&dir);
        let owner = test_owner_service(&dir);

        let _rec = seed_proposed_recommendation(&mut alloc, &fleet);

        let review = owner.review_recommendations(&alloc, None);
        assert!(review.total_count > 0);
        assert!(review.pending_count > 0);
        assert!(!review.recommendations.is_empty());
        assert!(review.recommendations[0].score > 0.0);
        assert!(!review.recommendations[0].key_reasoning.is_empty());
    }

    #[test]
    fn test_approve_decision_updates_status_and_generates_receipt() {
        let dir = tempdir().unwrap();
        let fleet = test_fleet_service(&dir);
        let mut alloc = test_allocation_service(&dir);
        let mut owner = test_owner_service(&dir);

        let rec = seed_proposed_recommendation(&mut alloc, &fleet);

        let decision = AllocationDecision {
            decision_id: "dec-001".to_string(),
            recommendation_id: rec.recommendation_id.clone(),
            session_id: "session-001".to_string(),
            decision: "approved".to_string(),
            alternative_node_id: None,
            reason: None,
            decided_at: chrono::Utc::now().to_rfc3339(),
        };

        let receipt = owner.submit_decision(&mut alloc, decision);
        assert_eq!(receipt.decision, "approved");
        assert_eq!(receipt.recommendation_id, rec.recommendation_id);
        assert_eq!(receipt.session_id, "session-001");
        assert!(receipt.selected_node_id.is_some());

        let updated = alloc.get_recommendations(Some("accepted"));
        assert_eq!(updated.len(), 1);
    }

    #[test]
    fn test_reject_decision_updates_status_and_generates_receipt() {
        let dir = tempdir().unwrap();
        let fleet = test_fleet_service(&dir);
        let mut alloc = test_allocation_service(&dir);
        let mut owner = test_owner_service(&dir);

        let rec = seed_proposed_recommendation(&mut alloc, &fleet);

        let decision = AllocationDecision {
            decision_id: "dec-002".to_string(),
            recommendation_id: rec.recommendation_id.clone(),
            session_id: "session-002".to_string(),
            decision: "rejected".to_string(),
            alternative_node_id: None,
            reason: Some("Insufficient resources".to_string()),
            decided_at: chrono::Utc::now().to_rfc3339(),
        };

        let receipt = owner.submit_decision(&mut alloc, decision);
        assert_eq!(receipt.decision, "rejected");
        assert_eq!(receipt.recommendation_id, rec.recommendation_id);

        let updated = alloc.get_recommendations(Some("rejected"));
        assert_eq!(updated.len(), 1);
    }

    #[test]
    fn test_decision_history_is_queryable() {
        let dir = tempdir().unwrap();
        let fleet = test_fleet_service(&dir);
        let mut alloc = test_allocation_service(&dir);
        let mut owner = test_owner_service(&dir);

        let rec1 = seed_proposed_recommendation(&mut alloc, &fleet);
        let rec2 = seed_proposed_recommendation(&mut alloc, &fleet);

        owner.submit_decision(
            &mut alloc,
            AllocationDecision {
                decision_id: "dec-001".to_string(),
                recommendation_id: rec1.recommendation_id.clone(),
                session_id: "s1".to_string(),
                decision: "approved".to_string(),
                alternative_node_id: None,
                reason: None,
                decided_at: chrono::Utc::now().to_rfc3339(),
            },
        );

        owner.submit_decision(
            &mut alloc,
            AllocationDecision {
                decision_id: "dec-002".to_string(),
                recommendation_id: rec2.recommendation_id.clone(),
                session_id: "s2".to_string(),
                decision: "rejected".to_string(),
                alternative_node_id: None,
                reason: Some("Nope".to_string()),
                decided_at: chrono::Utc::now().to_rfc3339(),
            },
        );

        let history = owner.get_decision_history();
        assert_eq!(history.len(), 2);

        let approved = history.iter().find(|h| h.decision == "approved").unwrap();
        assert_eq!(approved.decision_id, "dec-001");
        assert_eq!(approved.session_id, "s1");

        let rejected = history.iter().find(|h| h.decision == "rejected").unwrap();
        assert_eq!(rejected.decision_id, "dec-002");
    }

    #[test]
    fn test_get_recommendation_detail() {
        let dir = tempdir().unwrap();
        let fleet = test_fleet_service(&dir);
        let mut alloc = test_allocation_service(&dir);
        let owner = test_owner_service(&dir);

        let rec = seed_proposed_recommendation(&mut alloc, &fleet);

        let detail = owner.get_recommendation_detail(&alloc, &rec.recommendation_id);
        assert!(detail.is_some());
        assert_eq!(detail.unwrap().recommendation_id, rec.recommendation_id);
    }

    #[test]
    fn test_log_action_records_action_receipt() {
        let dir = tempdir().unwrap();
        let mut owner = test_owner_service(&dir);

        let receipt = owner.log_action("dec-001", "session_created", Some("session-001".to_string()), "node-001");
        assert_eq!(receipt.action, "session_created");
        assert_eq!(receipt.decision_id, "dec-001");
        assert_eq!(receipt.node_id, "node-001");

        let actions = owner.get_action_receipts();
        assert_eq!(actions.len(), 1);
    }

    #[test]
    fn test_persistence_survives_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("owner_allocation.json");
        let fleet = test_fleet_service(&dir);

        {
            let mut alloc = test_allocation_service(&dir);
            let rec = seed_proposed_recommendation(&mut alloc, &fleet);
            let mut owner = OwnerAllocationService::new(path.clone());
            owner.submit_decision(
                &mut alloc,
                AllocationDecision {
                    decision_id: "dec-persist".to_string(),
                    recommendation_id: rec.recommendation_id.clone(),
                    session_id: "s1".to_string(),
                    decision: "approved".to_string(),
                    alternative_node_id: None,
                    reason: None,
                    decided_at: chrono::Utc::now().to_rfc3339(),
                },
            );
        }

        {
            let owner = OwnerAllocationService::new(path.clone());
            let history = owner.get_decision_history();
            assert_eq!(history.len(), 1);
            assert_eq!(history[0].decision, "approved");
        }
    }

    #[test]
    fn test_empty_pending_queue_when_no_proposed() {
        let dir = tempdir().unwrap();
        let fleet = test_fleet_service(&dir);
        let mut alloc = test_allocation_service(&dir);
        let owner = test_owner_service(&dir);

        let queue = owner.get_pending_recommendations(&alloc);
        assert_eq!(queue.total_pending, 0);
        assert!(queue.items.is_empty());

        let rec = seed_proposed_recommendation(&mut alloc, &fleet);
        alloc.accept_recommendation(&rec.recommendation_id, None);

        let queue = owner.get_pending_recommendations(&alloc);
        assert_eq!(queue.total_pending, 0);
    }
}
