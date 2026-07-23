use crate::db::RuntimeDatabase;
use crate::node::{CapabilityEvidenceBridge, ModelRuntimeService};
use librarian_contracts::node::{Capability, CapabilityManifest, ModelDescriptor};

pub fn detect_capabilities(
    db: &RuntimeDatabase,
    node_id: &str,
    evidence_bridge: Option<&CapabilityEvidenceBridge>,
    model_runtime: Option<&ModelRuntimeService>,
) -> CapabilityManifest {
    // Capabilities are marked in the manifest as captured from the bridge state.
    // Enforcement (degrade if not registered) is handled by the enforcement service
    // calling bridge.degrade_if_not_registered() before manifest generation.
    let models = db.list_local_models().ok();
    let has_models = models.as_ref().map(|m| !m.is_empty()).unwrap_or(false);

    let model_descriptors: Option<Vec<ModelDescriptor>> = models.map(|ms| {
        ms.iter()
            .map(|m| ModelDescriptor {
                model_id: m.model_id.clone(),
                quantization: m.quantization.clone(),
                family: m.family.clone(),
            })
            .collect()
    });

    let evidence_state = evidence_bridge.map(|b| b.get_verification_state(node_id));
    let lookup_evidence = |cap_type: &str| -> (Option<String>, Option<u32>) {
        if let Some(ref state) = evidence_state {
            if let Some(vc) = state.capabilities.iter().find(|c| c.capability_type == cap_type) {
                return (
                    Some(vc.verification_status.clone()),
                    Some(vc.evidence_references.len() as u32),
                );
            }
        }
        (None, None)
    };

    let mut capabilities = Vec::new();

    let (inf_ver, inf_cnt) = lookup_evidence("llm.inference");
    capabilities.push(Capability {
        capability_type: "llm.inference".to_string(),
        runtime: Some("llama.cpp".to_string()),
        models: model_descriptors,
        available: has_models,
        verification_status: inf_ver,
        evidence_count: inf_cnt,
        runtime_qualification_status: None,
    });

    let (hw_ver, hw_cnt) = lookup_evidence("hardware");
    capabilities.push(Capability {
        capability_type: "hardware".to_string(),
        runtime: None,
        models: None,
        available: true,
        verification_status: hw_ver,
        evidence_count: hw_cnt,
        runtime_qualification_status: None,
    });

    let (rt_ver, rt_cnt) = lookup_evidence("runtime");
    let runtime_qual_status = model_runtime.map(|mr| {
        let profiles = mr.get_runtime_profiles();
        let qualified = profiles.iter().filter(|p| p.qualification_status == "qualified").count();
        let total = profiles.len();
        format!("{}/{} qualified", qualified, total)
    });
    capabilities.push(Capability {
        capability_type: "runtime".to_string(),
        runtime: Some(env!("CARGO_PKG_VERSION").to_string()),
        models: None,
        available: true,
        verification_status: rt_ver,
        evidence_count: rt_cnt,
        runtime_qualification_status: runtime_qual_status,
    });

    let (ql_ver, ql_cnt) = lookup_evidence("qualification");
    capabilities.push(Capability {
        capability_type: "qualification".to_string(),
        runtime: None,
        models: None,
        available: true,
        verification_status: ql_ver,
        evidence_count: ql_cnt,
        runtime_qualification_status: None,
    });

    let (eg_ver, eg_cnt) = lookup_evidence("evidence-generation");
    capabilities.push(Capability {
        capability_type: "evidence-generation".to_string(),
        runtime: None,
        models: None,
        available: true,
        verification_status: eg_ver,
        evidence_count: eg_cnt,
        runtime_qualification_status: None,
    });

    let (cc_ver, cc_cnt) = lookup_evidence("concurrency");
    capabilities.push(Capability {
        capability_type: "concurrency".to_string(),
        runtime: None,
        models: None,
        available: true,
        verification_status: cc_ver,
        evidence_count: cc_cnt,
        runtime_qualification_status: None,
    });

    CapabilityManifest {
        node_id: node_id.to_string(),
        capabilities,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::LocalModel;
    use tempfile::tempdir;

    fn test_db() -> RuntimeDatabase {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = RuntimeDatabase::open(path).unwrap();
        db.migrate().unwrap();
        Box::leak(Box::new(dir));
        db
    }

    fn test_bridge() -> CapabilityEvidenceBridge {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ev.json");
        CapabilityEvidenceBridge::new(path)
    }

    #[test]
    fn test_detect_capabilities_empty_db() {
        let db = test_db();
        let manifest = detect_capabilities(&db, "test-node", None, None);
        assert_eq!(manifest.node_id, "test-node");
        assert!(!manifest.capabilities.is_empty());
    }

    #[test]
    fn test_detect_capabilities_with_model() {
        let db = test_db();
        let model = LocalModel::new(
            "test-model".to_string(),
            "Test Model".to_string(),
            "test.gguf".to_string(),
        );
        db.insert_local_model(&model).unwrap();

        let manifest = detect_capabilities(&db, "test-node", None, None);
        let inference = manifest
            .capabilities
            .iter()
            .find(|c| c.capability_type == "llm.inference")
            .expect("should have inference capability");
        assert!(inference.available);
        assert!(inference.models.is_some());
    }

    #[test]
    fn test_manifest_serialization_roundtrip() {
        let db = test_db();
        let manifest = detect_capabilities(&db, "roundtrip-node", None, None);
        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: CapabilityManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.node_id, "roundtrip-node");
        assert_eq!(deserialized.capabilities.len(), manifest.capabilities.len());
    }

    #[test]
    fn test_capability_includes_verification_status() {
        let db = test_db();
        let mut bridge = test_bridge();

        // Register and verify an inference claim
        let claim = bridge.register_claim("test-node", "llm.inference", Some("llama.cpp".to_string()), None);
        let packet = make_test_packet();
        bridge.link_evidence(&claim.claim_id, "evt-001", "qr-001");
        bridge.verify_claim(&claim.claim_id, &packet).unwrap();

        let manifest = detect_capabilities(&db, "test-node", Some(&bridge), None);
        let inference = manifest
            .capabilities
            .iter()
            .find(|c| c.capability_type == "llm.inference")
            .expect("should have inference capability");

        assert_eq!(inference.verification_status, Some("verified".to_string()));
        assert_eq!(inference.evidence_count, Some(1));
    }

    #[test]
    fn test_capability_shows_unverified_when_no_evidence() {
        let db = test_db();
        let mut bridge = test_bridge();
        bridge.register_claim("test-node", "hardware", None, None);

        let manifest = detect_capabilities(&db, "test-node", Some(&bridge), None);
        let hw = manifest
            .capabilities
            .iter()
            .find(|c| c.capability_type == "hardware")
            .expect("should have hardware capability");

        assert_eq!(hw.verification_status, Some("unverified".to_string()));
        assert_eq!(hw.evidence_count, Some(0));
    }

    #[test]
    fn test_capability_shows_none_when_no_bridge() {
        let db = test_db();
        let manifest = detect_capabilities(&db, "test-node", None, None);
        let inference = manifest
            .capabilities
            .iter()
            .find(|c| c.capability_type == "llm.inference")
            .expect("should have inference capability");

        assert_eq!(inference.verification_status, None);
        assert_eq!(inference.evidence_count, None);
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
}
