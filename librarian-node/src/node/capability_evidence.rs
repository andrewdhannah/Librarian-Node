use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use librarian_contracts::capability_evidence::{
    CapabilityClaim, CapabilityState, CapabilityStateChangeReceipt, CapabilityVerificationState,
    EvidenceReference, VerifiedCapability,
};
use librarian_contracts::custody::CustodyMetadata;
use librarian_contracts::evidence_packet::EvidencePacket;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::CustodyService;

pub struct CapabilityEvidenceBridge {
    claims: Vec<CapabilityClaim>,
    evidence_refs: Vec<EvidenceReference>,
    lifecycle_records: Vec<CapabilityStateChangeReceipt>,
    persistence_path: PathBuf,
    custody_service: Option<Arc<std::sync::Mutex<CustodyService>>>,
}

impl CapabilityEvidenceBridge {
    pub fn new(persistence_path: PathBuf) -> Self {
        let (claims, evidence_refs, lifecycle_records) = if persistence_path.exists() {
            match std::fs::read_to_string(&persistence_path) {
                Ok(content) => match serde_json::from_str::<BridgeState>(&content) {
                    Ok(state) => (state.claims, state.evidence_refs, state.lifecycle_records),
                    Err(_) => (Vec::new(), Vec::new(), Vec::new()),
                },
                Err(_) => (Vec::new(), Vec::new(), Vec::new()),
            }
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };

        CapabilityEvidenceBridge {
            claims,
            evidence_refs,
            lifecycle_records,
            persistence_path,
            custody_service: None,
        }
    }

    pub fn with_custody(mut self, custody: Arc<std::sync::Mutex<CustodyService>>) -> Self {
        self.custody_service = Some(custody);
        self
    }

    fn get_current_state(&self, capability_type: &str) -> CapabilityState {
        self.lifecycle_records
            .iter()
            .filter(|r| r.capability_type == capability_type)
            .max_by(|a, b| a.changed_at.cmp(&b.changed_at))
            .map(|r| r.new_state.clone())
            .unwrap_or(CapabilityState::Discovered)
    }

    fn custody_state_change(&self, receipt: &CapabilityStateChangeReceipt) {
        if let Some(ref custody) = self.custody_service {
            let node_id = self
                .claims
                .first()
                .map(|c| c.node_id.clone())
                .unwrap_or_default();
            if !node_id.is_empty() {
                let payload = serde_json::to_value(receipt).unwrap_or_default();
                let metadata = CustodyMetadata {
                    source: "node".to_string(),
                    version: "1".to_string(),
                    notes: Some(format!(
                        "Capability state transition: {:?} -> {:?}",
                        receipt.previous_state, receipt.new_state
                    )),
                };
                let mut guard = custody.lock().unwrap();
                guard.append_receipt(
                    &node_id,
                    "capability_state_change",
                    &receipt.receipt_id,
                    payload,
                    Some(metadata),
                );
            }
        }
    }

    pub fn transition_state(
        &mut self,
        capability_type: &str,
        new_state: CapabilityState,
        reason: &str,
    ) -> Result<CapabilityStateChangeReceipt, String> {
        let current_state = self.get_current_state(capability_type);
        let allowed = current_state.valid_transitions();
        if !allowed.contains(&new_state) {
            return Err(format!(
                "Invalid state transition from {:?} to {:?} for capability '{}'",
                current_state, new_state, capability_type
            ));
        }

        let receipt = CapabilityStateChangeReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            capability_type: capability_type.to_string(),
            previous_state: current_state,
            new_state: new_state,
            reason: reason.to_string(),
            changed_at: chrono::Utc::now().to_rfc3339(),
        };

        self.lifecycle_records.push(receipt.clone());
        self.custody_state_change(&receipt);
        Ok(receipt)
    }

    pub fn degrade_if_not_registered(&mut self, _node_id: &str, is_registered: bool) -> Vec<String> {
        if is_registered {
            return Vec::new();
        }
        let mut degraded = Vec::new();
        for claim in self.claims.iter_mut() {
            if claim.status == "verified" {
                claim.status = "unverified".to_string();
                degraded.push(claim.capability_type.clone());
            }
        }
        degraded
    }

    pub fn get_verification_state(&self, node_id: &str) -> CapabilityVerificationState {
        let mut cap_map: std::collections::BTreeMap<String, Vec<&CapabilityClaim>> =
            std::collections::BTreeMap::new();
        for claim in &self.claims {
            cap_map
                .entry(claim.capability_type.clone())
                .or_default()
                .push(claim);
        }

        let mut capabilities = Vec::new();
        for (cap_type, claims) in cap_map {
            if let Some(latest) = claims.into_iter().max_by_key(|c| &c.claimed_at) {
                let refs: Vec<EvidenceReference> = self
                    .evidence_refs
                    .iter()
                    .filter(|r| r.claim_id == latest.claim_id)
                    .cloned()
                    .collect();

                capabilities.push(VerifiedCapability {
                    capability_type: cap_type.clone(),
                    claim_id: latest.claim_id.clone(),
                    verification_status: latest.status.clone(),
                    last_verified_at: refs
                        .iter()
                        .filter_map(|r| r.verified_at.clone())
                        .max(),
                    evidence_references: refs,
                    state: self.get_current_state(&cap_type),
                });
            }
        }

        CapabilityVerificationState {
            node_id: node_id.to_string(),
            capabilities,
        }
    }

    pub fn register_claim(
        &mut self,
        node_id: &str,
        capability_type: &str,
        runtime: Option<String>,
        model_id: Option<String>,
    ) -> CapabilityClaim {
        let claim = CapabilityClaim {
            claim_id: Uuid::new_v4().to_string(),
            node_id: node_id.to_string(),
            capability_type: capability_type.to_string(),
            runtime,
            model_id,
            claim_version: "1".to_string(),
            claimed_at: chrono::Utc::now().to_rfc3339(),
            status: "unverified".to_string(),
        };
        self.claims.push(claim.clone());

        let current = self.get_current_state(capability_type);
        if current == CapabilityState::Discovered {
            let _ = self.transition_state(
                capability_type,
                CapabilityState::PendingVerification,
                "Claim registered, pending evidence",
            );
        }

        claim
    }

    pub fn link_evidence(
        &mut self,
        claim_id: &str,
        evidence_packet_id: &str,
        qualification_run_id: &str,
    ) -> Option<EvidenceReference> {
        if !self.claims.iter().any(|c| c.claim_id == claim_id) {
            return None;
        }

        let reference = EvidenceReference {
            reference_id: Uuid::new_v4().to_string(),
            claim_id: claim_id.to_string(),
            evidence_packet_id: evidence_packet_id.to_string(),
            qualification_run_id: qualification_run_id.to_string(),
            verification_status: "pending".to_string(),
            verified_at: None,
            evidence_hash: None,
        };
        self.evidence_refs.push(reference.clone());
        Some(reference)
    }

    pub fn verify_claim(
        &mut self,
        claim_id: &str,
        evidence_packet: &EvidencePacket,
    ) -> Result<String, String> {
        let claim = self
            .claims
            .iter_mut()
            .find(|c| c.claim_id == claim_id)
            .ok_or_else(|| format!("Claim {} not found", claim_id))?;

        let cap_type = claim.capability_type.clone();
        let node_id = claim.node_id.clone();
        let _ = claim;

        match evidence_packet.validate() {
            Ok(()) => {
                let hash = evidence_packet
                    .compute_hash()
                    .map_err(|e| format!("Hash computation failed: {}", e))?;

                if let Some(c) = self.claims.iter_mut().find(|c| c.claim_id == claim_id) {
                    c.status = "verified".to_string();
                }

                for reference in self.evidence_refs.iter_mut() {
                    if reference.claim_id == claim_id
                        && reference.verification_status == "pending"
                    {
                        reference.verification_status = "passed".to_string();
                        reference.verified_at = Some(chrono::Utc::now().to_rfc3339());
                        reference.evidence_hash = Some(hash.clone());
                    }
                }

                let current = self.get_current_state(&cap_type);
                if current == CapabilityState::PendingVerification
                    || current == CapabilityState::Discovered
                {
                    let _ = self.transition_state(
                        &cap_type,
                        CapabilityState::Verified,
                        "Evidence linked and verified",
                    );
                }

                if let Some(ref custody) = self.custody_service {
                    let state = self.get_verification_state(&node_id);
                    let payload = serde_json::to_value(&state).unwrap_or_default();
                    let metadata = CustodyMetadata {
                        source: "node".to_string(),
                        version: "1".to_string(),
                        notes: Some("Auto-custodied on capability verification".to_string()),
                    };
                    let mut guard = custody.lock().unwrap();
                    guard.append_receipt(
                        &node_id,
                        "capability_evidence",
                        claim_id,
                        payload,
                        Some(metadata),
                    );
                }

                Ok("verified".to_string())
            }
            Err(e) => {
                if let Some(c) = self.claims.iter_mut().find(|c| c.claim_id == claim_id) {
                    c.status = "failed".to_string();
                }

                for reference in self.evidence_refs.iter_mut() {
                    if reference.claim_id == claim_id
                        && reference.verification_status == "pending"
                    {
                        reference.verification_status = "failed".to_string();
                        reference.verified_at = Some(chrono::Utc::now().to_rfc3339());
                    }
                }

                Err(format!("Evidence validation failed: {}", e))
            }
        }
    }

    pub fn get_unverified_claims(&self) -> Vec<&CapabilityClaim> {
        self.claims
            .iter()
            .filter(|c| c.status == "unverified")
            .collect()
    }

    pub fn get_verified_capabilities(&self) -> Vec<(&CapabilityClaim, Vec<&EvidenceReference>)> {
        self.claims
            .iter()
            .filter(|c| c.status == "verified")
            .map(|claim| {
                let refs: Vec<&EvidenceReference> = self
                    .evidence_refs
                    .iter()
                    .filter(|r| r.claim_id == claim.claim_id)
                    .collect();
                (claim, refs)
            })
            .collect()
    }

    pub fn get_capability_lifecycle(&self) -> Vec<Value> {
        let mut cap_types: BTreeSet<String> = BTreeSet::new();
        for record in &self.lifecycle_records {
            cap_types.insert(record.capability_type.clone());
        }
        for claim in &self.claims {
            cap_types.insert(claim.capability_type.clone());
        }

        cap_types
            .iter()
            .map(|cap_type| {
                let current_state = self.get_current_state(cap_type);
                let history: Vec<&CapabilityStateChangeReceipt> = self
                    .lifecycle_records
                    .iter()
                    .filter(|r| r.capability_type == *cap_type)
                    .collect();

                serde_json::json!({
                    "capability_type": cap_type,
                    "current_state": current_state.as_str(),
                    "state_changes": history,
                    "change_count": history.len(),
                })
            })
            .collect()
    }

    pub fn persist(&self) -> Result<(), String> {
        if let Some(parent) = self.persistence_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
        }
        let state = BridgeState {
            claims: self.claims.clone(),
            evidence_refs: self.evidence_refs.clone(),
            lifecycle_records: self.lifecycle_records.clone(),
        };
        let json = serde_json::to_string_pretty(&state)
            .map_err(|e| format!("Serialization failed: {}", e))?;
        std::fs::write(&self.persistence_path, json)
            .map_err(|e| format!("Write failed: {}", e))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct BridgeState {
    claims: Vec<CapabilityClaim>,
    evidence_refs: Vec<EvidenceReference>,
    #[serde(default)]
    lifecycle_records: Vec<CapabilityStateChangeReceipt>,
}

impl Serialize for CapabilityEvidenceBridge {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        BridgeState {
            claims: self.claims.clone(),
            evidence_refs: self.evidence_refs.clone(),
            lifecycle_records: self.lifecycle_records.clone(),
        }
        .serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_bridge() -> CapabilityEvidenceBridge {
        let dir = tempdir().unwrap();
        let path = dir.path().join("capability_evidence.json");
        CapabilityEvidenceBridge::new(path)
    }

    fn test_evidence_packet() -> EvidencePacket {
        use librarian_contracts::common::*;
        EvidencePacket {
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
    fn test_claim_creation_and_persistence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_claim.json");
        let mut bridge = CapabilityEvidenceBridge::new(path.clone());

        let claim = bridge.register_claim("test-node", "inference", Some("llama.cpp".to_string()), None);
        assert_eq!(claim.node_id, "test-node");
        assert_eq!(claim.capability_type, "inference");
        assert_eq!(claim.status, "unverified");
        assert!(bridge.persist().is_ok());

        // Reload
        let bridge2 = CapabilityEvidenceBridge::new(path);
        assert_eq!(bridge2.claims.len(), 1);
        assert_eq!(bridge2.claims[0].claim_id, claim.claim_id);
    }

    #[test]
    fn test_evidence_reference_creation_and_linking() {
        let mut bridge = test_bridge();
        let claim = bridge.register_claim("test-node", "inference", None, None);

        let reference = bridge.link_evidence(&claim.claim_id, "evt-001", "qr-001");
        assert!(reference.is_some());
        assert_eq!(reference.unwrap().claim_id, claim.claim_id);

        // Non-existent claim should return None
        let bad_ref = bridge.link_evidence("nonexistent", "evt-002", "qr-002");
        assert!(bad_ref.is_none());
    }

    #[test]
    fn test_verification_state_transitions() {
        let mut bridge = test_bridge();
        let claim = bridge.register_claim("test-node", "inference", None, None);
        assert_eq!(claim.status, "unverified");

        // Link evidence
        bridge.link_evidence(&claim.claim_id, "evt-001", "qr-001");

        // Verify with valid packet
        let packet = test_evidence_packet();
        let result = bridge.verify_claim(&claim.claim_id, &packet);
        assert!(result.is_ok());

        // Check state is now verified
        let state = bridge.get_verification_state("test-node");
        let vc = state.capabilities.iter().find(|c| c.claim_id == claim.claim_id);
        assert!(vc.is_some());
        assert_eq!(vc.unwrap().verification_status, "verified");
    }

    #[test]
    fn test_verification_state_transitions_failed() {
        let mut bridge = test_bridge();
        let claim = bridge.register_claim("test-node", "inference", None, None);
        assert_eq!(claim.status, "unverified");

        bridge.link_evidence(&claim.claim_id, "evt-001", "qr-001");

        // Verify with invalid packet
        let mut packet = test_evidence_packet();
        packet.packet_type = "invalid_type".to_string();
        let result = bridge.verify_claim(&claim.claim_id, &packet);
        assert!(result.is_err());

        let state = bridge.get_verification_state("test-node");
        let vc = state.capabilities.iter().find(|c| c.claim_id == claim.claim_id);
        assert!(vc.is_some());
        assert_eq!(vc.unwrap().verification_status, "failed");
    }

    #[test]
    fn test_get_unverified_claims() {
        let mut bridge = test_bridge();
        bridge.register_claim("test-node", "inference", None, None);
        bridge.register_claim("test-node", "hardware", None, None);

        assert_eq!(bridge.get_unverified_claims().len(), 2);

        // Verify one claim
        let claim = bridge.claims[0].clone();
        let packet = test_evidence_packet();
        bridge.link_evidence(&claim.claim_id, "evt-001", "qr-001");
        bridge.verify_claim(&claim.claim_id, &packet).unwrap();

        assert_eq!(bridge.get_unverified_claims().len(), 1);
    }

    #[test]
    fn test_get_verified_capabilities() {
        let mut bridge = test_bridge();
        let claim = bridge.register_claim("test-node", "inference", None, None);

        let packet = test_evidence_packet();
        bridge.link_evidence(&claim.claim_id, "evt-001", "qr-001");
        bridge.verify_claim(&claim.claim_id, &packet).unwrap();

        let verified = bridge.get_verified_capabilities();
        assert_eq!(verified.len(), 1);
        assert_eq!(verified[0].0.capability_type, "inference");
    }

    #[test]
    fn test_verification_state_returns_evidence() {
        let mut bridge = test_bridge();
        let claim = bridge.register_claim("test-node", "qualification", None, None);

        bridge.link_evidence(&claim.claim_id, "evt-001", "qr-001");
        let packet = test_evidence_packet();
        bridge.verify_claim(&claim.claim_id, &packet).unwrap();

        let state = bridge.get_verification_state("test-node");
        let vc = state.capabilities.iter().find(|c| c.claim_id == claim.claim_id);
        assert!(vc.is_some());
        assert_eq!(vc.unwrap().evidence_references.len(), 1);
        assert_eq!(vc.unwrap().evidence_references[0].evidence_packet_id, "evt-001");
    }

    #[test]
    fn test_persistence_survives_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_persist.json");
        {
            let mut bridge = CapabilityEvidenceBridge::new(path.clone());
            bridge.register_claim("test-node", "inference", None, None);
            bridge.persist().unwrap();
        }
        {
            let bridge = CapabilityEvidenceBridge::new(path.clone());
            assert_eq!(bridge.claims.len(), 1);
            assert_eq!(bridge.claims[0].capability_type, "inference");
        }
    }

    #[test]
    fn test_empty_bridge() {
        let bridge = test_bridge();
        assert!(bridge.claims.is_empty());
        assert!(bridge.evidence_refs.is_empty());
        let state = bridge.get_verification_state("test-node");
        assert!(state.capabilities.is_empty());
    }

    #[test]
    fn test_auto_transition_discovered_to_pending() {
        let mut bridge = test_bridge();
        bridge.register_claim("test-node", "inference", None, None);

        let state = bridge.get_current_state("inference");
        assert_eq!(state, CapabilityState::PendingVerification);

        let lifecycle = bridge.get_capability_lifecycle();
        let entry = lifecycle.iter().find(|e| e["capability_type"] == "inference");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap()["current_state"], "pending_verification");
        assert_eq!(entry.unwrap()["change_count"], 1);
    }

    #[test]
    fn test_auto_transition_pending_to_verified() {
        let mut bridge = test_bridge();
        let claim = bridge.register_claim("test-node", "inference", None, None);
        assert_eq!(bridge.get_current_state("inference"), CapabilityState::PendingVerification);

        bridge.link_evidence(&claim.claim_id, "evt-001", "qr-001");
        let packet = test_evidence_packet();
        bridge.verify_claim(&claim.claim_id, &packet).unwrap();

        let state = bridge.get_current_state("inference");
        assert_eq!(state, CapabilityState::Verified);

        let lifecycle = bridge.get_capability_lifecycle();
        let entry = lifecycle.iter().find(|e| e["capability_type"] == "inference").unwrap();
        assert_eq!(entry["current_state"], "verified");
        assert_eq!(entry["change_count"], 2);
    }

    #[test]
    fn test_explicit_transition_superseded() {
        let mut bridge = test_bridge();
        let claim = bridge.register_claim("test-node", "inference", None, None);
        bridge.link_evidence(&claim.claim_id, "evt-001", "qr-001");
        let packet = test_evidence_packet();
        bridge.verify_claim(&claim.claim_id, &packet).unwrap();

        let receipt = bridge.transition_state("inference", CapabilityState::Superseded, "New version available");
        assert!(receipt.is_ok());
        let receipt = receipt.unwrap();
        assert_eq!(receipt.previous_state, CapabilityState::Verified);
        assert_eq!(receipt.new_state, CapabilityState::Superseded);

        assert_eq!(bridge.get_current_state("inference"), CapabilityState::Superseded);
    }

    #[test]
    fn test_explicit_transition_active_to_degraded() {
        let mut bridge = test_bridge();
        let claim = bridge.register_claim("test-node", "runtime", None, None);
        bridge.link_evidence(&claim.claim_id, "evt-001", "qr-001");
        let packet = test_evidence_packet();
        bridge.verify_claim(&claim.claim_id, &packet).unwrap();

        bridge.transition_state("runtime", CapabilityState::Active, "Capability active").unwrap();
        let receipt = bridge.transition_state("runtime", CapabilityState::Degraded, "Performance degradation detected");
        assert!(receipt.is_ok());
        assert_eq!(bridge.get_current_state("runtime"), CapabilityState::Degraded);
    }

    #[test]
    fn test_invalid_transition_rejected() {
        let mut bridge = test_bridge();
        bridge.register_claim("test-node", "inference", None, None);

        // Discovered -> Active is invalid (must go through PendingVerification -> Verified -> Active)
        let result = bridge.transition_state("inference", CapabilityState::Active, "Jump ahead");
        assert!(result.is_err());

        // Should still be PendingVerification
        assert_eq!(bridge.get_current_state("inference"), CapabilityState::PendingVerification);
    }

    #[test]
    fn test_invalid_transition_from_retired() {
        let mut bridge = test_bridge();
        let claim = bridge.register_claim("test-node", "inference", None, None);
        bridge.link_evidence(&claim.claim_id, "evt-001", "qr-001");
        let packet = test_evidence_packet();
        bridge.verify_claim(&claim.claim_id, &packet).unwrap();

        bridge.transition_state("inference", CapabilityState::Retired, "End of life").unwrap();
        assert_eq!(bridge.get_current_state("inference"), CapabilityState::Retired);

        // Retired -> Active is invalid
        let result = bridge.transition_state("inference", CapabilityState::Active, "Restore");
        assert!(result.is_err());
    }

    #[test]
    fn test_lifecycle_includes_all_capability_types() {
        let mut bridge = test_bridge();
        bridge.register_claim("test-node", "llm.inference", None, None);
        bridge.register_claim("test-node", "hardware", None, None);
        bridge.register_claim("test-node", "runtime", None, None);

        let lifecycle = bridge.get_capability_lifecycle();
        let types: Vec<&str> = lifecycle.iter().map(|e| e["capability_type"].as_str().unwrap()).collect();
        assert!(types.contains(&"llm.inference"));
        assert!(types.contains(&"hardware"));
        assert!(types.contains(&"runtime"));
        assert_eq!(lifecycle.len(), 3);
    }

    #[test]
    fn test_verified_capability_includes_lifecycle_state() {
        let mut bridge = test_bridge();
        let claim = bridge.register_claim("test-node", "inference", None, None);
        bridge.link_evidence(&claim.claim_id, "evt-001", "qr-001");
        let packet = test_evidence_packet();
        bridge.verify_claim(&claim.claim_id, &packet).unwrap();

        let state = bridge.get_verification_state("test-node");
        let vc = state.capabilities.iter().find(|c| c.claim_id == claim.claim_id).unwrap();
        assert_eq!(vc.state, CapabilityState::Verified);
    }

    #[test]
    fn test_lifecycle_persistence_cross_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("lifecycle_test.json");
        {
            let mut bridge = CapabilityEvidenceBridge::new(path.clone());
            let claim = bridge.register_claim("test-node", "persistence-test", None, None);
            bridge.link_evidence(&claim.claim_id, "evt-001", "qr-001");
            let packet = test_evidence_packet();
            bridge.verify_claim(&claim.claim_id, &packet).unwrap();
            bridge.transition_state("persistence-test", CapabilityState::Active, "Now active").unwrap();
            bridge.persist().unwrap();
        }
        {
            let bridge = CapabilityEvidenceBridge::new(path.clone());
            assert_eq!(bridge.lifecycle_records.len(), 3);
            assert_eq!(bridge.get_current_state("persistence-test"), CapabilityState::Active);
        }
    }

    #[test]
    fn test_transition_receipt_has_correct_fields() {
        let mut bridge = test_bridge();
        let claim = bridge.register_claim("test-node", "test-cap", None, None);
        bridge.link_evidence(&claim.claim_id, "evt-001", "qr-001");
        let packet = test_evidence_packet();
        bridge.verify_claim(&claim.claim_id, &packet).unwrap();

        let receipt = bridge.transition_state("test-cap", CapabilityState::Superseded, "Upgraded").unwrap();
        assert!(!receipt.receipt_id.is_empty());
        assert_eq!(receipt.capability_type, "test-cap");
        assert_eq!(receipt.previous_state, CapabilityState::Verified);
        assert_eq!(receipt.new_state, CapabilityState::Superseded);
        assert_eq!(receipt.reason, "Upgraded");
        assert!(!receipt.changed_at.is_empty());
    }

    #[test]
    fn test_verified_capability_defaults_to_discovered() {
        let bridge = test_bridge();
        // No lifecycle records should mean Discovered
        assert_eq!(bridge.get_current_state("nonexistent"), CapabilityState::Discovered);
    }
}
