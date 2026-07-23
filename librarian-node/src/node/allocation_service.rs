use std::path::PathBuf;

use librarian_contracts::allocation::{
    AllocationReceipt, AllocationRecommendation, AllocationRequest,
    CapabilityMatch, CapabilityRequirement, SuitabilityScore,
};
use librarian_contracts::fleet::NodeInventoryEntry;
use uuid::Uuid;

use super::FleetService;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PersistedState {
    recommendations: Vec<AllocationRecommendation>,
    receipts: Vec<AllocationReceipt>,
}

pub struct AllocationService {
    recommendations: Vec<AllocationRecommendation>,
    receipts: Vec<AllocationReceipt>,
    persistence_path: PathBuf,
}

impl AllocationService {
    pub fn new(persistence_path: impl Into<PathBuf>) -> Self {
        let persistence_path = persistence_path.into();
        let (recommendations, receipts) = if persistence_path.exists() {
            match std::fs::read_to_string(&persistence_path) {
                Ok(content) => match serde_json::from_str::<PersistedState>(&content) {
                    Ok(state) => (state.recommendations, state.receipts),
                    Err(_) => (Vec::new(), Vec::new()),
                },
                Err(_) => (Vec::new(), Vec::new()),
            }
        } else {
            (Vec::new(), Vec::new())
        };

        AllocationService {
            recommendations,
            receipts,
            persistence_path,
        }
    }

    fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState {
            recommendations: self.recommendations.clone(),
            receipts: self.receipts.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.persistence_path, json);
        }
    }

    pub fn evaluate_requirements(
        &self,
        requirements: Vec<CapabilityRequirement>,
        nodes: Vec<NodeInventoryEntry>,
    ) -> Vec<CapabilityMatch> {
        let mut results = Vec::new();
        for node in &nodes {
            for req in &requirements {
                let has_capability = node.capability_count > 0;
                let is_verified = node.verified_capability_count > 0;

                let (matches, confidence) = if has_capability {
                    if is_verified {
                        (true, "confirmed")
                    } else {
                        (true, "likely")
                    }
                } else {
                    (false, "unknown")
                };

                results.push(CapabilityMatch {
                    node_id: node.node_id.clone(),
                    requirement_id: req.requirement_id.clone(),
                    matches,
                    evidence_verified: is_verified,
                    match_confidence: confidence.to_string(),
                    details: Some(format!(
                        "Node has {} capabilities ({} verified)",
                        node.capability_count, node.verified_capability_count
                    )),
                });
            }
        }
        results
    }

    pub fn score_nodes(
        &self,
        matches: Vec<CapabilityMatch>,
        nodes: Vec<NodeInventoryEntry>,
    ) -> Vec<SuitabilityScore> {
        let mut scores = Vec::new();
        for node in &nodes {
            let node_matches: Vec<&CapabilityMatch> =
                matches.iter().filter(|m| m.node_id == node.node_id).collect();
            let total = node_matches.len() as u32;
            let matched = node_matches.iter().filter(|m| m.matches).count() as u32;
            let evidence_verified = node_matches.iter().any(|m| m.evidence_verified);

            let score = if total > 0 {
                let match_ratio = matched as f64 / total as f64;
                let evidence_bonus = if evidence_verified { 0.1 } else { 0.0 };
                (match_ratio * 0.9 + evidence_bonus).min(1.0)
            } else {
                0.0
            };

            let mut notes = Vec::new();
            notes.push(format!("{}/{} requirements matched", matched, total));
            if evidence_verified {
                notes.push("Capabilities are evidence-verified".to_string());
            }
            notes.push(format!("Node status: {}", node.status));
            if node.registered {
                notes.push("Node is registered".to_string());
            }
            if node.bootstrap_completed {
                notes.push("Bootstrap completed".to_string());
            }

            scores.push(SuitabilityScore {
                node_id: node.node_id.clone(),
                score,
                requirement_matches: matched,
                requirement_total: total,
                constraints_satisfied: 0,
                constraints_total: 0,
                evidence_verified,
                notes,
            });
        }
        scores
    }

    pub fn generate_recommendation(
        &mut self,
        request: AllocationRequest,
        fleet: &FleetService,
    ) -> AllocationRecommendation {
        let nodes = fleet.all_nodes().to_vec();
        let matches = self.evaluate_requirements(request.requirements.clone(), nodes.clone());
        let scores = self.score_nodes(matches, nodes.clone());

        let best = scores
            .iter()
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));

        let (node_id, score, reasoning) = match best {
            Some(s) => {
                let reasoning = s.notes.clone();
                (s.node_id.clone(), s.clone(), reasoning)
            }
            None => {
                let reasoning = vec!["No suitable nodes found".to_string()];
                (
                    "none".to_string(),
                    SuitabilityScore {
                        node_id: "none".to_string(),
                        score: 0.0,
                        requirement_matches: 0,
                        requirement_total: request.requirements.len() as u32,
                        constraints_satisfied: 0,
                        constraints_total: 0,
                        evidence_verified: false,
                        notes: reasoning.clone(),
                    },
                    reasoning,
                )
            }
        };

        let recommendation = AllocationRecommendation {
            recommendation_id: Uuid::new_v4().to_string(),
            workload_id: request.request_id,
            node_id,
            score,
            reasoning,
            generated_at: chrono::Utc::now().to_rfc3339(),
            status: "proposed".to_string(),
        };

        self.recommendations.push(recommendation.clone());
        self.persist();
        recommendation
    }

    pub fn accept_recommendation(
        &mut self,
        recommendation_id: &str,
        session_id: Option<String>,
    ) -> Option<AllocationReceipt> {
        let rec = self
            .recommendations
            .iter_mut()
            .find(|r| r.recommendation_id == recommendation_id)?;
        rec.status = "accepted".to_string();

        let receipt = AllocationReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            recommendation_id: recommendation_id.to_string(),
            workload_id: rec.workload_id.clone(),
            node_id: rec.node_id.clone(),
            decided_by: "owner".to_string(),
            decision: "accepted".to_string(),
            decided_at: chrono::Utc::now().to_rfc3339(),
            session_id,
            custody_envelope_id: None,
        };

        self.receipts.push(receipt.clone());
        self.persist();
        Some(receipt)
    }

    pub fn reject_recommendation(
        &mut self,
        recommendation_id: &str,
        _reason: Option<String>,
    ) -> Option<AllocationReceipt> {
        let rec = self
            .recommendations
            .iter_mut()
            .find(|r| r.recommendation_id == recommendation_id)?;
        rec.status = "rejected".to_string();

        let receipt = AllocationReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            recommendation_id: recommendation_id.to_string(),
            workload_id: rec.workload_id.clone(),
            node_id: rec.node_id.clone(),
            decided_by: "owner".to_string(),
            decision: "rejected".to_string(),
            decided_at: chrono::Utc::now().to_rfc3339(),
            session_id: None,
            custody_envelope_id: None,
        };

        self.receipts.push(receipt.clone());
        self.persist();
        Some(receipt)
    }

    pub fn get_recommendations(&self, status_filter: Option<&str>) -> Vec<AllocationRecommendation> {
        match status_filter {
            Some(status) => self
                .recommendations
                .iter()
                .filter(|r| r.status == status)
                .cloned()
                .collect(),
            None => self.recommendations.clone(),
        }
    }

    pub fn get_receipts(&self) -> Vec<AllocationReceipt> {
        self.receipts.clone()
    }

    pub fn find_suitable_nodes(
        &self,
        requirements: Vec<CapabilityRequirement>,
        fleet: &FleetService,
    ) -> Vec<SuitabilityScore> {
        let nodes = fleet.all_nodes().to_vec();
        let matches = self.evaluate_requirements(requirements, nodes.clone());
        self.score_nodes(matches, nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_contracts::fleet::NodeInventoryEntry;
    use tempfile::tempdir;

    fn test_entry(
        node_id: &str,
        status: &str,
        cap_count: u32,
        verified_count: u32,
    ) -> NodeInventoryEntry {
        NodeInventoryEntry {
            node_id: node_id.to_string(),
            display_name: format!("node-{}", node_id),
            status: status.to_string(),
            last_seen_at: Some(chrono::Utc::now().to_rfc3339()),
            runtime_version: "0.1.0".to_string(),
            platform: "test".to_string(),
            capability_count: cap_count,
            verified_capability_count: verified_count,
            session_count: 2,
            custody_envelope_count: 1,
            registered: true,
            bootstrap_completed: true,
            last_health_status: Some(
                if status == "online" {
                    "healthy".to_string()
                } else {
                    "unhealthy".to_string()
                },
            ),
        }
    }

    fn test_requirement(id: &str, cap_type: &str, required: bool) -> CapabilityRequirement {
        CapabilityRequirement {
            requirement_id: id.to_string(),
            capability_type: cap_type.to_string(),
            required,
            constraints: None,
        }
    }

    fn test_allocation_request(request_id: &str) -> AllocationRequest {
        AllocationRequest {
            request_id: request_id.to_string(),
            workload_description: "Test workload".to_string(),
            requirements: vec![
                test_requirement("req-1", "inference", true),
                test_requirement("req-2", "vision", false),
            ],
            preferred_nodes: None,
            requested_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn test_service() -> (AllocationService, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("allocation.json");
        let service = AllocationService::new(path);
        (service, dir)
    }

    fn test_fleet_with_entries(
        entries: Vec<NodeInventoryEntry>,
    ) -> (FleetService, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("fleet-inventory.json");
        let mut fleet = FleetService::new(path);
        for entry in entries {
            fleet.add_or_update_node(entry);
        }
        (fleet, dir)
    }

    #[test]
    fn test_capability_matching_identifies_matches_and_non_matches() {
        let (service, _dir) = test_service();

        let nodes = vec![
            test_entry("node-a", "online", 5, 5),
            test_entry("node-b", "online", 3, 0),
            test_entry("node-c", "offline", 0, 0),
        ];

        let requirements = vec![
            test_requirement("req-1", "inference", true),
            test_requirement("req-2", "vision", false),
        ];

        let results = service.evaluate_requirements(requirements, nodes);

        assert_eq!(results.len(), 6);

        let node_a_matches: Vec<&CapabilityMatch> =
            results.iter().filter(|m| m.node_id == "node-a").collect();
        assert_eq!(node_a_matches.len(), 2);
        assert!(node_a_matches.iter().all(|m| m.matches));
        assert!(node_a_matches.iter().all(|m| m.evidence_verified));
        assert!(node_a_matches.iter().all(|m| m.match_confidence == "confirmed"));

        let node_b_matches: Vec<&CapabilityMatch> =
            results.iter().filter(|m| m.node_id == "node-b").collect();
        assert!(node_b_matches.iter().all(|m| m.matches));
        assert!(!node_b_matches.iter().any(|m| m.evidence_verified));
        assert!(node_b_matches.iter().all(|m| m.match_confidence == "likely"));

        let node_c_matches: Vec<&CapabilityMatch> =
            results.iter().filter(|m| m.node_id == "node-c").collect();
        assert!(node_c_matches.iter().all(|m| !m.matches));
        assert!(node_c_matches.iter().all(|m| m.match_confidence == "unknown"));
    }

    #[test]
    fn test_suitability_scoring_produces_0_to_1_range() {
        let (service, _dir) = test_service();

        let nodes = vec![
            test_entry("node-a", "online", 5, 5),
            test_entry("node-b", "online", 3, 0),
            test_entry("node-c", "offline", 0, 0),
        ];

        let requirements = vec![
            test_requirement("req-1", "inference", true),
            test_requirement("req-2", "vision", false),
        ];

        let matches = service.evaluate_requirements(requirements, nodes.clone());
        let scores = service.score_nodes(matches, nodes);

        assert_eq!(scores.len(), 3);

        for score in &scores {
            assert!(
                score.score >= 0.0 && score.score <= 1.0,
                "Score {} out of range for node {}",
                score.score,
                score.node_id
            );
            assert!(!score.notes.is_empty());
        }

        let node_a = scores.iter().find(|s| s.node_id == "node-a").unwrap();
        assert!(node_a.score > 0.0);
        assert_eq!(node_a.requirement_matches, 2);
        assert_eq!(node_a.requirement_total, 2);
        assert!(node_a.evidence_verified);

        let node_c = scores.iter().find(|s| s.node_id == "node-c").unwrap();
        assert_eq!(node_c.score, 0.0);
        assert_eq!(node_c.requirement_matches, 0);
        assert!(!node_c.evidence_verified);
    }

    #[test]
    fn test_recommendation_generation_uses_fleet_inventory() {
        let (mut service, _dir) = test_service();
        let entries = vec![
            test_entry("node-alpha", "online", 5, 5),
            test_entry("node-beta", "online", 3, 0),
            test_entry("node-gamma", "offline", 0, 0),
        ];
        let (fleet, _fleet_dir) = test_fleet_with_entries(entries);

        let request = test_allocation_request("workload-001");
        let recommendation = service.generate_recommendation(request, &fleet);

        assert_eq!(recommendation.recommendation_id.len(), 36);
        assert_eq!(recommendation.workload_id, "workload-001");
        assert_eq!(recommendation.status, "proposed");
        assert!(!recommendation.reasoning.is_empty());
        assert!(recommendation.score.score > 0.0);
        assert!(recommendation.node_id == "node-alpha");
    }

    #[test]
    fn test_accept_recommendation_creates_receipt_with_session_id() {
        let (mut service, _dir) = test_service();
        let entries = vec![test_entry("node-a", "online", 5, 5)];
        let (fleet, _fleet_dir) = test_fleet_with_entries(entries);

        let request = test_allocation_request("workload-002");
        let recommendation = service.generate_recommendation(request, &fleet);

        let receipt = service
            .accept_recommendation(&recommendation.recommendation_id, Some("session-001".to_string()))
            .expect("accept should succeed");

        assert_eq!(receipt.recommendation_id, recommendation.recommendation_id);
        assert_eq!(receipt.workload_id, "workload-002");
        assert_eq!(receipt.node_id, "node-a");
        assert_eq!(receipt.decision, "accepted");
        assert_eq!(receipt.session_id, Some("session-001".to_string()));
        assert_eq!(receipt.decided_by, "owner");
        assert!(!receipt.receipt_id.is_empty());
        assert!(!receipt.decided_at.is_empty());

        let updated = service.get_recommendations(Some("accepted"));
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].recommendation_id, recommendation.recommendation_id);
    }

    #[test]
    fn test_reject_recommendation_creates_receipt() {
        let (mut service, _dir) = test_service();
        let entries = vec![test_entry("node-a", "online", 5, 5)];
        let (fleet, _fleet_dir) = test_fleet_with_entries(entries);

        let request = test_allocation_request("workload-003");
        let recommendation = service.generate_recommendation(request, &fleet);

        let receipt = service
            .reject_recommendation(&recommendation.recommendation_id, Some("Insufficient resources".to_string()))
            .expect("reject should succeed");

        assert_eq!(receipt.recommendation_id, recommendation.recommendation_id);
        assert_eq!(receipt.decision, "rejected");
        assert_eq!(receipt.session_id, None);

        let rejected = service.get_recommendations(Some("rejected"));
        assert_eq!(rejected.len(), 1);
    }

    #[test]
    fn test_recommendations_and_receipts_persist_across_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("allocation-persist.json");

        let entries = vec![test_entry("node-a", "online", 5, 5)];
        let (fleet, _fleet_dir) = test_fleet_with_entries(entries);

        {
            let mut service = AllocationService::new(path.clone());
            let request = test_allocation_request("workload-004");
            let rec = service.generate_recommendation(request, &fleet);
            service.accept_recommendation(&rec.recommendation_id, None);
        }

        {
            let service = AllocationService::new(path.clone());
            let recs = service.get_recommendations(None);
            assert_eq!(recs.len(), 1);
            assert_eq!(recs[0].workload_id, "workload-004");

            let receipts = service.get_receipts();
            assert_eq!(receipts.len(), 1);
            assert_eq!(receipts[0].decision, "accepted");
        }
    }

    #[test]
    fn test_find_suitable_nodes_convenience() {
        let (service, _dir) = test_service();
        let entries = vec![
            test_entry("node-a", "online", 5, 5),
            test_entry("node-b", "offline", 0, 0),
        ];
        let (fleet, _fleet_dir) = test_fleet_with_entries(entries);

        let requirements = vec![test_requirement("req-1", "inference", true)];
        let scores = service.find_suitable_nodes(requirements, &fleet);

        assert_eq!(scores.len(), 2);

        let node_a = scores.iter().find(|s| s.node_id == "node-a").unwrap();
        assert!(node_a.score > 0.5);
        assert!(node_a.evidence_verified);

        let node_b = scores.iter().find(|s| s.node_id == "node-b").unwrap();
        assert_eq!(node_b.score, 0.0);
    }

    #[test]
    fn test_get_recommendations_with_status_filter() {
        let (mut service, _dir) = test_service();
        let entries = vec![test_entry("node-a", "online", 5, 5)];
        let (fleet, _fleet_dir) = test_fleet_with_entries(entries);

        let r1 = service.generate_recommendation(test_allocation_request("w1"), &fleet);
        let _r2 = service.generate_recommendation(test_allocation_request("w2"), &fleet);

        service.accept_recommendation(&r1.recommendation_id, None);

        let proposed = service.get_recommendations(Some("proposed"));
        assert_eq!(proposed.len(), 1);
        assert_eq!(proposed[0].workload_id, "w2");

        let all = service.get_recommendations(None);
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_empty_fleet_returns_no_match() {
        let (mut service, _dir) = test_service();
        let (fleet, _fleet_dir) = test_fleet_with_entries(vec![]);

        let request = test_allocation_request("empty-test");
        let rec = service.generate_recommendation(request, &fleet);

        assert_eq!(rec.status, "proposed");
        assert_eq!(rec.node_id, "none");
        assert_eq!(rec.score.score, 0.0);
        assert!(!rec.reasoning.is_empty());
    }
}
