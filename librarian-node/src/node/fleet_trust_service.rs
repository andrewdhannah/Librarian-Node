use std::collections::HashMap;
use std::path::PathBuf;

use librarian_contracts::fleet_trust::{
    trust_level_from_score, NodeTrustState, TrustAssessmentReceipt, TrustEvidence, TrustFactor,
};
use uuid::Uuid;

use super::anomaly_detection_service::AnomalyDetectionService;
use super::custody_service::CustodyService;
use super::fleet_service::FleetService;
use super::pattern_escalation_service::PatternEscalationService;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PersistedState {
    trust_states: Vec<NodeTrustState>,
    evidence_log: Vec<TrustEvidence>,
    receipts: Vec<TrustAssessmentReceipt>,
}

pub struct FleetTrustService {
    trust_states: HashMap<String, NodeTrustState>,
    evidence_log: Vec<TrustEvidence>,
    receipts: Vec<TrustAssessmentReceipt>,
    persistence_path: PathBuf,
}

impl FleetTrustService {
    pub fn new(persistence_path: impl Into<PathBuf>) -> Self {
        let persistence_path = persistence_path.into();
        let (trust_states, evidence_log, receipts) = if persistence_path.exists() {
            match std::fs::read_to_string(&persistence_path) {
                Ok(content) => match serde_json::from_str::<PersistedState>(&content) {
                    Ok(state) => {
                        let map: HashMap<String, NodeTrustState> = state
                            .trust_states
                            .into_iter()
                            .map(|ts| (ts.node_id.clone(), ts))
                            .collect();
                        (map, state.evidence_log, state.receipts)
                    }
                    Err(_) => (HashMap::new(), Vec::new(), Vec::new()),
                },
                Err(_) => (HashMap::new(), Vec::new(), Vec::new()),
            }
        } else {
            (HashMap::new(), Vec::new(), Vec::new())
        };

        FleetTrustService {
            trust_states,
            evidence_log,
            receipts,
            persistence_path,
        }
    }

    fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState {
            trust_states: self.trust_states.values().cloned().collect(),
            evidence_log: self.evidence_log.clone(),
            receipts: self.receipts.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.persistence_path, json);
        }
    }

    pub fn assess_node_trust(
        &mut self,
        node_id: &str,
        fleet: &FleetService,
        anomaly_service: &AnomalyDetectionService,
        pattern_service: &PatternEscalationService,
        custody_service: &CustodyService,
    ) -> NodeTrustState {
        let (chain_exists, integrity_verified, envelope_count) = {
            let chain = custody_service.get_chain();
            let exists = chain.is_some();
            let integrity = if exists {
                custody_service.verify_integrity().verified
            } else {
                false
            };
            let count = chain.map(|c| c.envelope_count).unwrap_or(0);
            (exists, integrity, count)
        };
        self.assess_node_trust_ext(node_id, fleet, anomaly_service, pattern_service, chain_exists, integrity_verified, envelope_count)
    }

    fn assess_node_trust_ext(
        &mut self,
        node_id: &str,
        fleet: &FleetService,
        anomaly_service: &AnomalyDetectionService,
        pattern_service: &PatternEscalationService,
        chain_exists: bool,
        integrity_verified: bool,
        envelope_count: u32,
    ) -> NodeTrustState {
        let previous_score = self
            .trust_states
            .get(node_id)
            .map(|s| s.score)
            .unwrap_or(0.0);

        let mut factors = Vec::new();

        let custody_score = self.compute_custody_score_ext(chain_exists, integrity_verified, envelope_count);
        factors.push(TrustFactor {
            name: "custody_integrity".to_string(),
            weight: 0.25,
            value: custody_score,
            description: format!("Custody chain integrity score: {:.2}", custody_score),
        });

        let anomaly_score = self.compute_anomaly_score(node_id, anomaly_service);
        factors.push(TrustFactor {
            name: "anomaly_recency_weighted".to_string(),
            weight: 0.25,
            value: anomaly_score,
            description: format!("Anomaly score (recency-weighted): {:.2}", anomaly_score),
        });

        let pattern_score = self.compute_pattern_severity_score(node_id, pattern_service);
        factors.push(TrustFactor {
            name: "pattern_severity".to_string(),
            weight: 0.15,
            value: pattern_score,
            description: format!("Pattern severity score: {:.2}", pattern_score),
        });

        let session_score = self.compute_session_success_score(node_id, fleet);
        factors.push(TrustFactor {
            name: "session_success_rate".to_string(),
            weight: 0.20,
            value: session_score,
            description: format!("Session success rate score: {:.2}", session_score),
        });

        let bootstrap_score = self.compute_bootstrap_completion_score(node_id, fleet);
        factors.push(TrustFactor {
            name: "bootstrap_completion".to_string(),
            weight: 0.15,
            value: bootstrap_score,
            description: format!("Bootstrap completion score: {:.2}", bootstrap_score),
        });

        let total_weight: f64 = factors.iter().map(|f| f.weight).sum();
        let new_score = if total_weight > 0.0 {
            (factors.iter().map(|f| f.weight * f.value).sum::<f64>() / total_weight * 100.0)
                .clamp(0.0, 100.0)
        } else {
            0.0
        };

        let trust_level = trust_level_from_score(new_score);
        let evidence_lines: Vec<String> = factors
            .iter()
            .map(|f| format!("{}: {:.1} (weight {:.0}%)", f.name, f.value, f.weight * 100.0))
            .collect();
        let evidence_summary = evidence_lines.join("; ");

        let state = NodeTrustState {
            node_id: node_id.to_string(),
            trust_level,
            score: (new_score * 100.0).round() / 100.0,
            evidence_summary,
            last_assessed_at: chrono::Utc::now().to_rfc3339(),
        };

        let receipt = TrustAssessmentReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            node_id: node_id.to_string(),
            previous_score: (previous_score * 100.0).round() / 100.0,
            new_score: state.score,
            factors,
            assessed_at: state.last_assessed_at.clone(),
        };

        self.evidence_log.push(TrustEvidence {
            evidence_id: Uuid::new_v4().to_string(),
            node_id: node_id.to_string(),
            metric: "trust_score".to_string(),
            value: state.score,
            timestamp: state.last_assessed_at.clone(),
        });

        self.receipts.push(receipt);
        self.trust_states.insert(node_id.to_string(), state.clone());
        self.persist();

        state
    }

    pub fn assess_all_nodes(
        &mut self,
        fleet: &FleetService,
        anomaly_service: &AnomalyDetectionService,
        pattern_service: &PatternEscalationService,
        custody_service: &CustodyService,
    ) -> Vec<NodeTrustState> {
        let node_ids: Vec<String> = fleet
            .all_nodes()
            .iter()
            .map(|n| n.node_id.clone())
            .collect();

        let mut results = Vec::new();
        for node_id in node_ids {
            let state = self.assess_node_trust(&node_id, fleet, anomaly_service, pattern_service, custody_service);
            results.push(state);
        }
        results
    }

    pub fn assess_all_nodes_ext(
        &mut self,
        fleet: &FleetService,
        anomaly_service: &AnomalyDetectionService,
        pattern_service: &PatternEscalationService,
        chain_exists: bool,
        integrity_verified: bool,
        envelope_count: u32,
    ) -> Vec<NodeTrustState> {
        let node_ids: Vec<String> = fleet
            .all_nodes()
            .iter()
            .map(|n| n.node_id.clone())
            .collect();

        let mut results = Vec::new();
        for node_id in node_ids {
            let state = self.assess_node_trust_ext(
                &node_id, fleet, anomaly_service, pattern_service,
                chain_exists, integrity_verified, envelope_count,
            );
            results.push(state);
        }
        results
    }

    pub fn get_node_trust(&self, node_id: &str) -> Option<NodeTrustState> {
        self.trust_states.get(node_id).cloned()
    }

    pub fn get_all_trust_states(&self) -> Vec<NodeTrustState> {
        let mut states: Vec<NodeTrustState> = self.trust_states.values().cloned().collect();
        states.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        states
    }

    pub fn get_receipts(&self) -> Vec<TrustAssessmentReceipt> {
        let mut receipts = self.receipts.clone();
        receipts.sort_by(|a, b| b.assessed_at.cmp(&a.assessed_at));
        receipts
    }

    fn compute_custody_score(&self, _node_id: &str, custody_service: &CustodyService) -> f64 {
        match custody_service.get_chain() {
            Some(chain) => {
                let integrity = custody_service.verify_integrity();
                let base = if integrity.verified { 1.0 } else { 0.4 };
                let envelope_factor = f64::min(chain.envelope_count as f64 / 10.0, 1.0);
                base * (0.6 + 0.4 * envelope_factor)
            }
            None => 0.3,
        }
    }

    fn compute_custody_score_ext(&self, chain_exists: bool, integrity_verified: bool, envelope_count: u32) -> f64 {
        if !chain_exists {
            return 0.3;
        }
        let base = if integrity_verified { 1.0 } else { 0.4 };
        let envelope_factor = f64::min(envelope_count as f64 / 10.0, 1.0);
        base * (0.6 + 0.4 * envelope_factor)
    }

    fn compute_anomaly_score(&self, _node_id: &str, anomaly_service: &AnomalyDetectionService) -> f64 {
        let findings = anomaly_service.get_all_baselines();
        if findings.is_empty() {
            return 0.8;
        }
        let now = chrono::Utc::now();
        let recent = findings
            .iter()
            .filter(|b| {
                chrono::DateTime::parse_from_rfc3339(&b.recorded_at)
                    .ok()
                    .map(|dt| (now - dt.to_utc()).num_hours() < 24)
                    .unwrap_or(false)
            })
            .count() as f64;
        let total = findings.len() as f64;
        let penalty = (recent / total).min(1.0) * 0.5;
        f64::max(1.0 - penalty, 0.0)
    }

    fn compute_pattern_severity_score(&self, _node_id: &str, pattern_service: &PatternEscalationService) -> f64 {
        let summary = pattern_service.get_summary();
        let total = summary.total_patterns as f64;
        if total == 0.0 {
            return 0.9;
        }
        let critical_count = summary.by_severity.critical as f64;
        let warning_count = summary.by_severity.warning as f64;
        let penalty = f64::min((critical_count * 0.4 + warning_count * 0.2) / total, 1.0);
        f64::max(1.0 - penalty, 0.0)
    }

    fn compute_session_success_score(&self, node_id: &str, fleet: &FleetService) -> f64 {
        match fleet.get_node(node_id) {
            Some(entry) => {
                let sessions = entry.session_count as f64;
                if sessions > 0.0 {
                    let success_rate = (sessions - 0.0) / sessions;
                    let registered_bonus = if entry.registered { 0.1 } else { 0.0 };
                    (success_rate * 0.8 + registered_bonus).min(1.0)
                } else {
                    if entry.registered { 0.6 } else { 0.3 }
                }
            }
            None => 0.0,
        }
    }

    fn compute_bootstrap_completion_score(&self, node_id: &str, fleet: &FleetService) -> f64 {
        match fleet.get_node(node_id) {
            Some(entry) => {
                if entry.bootstrap_completed {
                    1.0
                } else if entry.registered {
                    0.5
                } else {
                    0.0
                }
            }
            None => 0.0,
        }
    }

    pub fn publish_trust_to_fleet(&self, fleet: &mut FleetService) {
        for trust_state in self.trust_states.values() {
            if let Some(mut entry) = fleet.get_node(&trust_state.node_id) {
                entry.last_health_status = Some(format!("trust_{}", trust_state.trust_level));
                fleet.add_or_update_node(entry);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_contracts::fleet::NodeInventoryEntry;
    use tempfile::tempdir;

    fn test_fleet_with_node(node_id: &str) -> FleetService {
        let dir = tempdir().unwrap();
        let path = dir.path().join("fleet.json");
        let mut fleet = FleetService::new(path);
        fleet.add_or_update_node(NodeInventoryEntry {
            node_id: node_id.to_string(),
            display_name: format!("node-{}", node_id),
            status: "online".to_string(),
            last_seen_at: Some(chrono::Utc::now().to_rfc3339()),
            runtime_version: "0.1.0".to_string(),
            platform: "test".to_string(),
            capability_count: 3,
            verified_capability_count: 3,
            session_count: 5,
            custody_envelope_count: 3,
            registered: true,
            bootstrap_completed: true,
            last_health_status: Some("healthy".to_string()),
        });
        fleet
    }

    fn test_anomaly_service() -> AnomalyDetectionService {
        let dir = tempdir().unwrap();
        let path = dir.path().join("anomaly.json");
        AnomalyDetectionService::new(path)
    }

    fn test_pattern_service() -> PatternEscalationService {
        let dir = tempdir().unwrap();
        let path = dir.path().join("pattern.json");
        PatternEscalationService::new(path)
    }

    fn test_custody_service() -> CustodyService {
        let dir = tempdir().unwrap();
        let path = dir.path().join("custody.json");
        CustodyService::new(path)
    }

    fn test_service() -> (FleetTrustService, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("fleet-trust.json");
        let service = FleetTrustService::new(path);
        (service, dir)
    }

    #[test]
    fn test_assess_node_trust_returns_valid_state() {
        let (mut service, _dir) = test_service();
        let fleet = test_fleet_with_node("node-a");
        let anomaly = test_anomaly_service();
        let pattern = test_pattern_service();
        let custody = test_custody_service();

        let state = service.assess_node_trust("node-a", &fleet, &anomaly, &pattern, &custody);

        assert_eq!(state.node_id, "node-a");
        assert!(!state.evidence_summary.is_empty());
        assert!(!state.last_assessed_at.is_empty());
        assert!(state.score >= 0.0 && state.score <= 100.0);
        assert!(!state.trust_level.is_empty());
    }

    #[test]
    fn test_trust_level_mapping() {
        assert_eq!(trust_level_from_score(95.0), "trusted");
        assert_eq!(trust_level_from_score(80.0), "onboarding");
        assert_eq!(trust_level_from_score(60.0), "degraded");
        assert_eq!(trust_level_from_score(30.0), "suspended");
        assert_eq!(trust_level_from_score(10.0), "retired");
    }

    #[test]
    fn test_get_node_trust_after_assessment() {
        let (mut service, _dir) = test_service();
        let fleet = test_fleet_with_node("node-a");
        let anomaly = test_anomaly_service();
        let pattern = test_pattern_service();
        let custody = test_custody_service();

        service.assess_node_trust("node-a", &fleet, &anomaly, &pattern, &custody);

        let retrieved = service.get_node_trust("node-a");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().node_id, "node-a");
    }

    #[test]
    fn test_assess_all_nodes_evaluates_entire_fleet() {
        let (mut service, _dir) = test_service();
        let mut fleet = test_fleet_with_node("node-a");
        let anomaly = test_anomaly_service();
        let pattern = test_pattern_service();
        let custody = test_custody_service();

        fleet.add_or_update_node(NodeInventoryEntry {
            node_id: "node-b".to_string(),
            display_name: "node-b".to_string(),
            status: "online".to_string(),
            last_seen_at: Some(chrono::Utc::now().to_rfc3339()),
            runtime_version: "0.1.0".to_string(),
            platform: "test".to_string(),
            capability_count: 1,
            verified_capability_count: 0,
            session_count: 1,
            custody_envelope_count: 0,
            registered: false,
            bootstrap_completed: false,
            last_health_status: None,
        });

        let results = service.assess_all_nodes(&fleet, &anomaly, &pattern, &custody);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_get_receipts_returns_assessment_history() {
        let (mut service, _dir) = test_service();
        let fleet = test_fleet_with_node("node-a");
        let anomaly = test_anomaly_service();
        let pattern = test_pattern_service();
        let custody = test_custody_service();

        service.assess_node_trust("node-a", &fleet, &anomaly, &pattern, &custody);

        let receipts = service.get_receipts();
        assert_eq!(receipts.len(), 1);
        assert!(!receipts[0].receipt_id.is_empty());
    }

    #[test]
    fn test_publish_trust_to_fleet_updates_health_status() {
        let (mut service, _dir) = test_service();
        let mut fleet = test_fleet_with_node("node-a");
        let anomaly = test_anomaly_service();
        let pattern = test_pattern_service();
        let custody = test_custody_service();

        service.assess_node_trust("node-a", &fleet, &anomaly, &pattern, &custody);
        service.publish_trust_to_fleet(&mut fleet);

        let entry = fleet.get_node("node-a").unwrap();
        assert!(entry.last_health_status.unwrap_or_default().starts_with("trust_"));
    }

    #[test]
    fn test_get_all_trust_states_returns_sorted() {
        let (mut service, _dir) = test_service();
        let mut fleet = test_fleet_with_node("node-a");
        let anomaly = test_anomaly_service();
        let pattern = test_pattern_service();
        let custody = test_custody_service();

        fleet.add_or_update_node(NodeInventoryEntry {
            node_id: "node-b".to_string(),
            display_name: "node-b".to_string(),
            status: "online".to_string(),
            last_seen_at: Some(chrono::Utc::now().to_rfc3339()),
            runtime_version: "0.1.0".to_string(),
            platform: "test".to_string(),
            capability_count: 0,
            verified_capability_count: 0,
            session_count: 0,
            custody_envelope_count: 0,
            registered: false,
            bootstrap_completed: false,
            last_health_status: None,
        });

        service.assess_all_nodes(&fleet, &anomaly, &pattern, &custody);
        let states = service.get_all_trust_states();
        assert_eq!(states.len(), 2);

        for i in 1..states.len() {
            assert!(states[i - 1].score >= states[i].score);
        }
    }

    #[test]
    fn test_persistence_survives_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("fleet-trust-persist.json");

        {
            let mut service = FleetTrustService::new(path.clone());
            let fleet = test_fleet_with_node("node-a");
            let anomaly = test_anomaly_service();
            let pattern = test_pattern_service();
            let custody = test_custody_service();
            service.assess_node_trust("node-a", &fleet, &anomaly, &pattern, &custody);
        }

        {
            let service = FleetTrustService::new(path.clone());
            let state = service.get_node_trust("node-a");
            assert!(state.is_some());
            assert!(state.unwrap().score > 0.0);
        }
    }

    #[test]
    fn test_score_factors_are_bounded() {
        let (mut service, _dir) = test_service();
        let fleet = test_fleet_with_node("node-a");
        let anomaly = test_anomaly_service();
        let pattern = test_pattern_service();
        let custody = test_custody_service();

        let state = service.assess_node_trust("node-a", &fleet, &anomaly, &pattern, &custody);
        assert!(state.score >= 0.0 && state.score <= 100.0);
    }
}
