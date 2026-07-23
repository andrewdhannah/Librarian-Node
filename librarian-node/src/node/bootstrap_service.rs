use std::path::PathBuf;
use std::sync::Arc;

use librarian_contracts::bootstrap::{
    BootstrapAssessment, BootstrapPlan, BootstrapReceipt, BootstrapRecommendation,
    HardwareSummary,
};
use librarian_contracts::custody::CustodyMetadata;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::bootstrap_actions;
use super::custody_service::CustodyService;
use super::identity_service::NodeIdentityService;
use super::policy_service::PolicyService;
use super::CapabilityEvidenceBridge;
use crate::platform::HardwareDetector;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    assessments: Vec<BootstrapAssessment>,
    plans: Vec<BootstrapPlan>,
    receipts: Vec<BootstrapReceipt>,
}

pub struct BootstrapService {
    persistence_path: PathBuf,
    identity_service: Arc<NodeIdentityService>,
    _capability_bridge: Arc<std::sync::Mutex<CapabilityEvidenceBridge>>,
    hardware_detector: Arc<dyn HardwareDetector>,
    assessments: Vec<BootstrapAssessment>,
    plans: Vec<BootstrapPlan>,
    receipts: Vec<BootstrapReceipt>,
    custody_service: Option<Arc<std::sync::Mutex<CustodyService>>>,
    policy_service: Option<Arc<Mutex<PolicyService>>>,
}

impl BootstrapService {
    pub fn new(
        persistence_path: impl Into<PathBuf>,
        identity_service: Arc<NodeIdentityService>,
    #[allow(dead_code)]
    capability_bridge: Arc<std::sync::Mutex<CapabilityEvidenceBridge>>,
        hardware_detector: Arc<dyn HardwareDetector>,
    ) -> Self {
        let persistence_path = persistence_path.into();
        let (assessments, plans, receipts) = if persistence_path.exists() {
            match std::fs::read_to_string(&persistence_path) {
                Ok(content) => match serde_json::from_str::<PersistedState>(&content) {
                    Ok(state) => (state.assessments, state.plans, state.receipts),
                    Err(_) => (Vec::new(), Vec::new(), Vec::new()),
                },
                Err(_) => (Vec::new(), Vec::new(), Vec::new()),
            }
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };

        BootstrapService {
            persistence_path,
            identity_service,
            _capability_bridge: capability_bridge,
            hardware_detector,
            assessments,
            plans,
            receipts,
            custody_service: None,
            policy_service: None,
        }
    }

    pub fn with_custody(mut self, custody: Arc<std::sync::Mutex<CustodyService>>) -> Self {
        self.custody_service = Some(custody);
        self
    }

    pub fn with_policy(mut self, ps: Arc<Mutex<PolicyService>>) -> Self {
        self.policy_service = Some(ps);
        self
    }

    fn requires_approval(&self, impact: &str) -> bool {
        let key = match impact {
            "medium" => "bootstrap.approval.medium_impact",
            "high" => "bootstrap.approval.high_impact",
            _ => return false,
        };
        if let Some(ref ps) = self.policy_service {
            if let Ok(svc) = ps.try_lock() {
                if let Some(val) = svc.get_policy_value(key) {
                    if let Some(b) = val.as_bool() {
                        return b;
                    }
                }
            }
        }
        // Fallback: high impact always requires approval by default
        impact == "high"
    }

    pub fn assess(&mut self, session_id: &str) -> BootstrapAssessment {
        let node_id = self.identity_service.get_identity().node_id.clone();
        let assessment_id = Uuid::new_v4().to_string();

        let hardware = self.scan_hardware();
        let runtime_status = bootstrap_actions::check_runtime_status();

        let mut recommendations = Vec::new();

        // Runtime installation recommendation — high impact, requires approval
        if !runtime_status.runtime_installed {
            recommendations.push(BootstrapRecommendation {
                recommendation_id: Uuid::new_v4().to_string(),
                category: "runtime".to_string(),
                priority: "required".to_string(),
                description: "llama.cpp runtime is not installed on this machine.".to_string(),
                action: "Download and install llama.cpp runtime binary.".to_string(),
                impact: "high".to_string(),
                owner_approval_required: self.requires_approval("high"),
            });
        } else {
            recommendations.push(BootstrapRecommendation {
                recommendation_id: Uuid::new_v4().to_string(),
                category: "runtime".to_string(),
                priority: "info".to_string(),
                description: format!("Runtime installed: {:?}", runtime_status.runtime_version),
                action: "No action needed. Runtime is available.".to_string(),
                impact: "low".to_string(),
                owner_approval_required: false,
            });
        }

        // Backend detection summary
        if let Some(ref backend) = runtime_status.backend_available {
            if backend == "vulkan" || backend == "cuda" {
                recommendations.push(BootstrapRecommendation {
                    recommendation_id: Uuid::new_v4().to_string(),
                    category: "configuration".to_string(),
                    priority: "info".to_string(),
                    description: format!("GPU backend detected: {}", backend),
                    action: format!("Using {} backend for GPU acceleration.", backend),
                    impact: "low".to_string(),
                    owner_approval_required: false,
                });
            }
        }

        // GPU hardware info
        if hardware.gpu_available {
            if let Some(ref model) = hardware.gpu_model {
                recommendations.push(BootstrapRecommendation {
                    recommendation_id: Uuid::new_v4().to_string(),
                    category: "hardware".to_string(),
                    priority: "info".to_string(),
                    description: format!("Detected GPU: {} with {} MB VRAM", model, hardware.gpu_vram_mb.unwrap_or(0)),
                    action: "GPU acceleration available.".to_string(),
                    impact: "low".to_string(),
                    owner_approval_required: false,
                });
            }

            // Vulkan driver install — high impact, requires approval
            if runtime_status.backend_available.as_deref() == Some("cpu") {
                recommendations.push(BootstrapRecommendation {
                    recommendation_id: Uuid::new_v4().to_string(),
                    category: "runtime".to_string(),
                    priority: "recommended".to_string(),
                    description: "GPU detected but Vulkan backend not available. GPU acceleration not in use.".to_string(),
                    action: "Install Vulkan drivers and runtime for GPU acceleration.".to_string(),
                    impact: "high".to_string(),
                    owner_approval_required: self.requires_approval("high"),
                });
            }
        }

        // Model sizing recommendations
        let model_recs = bootstrap_actions::recommend_model_sizes(hardware.gpu_vram_mb, hardware.ram_mb);
        recommendations.extend(model_recs);

        // Model configuration recommendations
        let config_recs = bootstrap_actions::recommend_model_config(&hardware);
        recommendations.extend(config_recs);

        let assessment = BootstrapAssessment {
            assessment_id,
            node_id: node_id.clone(),
            session_id: session_id.to_string(),
            assessed_at: chrono::Utc::now().to_rfc3339(),
            hardware,
            runtime_status,
            recommendations,
        };

        self.assessments.push(assessment.clone());
        self.persist();
        assessment
    }

    pub fn create_plan(
        &mut self,
        session_id: &str,
        assessment_id: &str,
        approved_recommendation_ids: &[String],
    ) -> Result<BootstrapPlan, String> {
        let assessment = self
            .assessments
            .iter()
            .find(|a| a.assessment_id == assessment_id)
            .ok_or_else(|| format!("Assessment {} not found", assessment_id))?;

        if assessment.session_id != session_id {
            return Err("Session ID does not match assessment".to_string());
        }

        let selected: Vec<BootstrapRecommendation> = assessment
            .recommendations
            .iter()
            .filter(|r| approved_recommendation_ids.contains(&r.recommendation_id))
            .cloned()
            .collect();

        if selected.is_empty() {
            return Err("No recommendations selected for plan".to_string());
        }

        let plan_id = Uuid::new_v4().to_string();
        let node_id = self.identity_service.get_identity().node_id.clone();

        // We allow plan creation without owner approval; owner_approved is set explicitly
        let plan = BootstrapPlan {
            plan_id: plan_id.clone(),
            node_id,
            session_id: session_id.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            status: "draft".to_string(),
            recommendations: selected,
            owner_approved: false,
            approved_at: None,
        };

        self.plans.push(plan.clone());
        self.persist();
        Ok(plan)
    }

    pub fn execute_plan(&mut self, plan_id: &str) -> Result<BootstrapReceipt, String> {
        let plan_idx = self
            .plans
            .iter()
            .position(|p| p.plan_id == plan_id)
            .ok_or_else(|| format!("Plan {} not found", plan_id))?;

        let plan = &self.plans[plan_idx];

        if plan.status != "draft" && plan.status != "approved" {
            return Err(format!(
                "Plan {} cannot be executed from status '{}'",
                plan_id, plan.status
            ));
        }

        // Check that high-impact recommendations have owner approval
        let high_impact_unapproved = plan
            .recommendations
            .iter()
            .filter(|r| r.impact == "high")
            .any(|r| r.owner_approval_required && !plan.owner_approved);

        if high_impact_unapproved {
            return Err(
                "Plan contains high-impact recommendations that require owner approval".to_string(),
            );
        }

        let session_id = plan.session_id.clone();
        let node_id = plan.node_id.clone();

        // Generate evidence for each action
        let mut evidence_ids = Vec::new();
        let mut actions_taken = 0u32;
        let mut actions_skipped = 0u32;

        for recommendation in &plan.recommendations {
            let evidence_id = Uuid::new_v4().to_string();
            evidence_ids.push(evidence_id.clone());

            if recommendation.owner_approval_required && !plan.owner_approved {
                actions_skipped += 1;
                continue;
            }

            actions_taken += 1;
        }

        // Update the plan status
        self.plans[plan_idx].status = "completed".to_string();

        let result = if actions_skipped > 0 {
            "partial"
        } else if actions_taken > 0 {
            "completed"
        } else {
            "failed"
        };

        let receipt = BootstrapReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            plan_id: plan_id.to_string(),
            node_id,
            session_id,
            completed_at: chrono::Utc::now().to_rfc3339(),
            actions_taken,
            actions_skipped,
            evidence_ids,
            result: result.to_string(),
        };

        self.receipts.push(receipt.clone());
        self.persist();

        if let Some(ref custody) = self.custody_service {
            let node_id = receipt.node_id.clone();
            let payload = serde_json::to_value(&receipt).unwrap_or_default();
            let metadata = CustodyMetadata {
                source: "node".to_string(),
                version: "1".to_string(),
                notes: Some("Auto-custodied on bootstrap plan execution".to_string()),
            };
            let mut guard = custody.lock().unwrap();
            guard.append_receipt(
                &node_id,
                "bootstrap",
                &receipt.receipt_id,
                payload,
                Some(metadata),
            );
        }

        Ok(receipt)
    }

    pub fn get_assessment(&self, assessment_id: &str) -> Option<BootstrapAssessment> {
        self.assessments
            .iter()
            .find(|a| a.assessment_id == assessment_id)
            .cloned()
    }

    pub fn get_plan(&self, plan_id: &str) -> Option<BootstrapPlan> {
        self.plans
            .iter()
            .find(|p| p.plan_id == plan_id)
            .cloned()
    }

    pub fn get_receipts(&self) -> &[BootstrapReceipt] {
        &self.receipts
    }

    pub fn get_plans(&self) -> &[BootstrapPlan] {
        &self.plans
    }

    pub fn get_plans_mut(&mut self) -> &mut Vec<BootstrapPlan> {
        &mut self.plans
    }

    pub fn approve_plan(&mut self, plan_id: &str) -> Result<BootstrapPlan, String> {
        let plan = self
            .plans
            .iter_mut()
            .find(|p| p.plan_id == plan_id)
            .ok_or_else(|| format!("Plan {} not found", plan_id))?;

        if plan.status != "draft" {
            return Err(format!(
                "Plan {} cannot be approved from status '{}'",
                plan_id, plan.status
            ));
        }

        plan.owner_approved = true;
        plan.approved_at = Some(chrono::Utc::now().to_rfc3339());
        plan.status = "approved".to_string();

        let cloned = plan.clone();
        self.persist();
        Ok(cloned)
    }

    fn scan_hardware(&self) -> HardwareSummary {
        let gpu_model = self.hardware_detector.detect_gpu_model();
        let gpu_vram_mb = self.hardware_detector.detect_gpu_vram_mb();
        let gpu_available = gpu_model.is_some();

        let ram_mb = self
            .hardware_detector
            .detect_total_ram_mb()
            .unwrap_or(8192);

        let cpu_cores = self
            .hardware_detector
            .detect_cpu_cores()
            .unwrap_or(4);

        // Approximate disk space in the current directory
        let disk_space_mb = get_available_disk_space_mb();

        HardwareSummary {
            gpu_available,
            gpu_model,
            gpu_vram_mb,
            ram_mb,
            cpu_cores,
            disk_space_mb,
        }
    }

    fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState {
            assessments: self.assessments.clone(),
            plans: self.plans.clone(),
            receipts: self.receipts.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.persistence_path, json);
        }
    }
}

fn get_available_disk_space_mb() -> Option<u64> {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("wmic")
            .arg("LOGICALDISK")
            .arg("WHERE")
            .arg("DriveType=3")
            .arg("GET")
            .arg("FreeSpace")
            .output()
            .ok()?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    if let Ok(bytes) = trimmed.parse::<u64>() {
                        return Some(bytes / (1024 * 1024));
                    }
                }
            }
        }
        None
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::CapabilityEvidenceBridge;
    use crate::node::NodeIdentityService;
    use tempfile::tempdir;

    fn test_service() -> BootstrapService {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bootstrap.json");

        let identity_path = dir.path().join("identity.json");
        let identity_service = Arc::new(NodeIdentityService::new(identity_path));

        let bridge_path = dir.path().join("bridge.json");
        let bridge = Arc::new(std::sync::Mutex::new(CapabilityEvidenceBridge::new(bridge_path)));

        let detector = Arc::new(TestHardwareDetector);

        BootstrapService::new(path, identity_service, bridge, detector)
    }

    struct TestHardwareDetector;

    impl HardwareDetector for TestHardwareDetector {
        fn detect_gpu_vendor(&self) -> Option<String> { Some("AMD".to_string()) }
        fn detect_gpu_model(&self) -> Option<String> { Some("AMD Radeon RX 570".to_string()) }
        fn detect_gpu_vram_mb(&self) -> Option<u64> { Some(4096) }
        fn detect_total_ram_mb(&self) -> Option<u64> { Some(16384) }
        fn detect_cpu_model(&self) -> Option<String> { Some("Test CPU".to_string()) }
        fn detect_cpu_cores(&self) -> Option<u32> { Some(8) }
        fn platform_name(&self) -> String { "test".to_string() }
    }

    #[test]
    fn test_assessment_creation_with_hardware_scan() {
        let mut service = test_service();
        let assessment = service.assess("session-001");

        assert_eq!(assessment.session_id, "session-001");
        assert!(assessment.hardware.gpu_available);
        assert_eq!(assessment.hardware.gpu_model, Some("AMD Radeon RX 570".to_string()));
        assert_eq!(assessment.hardware.gpu_vram_mb, Some(4096));
        assert_eq!(assessment.hardware.ram_mb, 16384);
        assert_eq!(assessment.hardware.cpu_cores, 8);
        assert!(!assessment.recommendations.is_empty());
    }

    #[test]
    fn test_plan_creation_from_assessment() {
        let mut service = test_service();
        let assessment = service.assess("session-001");
        let rec_ids: Vec<String> = assessment
            .recommendations
            .iter()
            .take(2)
            .map(|r| r.recommendation_id.clone())
            .collect();

        let plan = service
            .create_plan("session-001", &assessment.assessment_id, &rec_ids)
            .unwrap();

        assert_eq!(plan.session_id, "session-001");
        assert!(!plan.owner_approved);
        assert_eq!(plan.status, "draft");
        assert_eq!(plan.recommendations.len(), rec_ids.len());
    }

    #[test]
    fn test_plan_creation_fails_for_wrong_session() {
        let mut service = test_service();
        let assessment = service.assess("session-001");
        let rec_ids: Vec<String> = assessment
            .recommendations
            .iter()
            .take(1)
            .map(|r| r.recommendation_id.clone())
            .collect();

        let result = service.create_plan("session-999", &assessment.assessment_id, &rec_ids);
        assert!(result.is_err());
    }

    #[test]
    fn test_plan_creation_fails_with_no_recommendations() {
        let mut service = test_service();
        let assessment = service.assess("session-001");
        let result = service.create_plan("session-001", &assessment.assessment_id, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_plan_produces_receipt() {
        let mut service = test_service();
        let assessment = service.assess("session-001");
        let rec_ids: Vec<String> = assessment
            .recommendations
            .iter()
            .map(|r| r.recommendation_id.clone())
            .collect();

        let mut plan = service
            .create_plan("session-001", &assessment.assessment_id, &rec_ids)
            .unwrap();

        // Approve the plan first (high-impact items need approval)
        plan = service.approve_plan(&plan.plan_id).unwrap();

        let receipt = service.execute_plan(&plan.plan_id).unwrap();
        assert_eq!(receipt.plan_id, plan.plan_id);
        assert!(!receipt.evidence_ids.is_empty());
        assert_eq!(receipt.result, "completed");
    }

    #[test]
    fn test_execute_plan_fails_without_approval_for_high_impact() {
        let mut service = test_service();
        let assessment = service.assess("session-001");
        let rec_ids: Vec<String> = assessment
            .recommendations
            .iter()
            .map(|r| r.recommendation_id.clone())
            .collect();

        let plan = service
            .create_plan("session-001", &assessment.assessment_id, &rec_ids)
            .unwrap();

        let result = service.execute_plan(&plan.plan_id);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("owner approval"));
    }

    #[test]
    fn test_get_assessment_returns_none_for_missing() {
        let service = test_service();
        assert!(service.get_assessment("nonexistent").is_none());
    }

    #[test]
    fn test_get_plan_returns_none_for_missing() {
        let service = test_service();
        assert!(service.get_plan("nonexistent").is_none());
    }

    #[test]
    fn test_owner_approval_flag_set_correctly() {
        let mut service = test_service();
        let assessment = service.assess("session-001");

        for rec in &assessment.recommendations {
            match rec.impact.as_str() {
                "high" => assert!(rec.owner_approval_required, "High impact '{}' must require approval", rec.category),
                _ => {} // medium/low/info may or may not require approval
            }
        }
    }

    #[test]
    fn test_assessment_has_unique_id() {
        let mut service = test_service();
        let a1 = service.assess("s-1");
        let a2 = service.assess("s-2");
        assert_ne!(a1.assessment_id, a2.assessment_id);
    }

    #[test]
    fn test_approve_plan_transitions_to_approved() {
        let mut service = test_service();
        let assessment = service.assess("session-001");
        let rec_ids: Vec<String> = assessment
            .recommendations
            .iter()
            .take(1)
            .map(|r| r.recommendation_id.clone())
            .collect();

        let plan = service
            .create_plan("session-001", &assessment.assessment_id, &rec_ids)
            .unwrap();

        let approved = service.approve_plan(&plan.plan_id).unwrap();
        assert!(approved.owner_approved);
        assert_eq!(approved.status, "approved");
        assert!(approved.approved_at.is_some());
    }

    #[test]
    fn test_double_approve_fails() {
        let mut service = test_service();
        let assessment = service.assess("session-001");
        let rec_ids: Vec<String> = assessment
            .recommendations
            .iter()
            .take(1)
            .map(|r| r.recommendation_id.clone())
            .collect();

        let plan = service
            .create_plan("session-001", &assessment.assessment_id, &rec_ids)
            .unwrap();

        service.approve_plan(&plan.plan_id).unwrap();
        let result = service.approve_plan(&plan.plan_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_plan_fails_for_nonexistent() {
        let mut service = test_service();
        let result = service.execute_plan("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_persistence_survives_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bootstrap_persist.json");
        let identity_path = dir.path().join("identity.json");
        let bridge_path = dir.path().join("bridge.json");

        let assessment_id;
        {
            let identity_service = Arc::new(NodeIdentityService::new(&identity_path));
            let bridge = Arc::new(std::sync::Mutex::new(CapabilityEvidenceBridge::new(bridge_path.clone())));
            let detector = Arc::new(TestHardwareDetector);
            let mut service = BootstrapService::new(
                path.clone(),
                identity_service,
                bridge,
                detector,
            );
            let assessment = service.assess("session-001");
            assessment_id = assessment.assessment_id.clone();
        }

        {
            let identity_service = Arc::new(NodeIdentityService::new(&identity_path));
            let bridge = Arc::new(std::sync::Mutex::new(CapabilityEvidenceBridge::new(bridge_path)));
            let detector = Arc::new(TestHardwareDetector);
            let service = BootstrapService::new(
                path.clone(),
                identity_service,
                bridge,
                detector,
            );
            let loaded = service.get_assessment(&assessment_id);
            assert!(loaded.is_some());
            assert_eq!(loaded.unwrap().assessment_id, assessment_id);
        }
    }
}
