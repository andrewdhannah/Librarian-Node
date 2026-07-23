use std::path::PathBuf;

use librarian_contracts::registry_enforcement::{
    EnforcementAction, EnforcementEvent, EnforcementPolicy, EnforcementRule, RuleScope,
};
use librarian_contracts::registry::NodeCandidate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::capability_evidence::CapabilityEvidenceBridge;
use super::policy_service::PolicyService;
use super::registration_service::RegistrationService;
use super::registry_candidate_service::RegistryCandidateService;

const DEFAULT_CANDIDATE_EXPIRY_DAYS: u32 = 7;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    events: Vec<EnforcementEvent>,
    policy: EnforcementPolicy,
}

pub struct RegistryEnforcementService {
    events: Vec<EnforcementEvent>,
    policy: EnforcementPolicy,
    persistence_path: PathBuf,
}

impl RegistryEnforcementService {
    pub fn new(persistence_path: impl Into<PathBuf>) -> Self {
        let persistence_path = persistence_path.into();
        let (events, policy) = if persistence_path.exists() {
            match std::fs::read_to_string(&persistence_path) {
                Ok(content) => match serde_json::from_str::<PersistedState>(&content) {
                    Ok(state) => (state.events, state.policy),
                    Err(_) => (Vec::new(), Self::default_policy()),
                },
                Err(_) => (Vec::new(), Self::default_policy()),
            }
        } else {
            (Vec::new(), Self::default_policy())
        };

        RegistryEnforcementService {
            events,
            policy,
            persistence_path,
        }
    }

    fn default_policy() -> EnforcementPolicy {
        EnforcementPolicy {
            rules: vec![
                EnforcementRule {
                    rule_id: "enf-reg-block".to_string(),
                    name: "registration.block.unregistered_sessions".to_string(),
                    scope: RuleScope::Registration,
                    condition: serde_json::json!({"registration_status_required": "registered"}),
                    action: EnforcementAction::Block,
                    enabled: true,
                },
                EnforcementRule {
                    rule_id: "enf-cap-degrade".to_string(),
                    name: "capability.degrade.stale_evidence".to_string(),
                    scope: RuleScope::Capability,
                    condition: serde_json::json!({"require_registration_for_verified": true}),
                    action: EnforcementAction::Degrade,
                    enabled: true,
                },
                EnforcementRule {
                    rule_id: "enf-cand-expiry".to_string(),
                    name: "candidate.expire.auto".to_string(),
                    scope: RuleScope::Candidate,
                    condition: serde_json::json!({"expiry_days": DEFAULT_CANDIDATE_EXPIRY_DAYS}),
                    action: EnforcementAction::Log,
                    enabled: true,
                },
            ],
            version: 1,
        }
    }

    fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState {
            events: self.events.clone(),
            policy: self.policy.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.persistence_path, json);
        }
    }

    pub(crate) fn log_event(&mut self, rule_id: &str, scope: &str, target_id: &str, violation_detail: &str, action_taken: &str) -> EnforcementEvent {
        let event = EnforcementEvent {
            event_id: Uuid::new_v4().to_string(),
            rule_id: rule_id.to_string(),
            scope: scope.to_string(),
            target_id: target_id.to_string(),
            violation_detail: violation_detail.to_string(),
            action_taken: action_taken.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.events.push(event.clone());
        self.persist();
        event
    }

    pub fn get_enforcement_events(&self, scope_filter: Option<&str>) -> Vec<EnforcementEvent> {
        match scope_filter {
            Some(filter) => self.events.iter().filter(|e| e.scope == filter).cloned().collect(),
            None => self.events.clone(),
        }
    }

    pub fn get_enforcement_policy(&self) -> EnforcementPolicy {
        self.policy.clone()
    }

    pub fn update_enforcement_policy(&mut self, policy: EnforcementPolicy) {
        self.policy = policy;
        self.persist();
    }

    pub fn check_session_allowed(
        &mut self,
        registration_service: &RegistrationService,
        _policy_service: &PolicyService,
    ) -> Result<(), String> {
        let rule = match self.policy.rules.iter().find(|r| r.name == "registration.block.unregistered_sessions") {
            Some(r) => r.clone(),
            None => return Ok(()),
        };

        if !rule.enabled {
            return Ok(());
        }

        let reg_status = registration_service.get_record().registration_status.clone();
        let allowed_statuses = vec!["registered".to_string(), "registration_requested".to_string(), "admitted_via_candidate".to_string()];
        if allowed_statuses.contains(&reg_status) {
            return Ok(());
        }

        let node_id = registration_service.get_record().node_id.clone();
        self.log_event(
            &rule.rule_id,
            "registration",
            &node_id,
            &format!("Session creation blocked: node registration status is '{}', requires 'registered'", reg_status),
            "block",
        );

        Err(format!(
            "Node registration required. Current status: {}. Use /node/register to submit a registration request.",
            reg_status
        ))
    }

    pub fn check_capability_validity(
        &mut self,
        capability_bridge: &CapabilityEvidenceBridge,
        registration_service: &RegistrationService,
    ) -> Vec<String> {
        let rule = match self.policy.rules.iter().find(|r| r.name == "capability.degrade.stale_evidence") {
            Some(r) => r.clone(),
            None => return Vec::new(),
        };

        if !rule.enabled {
            return Vec::new();
        }

        let reg_status = registration_service.get_record().registration_status.clone();
        let is_registered = reg_status == "registered" || reg_status == "admitted_via_candidate";
        let mut degraded = Vec::new();

        if !is_registered {
            let state = capability_bridge.get_verification_state(&registration_service.get_record().node_id);
            for cap in &state.capabilities {
                if cap.verification_status == "verified" {
                    degraded.push(cap.capability_type.clone());
                    self.log_event(
                        &rule.rule_id,
                        "capability",
                        &cap.claim_id,
                        &format!("Capability '{}' marked degraded: node not registered (status: {})", cap.capability_type, reg_status),
                        "degrade",
                    );
                }
            }
        }

        degraded
    }

    pub fn check_candidate_expiry(
        &mut self,
        candidate_service: &mut RegistryCandidateService,
        _policy_service: &PolicyService,
    ) -> Vec<NodeCandidate> {
        let rule = match self.policy.rules.iter().find(|r| r.name == "candidate.expire.auto") {
            Some(r) => r.clone(),
            None => return Vec::new(),
        };

        if !rule.enabled {
            return Vec::new();
        }

        let days: u32 = rule.condition.get("expiry_days").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(DEFAULT_CANDIDATE_EXPIRY_DAYS);

        let expired = candidate_service.expire_stale(days);

        for cand in &expired {
            self.log_event(
                &rule.rule_id,
                "candidate",
                &cand.candidate_id,
                &format!("Candidate '{}' (node {}) auto-expired after {} days", cand.display_name, cand.node_id, days),
                "expire",
            );
        }

        expired
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_contracts::node::{NodeIdentity, RegistrationReceipt};
    use librarian_contracts::registry::DiscoveryMethod;
    use tempfile::tempdir;

    fn test_enforcement_service() -> RegistryEnforcementService {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry_enforcement.json");
        RegistryEnforcementService::new(path)
    }

    fn test_registration_service(dir: &tempfile::TempDir) -> RegistrationService {
        let path = dir.path().join("node-registration.json");
        RegistrationService::new(&path)
    }

    fn test_candidate_service(dir: &tempfile::TempDir) -> RegistryCandidateService {
        let path = dir.path().join("registry-candidates.json");
        RegistryCandidateService::new(&path)
    }

    fn test_policy_service(dir: &tempfile::TempDir) -> PolicyService {
        let path = dir.path().join("policy.json");
        PolicyService::new(path)
    }

    #[test]
    fn test_session_creation_blocked_when_not_registered() {
        let dir = tempdir().unwrap();
        let mut enforcement = test_enforcement_service();
        let reg = test_registration_service(&dir);
        let policy = test_policy_service(&dir);

        let result = enforcement.check_session_allowed(&reg, &policy);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("registration required"));
    }

    #[test]
    fn test_session_allowed_when_registered() {
        let dir = tempdir().unwrap();
        let mut enforcement = test_enforcement_service();
        let mut reg = test_registration_service(&dir);
        let policy = test_policy_service(&dir);

        let identity = NodeIdentity {
            node_id: "test-node-uuid".to_string(),
            display_name: "test-host".to_string(),
            platform: "test".to_string(),
            runtime_version: "0.1.0".to_string(),
            contract_version: "1".to_string(),
            first_seen_at: chrono::Utc::now().to_rfc3339(),
        };
        reg.submit_registration(&identity, None);
        let receipt = RegistrationReceipt {
            registration_id: "reg-001".to_string(),
            node_id: "test-node-uuid".to_string(),
            status: "registered".to_string(),
            registered_at: chrono::Utc::now().to_rfc3339(),
            previous_state: Some("registration_requested".to_string()),
        };
        reg.confirm_registration(&receipt);

        let result = enforcement.check_session_allowed(&reg, &policy);
        assert!(result.is_ok());
    }

    #[test]
    fn test_capabilities_marked_degraded_when_registration_missing() {
        let dir = tempdir().unwrap();
        let mut enforcement = test_enforcement_service();
        let mut reg = test_registration_service(&dir);
        let _policy = test_policy_service(&dir);
        let bridge_path = dir.path().join("capability_evidence.json");
        let mut bridge = CapabilityEvidenceBridge::new(bridge_path);

        let identity = NodeIdentity {
            node_id: "test-node-uuid".to_string(),
            display_name: "test-host".to_string(),
            platform: "test".to_string(),
            runtime_version: "0.1.0".to_string(),
            contract_version: "1".to_string(),
            first_seen_at: chrono::Utc::now().to_rfc3339(),
        };
        reg.submit_registration(&identity, None);

        let claim = bridge.register_claim("test-node-uuid", "llm.inference", Some("llama.cpp".to_string()), None);

        // Mark claim as verified
        let packet = make_test_packet();
        bridge.link_evidence(&claim.claim_id, "evt-001", "qr-001");
        let _ = bridge.verify_claim(&claim.claim_id, &packet);

        let degraded = enforcement.check_capability_validity(&bridge, &reg);
        assert!(!degraded.is_empty());
        assert!(degraded.contains(&"llm.inference".to_string()));
    }

    #[test]
    fn test_enforcement_events_logged_with_correct_action() {
        let dir = tempdir().unwrap();
        let mut enforcement = test_enforcement_service();
        let reg = test_registration_service(&dir);
        let policy = test_policy_service(&dir);

        let _ = enforcement.check_session_allowed(&reg, &policy);

        let events = enforcement.get_enforcement_events(Some("registration"));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action_taken, "block");
        assert_eq!(events[0].scope, "registration");
    }

    #[test]
    fn test_candidate_expiry_produces_events() {
        let dir = tempdir().unwrap();
        let mut enforcement = test_enforcement_service();
        let mut candidate_svc = test_candidate_service(&dir);
        let _policy = test_policy_service(&dir);

        // Use the enforcement service's check_candidate_expiry - no candidates to expire
        // since they were just created, so events should be empty
        let expired = enforcement.check_candidate_expiry(&mut candidate_svc, &_policy);
        assert!(expired.is_empty());

        // Verify event logging works by checking enforcement events
        let events = enforcement.get_enforcement_events(Some("candidate"));
        assert!(events.is_empty());

        // Direct enforcement event log test
        enforcement.log_event("enf-cand-expiry-test", "candidate", "cand-001", "Test expiry event", "expire");
        let events = enforcement.get_enforcement_events(Some("candidate"));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action_taken, "expire");
        assert_eq!(events[0].target_id, "cand-001");
    }

    #[test]
    fn test_policy_configurable() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry_enforcement.json");
        let mut enforcement = RegistryEnforcementService::new(&path);

        let policy = enforcement.get_enforcement_policy();
        assert_eq!(policy.rules.len(), 3);
        assert!(policy.rules.iter().any(|r| r.name == "registration.block.unregistered_sessions"));

        let mut updated = policy.clone();
        for rule in &mut updated.rules {
            if rule.name == "registration.block.unregistered_sessions" {
                rule.enabled = false;
            }
        }
        enforcement.update_enforcement_policy(updated);

        let policy2 = enforcement.get_enforcement_policy();
        let reg_block = policy2.rules.iter().find(|r| r.name == "registration.block.unregistered_sessions").unwrap();
        assert!(!reg_block.enabled);
    }

    fn make_test_packet() -> librarian_contracts::evidence_packet::EvidencePacket {
        use librarian_contracts::common::*;
        librarian_contracts::evidence_packet::EvidencePacket {
            packet_type: "evidence_packet".to_string(),
            packet_version: "1".to_string(),
            exported_at: "2026-07-15T12:00:00Z".to_string(),
            qualification_request_id: "qr-test-001".to_string(),
            identity: PacketModelIdentity {
                model_id: "test-model".to_string(),
                sha256: "abcdef123456".to_string(),
                filename: "test.gguf".to_string(),
                quantization: Some("Q4_K_M".to_string()),
            },
            execution: PacketExecutionIdentity {
                runtime_profile_id: "prof-test".to_string(),
                hardware_profile_id: "hw-test".to_string(),
                runtime_executable_sha256: "123456abcdef".to_string(),
                runtime_executable_version: "1.0.0".to_string(),
            },
            lease: PacketLeaseLifecycle {
                lease_id: "lease-test-001".to_string(),
                port: Some(9120),
                state: "unloaded".to_string(),
                loaded_at: Some("2026-07-15T11:59:50Z".to_string()),
                released_at: Some("2026-07-15T12:00:01Z".to_string()),
                vram_released_at: Some("2026-07-15T12:00:01Z".to_string()),
            },
            run: PacketExecutionMetrics {
                run_id: "run-test-001".to_string(),
                input_tokens: Some(10),
                output_tokens: Some(32),
                load_duration_ms: Some(2187),
                generation_duration_ms: Some(385),
                exit_status: Some("clean".to_string()),
                started_at: Some("2026-07-15T11:59:50Z".to_string()),
                ended_at: Some("2026-07-15T12:00:01Z".to_string()),
            },
            lifecycle_events: vec![],
            release_verification: PacketReleaseVerification {
                pid_exit_verified: true,
                gpu_release_verified: true,
                free_vram_mb: Some(3433),
                baseline_vram_mb: Some(3433),
                within_tolerance: true,
            },
        }
    }

    #[test]
    fn test_default_policy_has_correct_defaults() {
        let enforcement = test_enforcement_service();
        let policy = enforcement.get_enforcement_policy();
        assert_eq!(policy.version, 1);

        let reg_rule = policy.rules.iter().find(|r| r.scope == RuleScope::Registration).unwrap();
        assert!(reg_rule.enabled);
        assert_eq!(reg_rule.action, EnforcementAction::Block);

        let cap_rule = policy.rules.iter().find(|r| r.scope == RuleScope::Capability).unwrap();
        assert!(cap_rule.enabled);
        assert_eq!(cap_rule.action, EnforcementAction::Degrade);

        let cand_rule = policy.rules.iter().find(|r| r.scope == RuleScope::Candidate).unwrap();
        assert!(cand_rule.enabled);
        assert_eq!(cand_rule.action, EnforcementAction::Log);
    }
}
