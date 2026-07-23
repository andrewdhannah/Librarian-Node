//! # Model Profile Mapping
//!
//! Maps existing model profile data to `Capability` contract types.
//! Model profiles are converted from their platform-specific JSON format
//! into governed capability declarations.

use anyhow::Result;
use librarian_contracts::prelude::*;
use serde::{Deserialize, Serialize};

/// A model profile as stored in the platform-specific config.
/// Mirrors the structure of `config/model-profiles.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfileConfig {
    pub alias: String,
    pub model_file: String,
    pub gguf_size_gb: f64,
    pub backend: String,
    pub ngl: u32,
    pub context: u32,
    pub port: u16,
    pub task_classes: Vec<String>,
    pub verified_status: String,
    pub stability: String,
    pub requires_reduced_offload: bool,
    pub authority_status: String,
    pub limitations: String,
    pub known_behavior: String,
}

impl ModelProfileConfig {
    /// Map this model profile to a `Capability` declaration.
    /// No new CapabilityCategory — uses existing variants.
    pub fn to_capability(&self) -> Capability {
        let category = if self.task_classes.iter().any(|t| t.contains("code")) {
            CapabilityCategory::ModelExecution
        } else {
            CapabilityCategory::ModelExecution
        };

        Capability {
            capability_id: format!("model-{}", self.alias),
            name: format!("Model: {}", self.alias),
            description: format!(
                "{} — {} GB, {} context, ngl={}, backend: {}",
                self.model_file, self.gguf_size_gb, self.context, self.ngl, self.backend
            ),
            category,
            requires_authorization: true,
            enabled: self.verified_status == "verified",
            schema_version: CAPABILITY_CONTRACT_VERSION.into(),
        }
    }

    /// Map verification status to an evidence category.
    pub fn evidence_category(&self) -> EvidenceCategory {
        match self.verified_status.as_str() {
            "verified" => EvidenceCategory::ContractValidation,
            _ => EvidenceCategory::Manual,
        }
    }
}

/// A model profile mapped to governance types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernedModelProfile {
    /// The original profile alias.
    pub alias: String,
    /// The mapped capability declaration.
    pub capability: Capability,
    /// Task classes as capability identifiers.
    pub task_capabilities: Vec<String>,
    /// Hardware requirements.
    pub hardware_requirements: ModelHardwareRequirements,
}

/// Hardware requirements for a model profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHardwareRequirements {
    pub gguf_size_gb: f64,
    pub required_ngl: u32,
    pub required_context: u32,
    pub requires_reduced_offload: bool,
    pub backend: String,
}

/// Convert a collection of model profiles to governed capabilities.
pub fn profiles_to_capabilities(profiles: &[ModelProfileConfig]) -> Vec<GovernedModelProfile> {
    profiles
        .iter()
        .map(|p| {
            let capability = p.to_capability();
            GovernedModelProfile {
                alias: p.alias.clone(),
                capability,
                task_capabilities: p.task_classes.clone(),
                hardware_requirements: ModelHardwareRequirements {
                    gguf_size_gb: p.gguf_size_gb,
                    required_ngl: p.ngl,
                    required_context: p.context,
                    requires_reduced_offload: p.requires_reduced_offload,
                    backend: p.backend.clone(),
                },
            }
        })
        .collect()
}

/// Load model profiles from a JSON config file path.
pub fn load_profiles_from_path(path: &str) -> Result<Vec<ModelProfileConfig>> {
    let content = std::fs::read_to_string(path)?;
    let wrapper: ProfileWrapper = serde_json::from_str(&content)?;
    Ok(wrapper.profiles)
}

/// Top-level wrapper for the model-profiles.json format.
#[derive(Debug, Deserialize)]
struct ProfileWrapper {
    profiles: Vec<ModelProfileConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_profile() -> ModelProfileConfig {
        ModelProfileConfig {
            alias: "phi-4".into(),
            model_file: "microsoft_Phi-4-mini-instruct-Q4_K_M.gguf".into(),
            gguf_size_gb: 2.32,
            backend: "vulkan".into(),
            ngl: 99,
            context: 4096,
            port: 9120,
            task_classes: vec!["general_advisory".into(), "summarization_advisory".into()],
            verified_status: "verified".into(),
            stability: "stable".into(),
            requires_reduced_offload: false,
            authority_status: "advisory_only".into(),
            limitations: "Safe up to 4096 context".into(),
            known_behavior: "Clean output".into(),
        }
    }

    #[test]
    fn test_profile_to_capability() {
        let profile = sample_profile();
        let capability = profile.to_capability();
        assert_eq!(capability.capability_id, "model-phi-4");
        assert!(capability.enabled);
        assert_eq!(capability.category, CapabilityCategory::ModelExecution);
        assert!(capability.description.contains("2.32 GB"));
    }

    #[test]
    fn test_profiles_to_capabilities() {
        let profiles = vec![sample_profile()];
        let governed = profiles_to_capabilities(&profiles);
        assert_eq!(governed.len(), 1);
        assert_eq!(governed[0].alias, "phi-4");
        assert!(governed[0].capability.enabled);
    }

    #[test]
    fn test_evidence_category() {
        let verified = sample_profile();
        assert_eq!(verified.evidence_category(), EvidenceCategory::ContractValidation);

        let mut unverified = sample_profile();
        unverified.verified_status = "unverified".into();
        assert_eq!(unverified.evidence_category(), EvidenceCategory::Manual);
    }

    #[test]
    fn test_hardware_requirements() {
        let profile = sample_profile();
        let governed = profiles_to_capabilities(&[profile]);
        let hw = &governed[0].hardware_requirements;
        assert_eq!(hw.required_ngl, 99);
        assert_eq!(hw.required_context, 4096);
        assert!(!hw.requires_reduced_offload);
    }
}
