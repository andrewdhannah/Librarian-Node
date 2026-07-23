use std::collections::HashMap;

use librarian_contracts::model_runtime::{ModelRuntimeEvidenceLink, ModelRuntimeProfile, RuntimeCapability};
use uuid::Uuid;

pub struct ModelRuntimeService {
    profiles: HashMap<String, ModelRuntimeProfile>,
    evidence_links: Vec<ModelRuntimeEvidenceLink>,
}

impl ModelRuntimeService {
    pub fn new() -> Self {
        ModelRuntimeService {
            profiles: HashMap::new(),
            evidence_links: Vec::new(),
        }
    }

    pub fn get_runtime_profiles(&self) -> Vec<RuntimeCapability> {
        let mut capabilities = Vec::new();
        for profile in self.profiles.values() {
            capabilities.extend(profile.runtime_capabilities.clone());
        }
        capabilities
    }

    pub fn get_model_runtime_profile(&self, model_id: &str) -> Option<ModelRuntimeProfile> {
        self.profiles.get(model_id).cloned()
    }

    pub fn link_evidence(
        &mut self,
        model_id: &str,
        runtime_type: &str,
        evidence_packet_id: &str,
        qualification_run_id: &str,
    ) -> ModelRuntimeEvidenceLink {
        let link = ModelRuntimeEvidenceLink {
            link_id: Uuid::new_v4().to_string(),
            model_id: model_id.to_string(),
            runtime_type: runtime_type.to_string(),
            evidence_packet_id: evidence_packet_id.to_string(),
            qualification_run_id: qualification_run_id.to_string(),
            linked_at: chrono::Utc::now().to_rfc3339(),
        };

        let profile = self
            .profiles
            .entry(model_id.to_string())
            .or_insert_with(|| ModelRuntimeProfile {
                model_id: model_id.to_string(),
                runtime_capabilities: Vec::new(),
                last_qualified_at: None,
                qualification_summary: None,
            });

        if let Some(cap) = profile
            .runtime_capabilities
            .iter_mut()
            .find(|c| c.runtime_type == runtime_type)
        {
            cap.evidence_packet_ids.push(evidence_packet_id.to_string());
            cap.qualification_status = "qualified".to_string();
        } else {
            profile.runtime_capabilities.push(RuntimeCapability {
                runtime_type: runtime_type.to_string(),
                runtime_version: "unknown".to_string(),
                backend: "llama.cpp".to_string(),
                hardware_requirements: Vec::new(),
                evidence_packet_ids: vec![evidence_packet_id.to_string()],
                qualification_status: "qualified".to_string(),
            });
        }

        profile.last_qualified_at = Some(chrono::Utc::now().to_rfc3339());
        profile.qualification_summary = Some(format!(
            "Linked evidence {} for runtime {}",
            evidence_packet_id, runtime_type
        ));

        self.evidence_links.push(link.clone());
        link
    }

    pub fn get_evidence_links(&self, model_id: &str) -> Vec<ModelRuntimeEvidenceLink> {
        self.evidence_links
            .iter()
            .filter(|l| l.model_id == model_id)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_runtime_profiles_empty() {
        let svc = ModelRuntimeService::new();
        assert!(svc.get_runtime_profiles().is_empty());
    }

    #[test]
    fn test_link_evidence_creates_profile() {
        let mut svc = ModelRuntimeService::new();
        let link = svc.link_evidence("model-a", "llama.cpp", "evt-001", "qr-001");
        assert_eq!(link.model_id, "model-a");
        assert_eq!(link.runtime_type, "llama.cpp");
        assert_eq!(link.evidence_packet_id, "evt-001");

        let profile = svc.get_model_runtime_profile("model-a").unwrap();
        assert_eq!(profile.model_id, "model-a");
        assert_eq!(profile.runtime_capabilities.len(), 1);
        assert_eq!(profile.runtime_capabilities[0].runtime_type, "llama.cpp");
        assert_eq!(profile.runtime_capabilities[0].qualification_status, "qualified");
    }

    #[test]
    fn test_link_evidence_appends_to_existing() {
        let mut svc = ModelRuntimeService::new();
        svc.link_evidence("model-a", "llama.cpp", "evt-001", "qr-001");
        svc.link_evidence("model-a", "llama.cpp", "evt-002", "qr-002");

        let profile = svc.get_model_runtime_profile("model-a").unwrap();
        assert_eq!(profile.runtime_capabilities.len(), 1);
        assert_eq!(profile.runtime_capabilities[0].evidence_packet_ids.len(), 2);
    }

    #[test]
    fn test_link_evidence_multiple_runtimes() {
        let mut svc = ModelRuntimeService::new();
        svc.link_evidence("model-a", "llama.cpp", "evt-001", "qr-001");
        svc.link_evidence("model-a", "whisper", "evt-002", "qr-002");

        let profile = svc.get_model_runtime_profile("model-a").unwrap();
        assert_eq!(profile.runtime_capabilities.len(), 2);
    }

    #[test]
    fn test_get_evidence_links() {
        let mut svc = ModelRuntimeService::new();
        svc.link_evidence("model-a", "llama.cpp", "evt-001", "qr-001");
        svc.link_evidence("model-a", "whisper", "evt-002", "qr-002");
        svc.link_evidence("model-b", "llama.cpp", "evt-003", "qr-003");

        let links = svc.get_evidence_links("model-a");
        assert_eq!(links.len(), 2);

        let links_b = svc.get_evidence_links("model-b");
        assert_eq!(links_b.len(), 1);
    }

    #[test]
    fn test_get_model_runtime_profile_nonexistent() {
        let svc = ModelRuntimeService::new();
        assert!(svc.get_model_runtime_profile("nonexistent").is_none());
    }

    #[test]
    fn test_runtime_profiles_aggregates_all() {
        let mut svc = ModelRuntimeService::new();
        svc.link_evidence("model-a", "llama.cpp", "evt-001", "qr-001");
        svc.link_evidence("model-b", "whisper", "evt-002", "qr-002");

        let caps = svc.get_runtime_profiles();
        assert_eq!(caps.len(), 2);
    }
}
