use std::path::PathBuf;

use librarian_contracts::policy::{PolicyChangeReceipt, PolicyConfig, PolicyEntry};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedState {
    policies: Vec<PolicyEntry>,
    receipts: Vec<PolicyChangeReceipt>,
    version: u32,
}

pub struct PolicyService {
    persistence_path: PathBuf,
    policies: Vec<PolicyEntry>,
    receipts: Vec<PolicyChangeReceipt>,
    version: u32,
}

impl PolicyService {
    pub fn new(persistence_path: PathBuf) -> Self {
        let (policies, receipts, version) = Self::load(&persistence_path);
        let mut svc = PolicyService {
            persistence_path,
            policies,
            receipts,
            version,
        };
        if svc.policies.is_empty() {
            svc.load_defaults();
        }
        svc
    }

    fn load(path: &PathBuf) -> (Vec<PolicyEntry>, Vec<PolicyChangeReceipt>, u32) {
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(path) {
                if let Ok(state) = serde_json::from_str::<PersistedState>(&data) {
                    return (state.policies, state.receipts, state.version);
                }
            }
        }
        (Vec::new(), Vec::new(), 1)
    }

    pub fn persist(&self) {
        if let Some(parent) = self.persistence_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let state = PersistedState {
            policies: self.policies.clone(),
            receipts: self.receipts.clone(),
            version: self.version,
        };
        if let Ok(data) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(&self.persistence_path, data);
        }
    }

    pub fn load_defaults(&mut self) {
        let now = chrono::Utc::now().to_rfc3339();
        let defaults: Vec<PolicyEntry> = vec![
            PolicyEntry {
                policy_id: Uuid::new_v4().to_string(),
                name: "anomaly.threshold.inference_latency".to_string(),
                scope: "node".to_string(),
                category: "anomaly".to_string(),
                value: serde_json::json!({"deviation_factor_threshold": 2.0, "min_samples": 10}),
                owner: "system".to_string(),
                effective_date: now.clone(),
                version: 1,
                receipt_id: None,
            },
            PolicyEntry {
                policy_id: Uuid::new_v4().to_string(),
                name: "anomaly.threshold.success_rate".to_string(),
                scope: "node".to_string(),
                category: "anomaly".to_string(),
                value: serde_json::json!({"deviation_factor_threshold": 2.0, "min_samples": 5}),
                owner: "system".to_string(),
                effective_date: now.clone(),
                version: 1,
                receipt_id: None,
            },
            PolicyEntry {
                policy_id: Uuid::new_v4().to_string(),
                name: "anomaly.threshold.duration".to_string(),
                scope: "node".to_string(),
                category: "anomaly".to_string(),
                value: serde_json::json!({"deviation_factor_threshold": 2.0, "min_samples": 10}),
                owner: "system".to_string(),
                effective_date: now.clone(),
                version: 1,
                receipt_id: None,
            },
            PolicyEntry {
                policy_id: Uuid::new_v4().to_string(),
                name: "pattern.min_findings".to_string(),
                scope: "node".to_string(),
                category: "pattern".to_string(),
                value: serde_json::json!(3),
                owner: "system".to_string(),
                effective_date: now.clone(),
                version: 1,
                receipt_id: None,
            },
            PolicyEntry {
                policy_id: Uuid::new_v4().to_string(),
                name: "pattern.time_window_hours".to_string(),
                scope: "node".to_string(),
                category: "pattern".to_string(),
                value: serde_json::json!(24),
                owner: "system".to_string(),
                effective_date: now.clone(),
                version: 1,
                receipt_id: None,
            },
            PolicyEntry {
                policy_id: Uuid::new_v4().to_string(),
                name: "pattern.expiration_days".to_string(),
                scope: "node".to_string(),
                category: "pattern".to_string(),
                value: serde_json::json!(7),
                owner: "system".to_string(),
                effective_date: now.clone(),
                version: 1,
                receipt_id: None,
            },
            PolicyEntry {
                policy_id: Uuid::new_v4().to_string(),
                name: "bootstrap.approval.medium_impact".to_string(),
                scope: "node".to_string(),
                category: "bootstrap".to_string(),
                value: serde_json::json!(true),
                owner: "system".to_string(),
                effective_date: now.clone(),
                version: 1,
                receipt_id: None,
            },
            PolicyEntry {
                policy_id: Uuid::new_v4().to_string(),
                name: "bootstrap.approval.high_impact".to_string(),
                scope: "node".to_string(),
                category: "bootstrap".to_string(),
                value: serde_json::json!(true),
                owner: "system".to_string(),
                effective_date: now.clone(),
                version: 1,
                receipt_id: None,
            },
            PolicyEntry {
                policy_id: Uuid::new_v4().to_string(),
                name: "capability.review_required".to_string(),
                scope: "node".to_string(),
                category: "capability".to_string(),
                value: serde_json::json!(true),
                owner: "system".to_string(),
                effective_date: now,
                version: 1,
                receipt_id: None,
            },
        ];
        self.policies = defaults;
        self.version = 1;
        self.persist();
    }

    pub fn get_policies(&self) -> PolicyConfig {
        PolicyConfig {
            policies: self.policies.clone(),
            version: self.version,
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn get_policy(&self, name: &str) -> Option<PolicyEntry> {
        self.policies.iter().find(|p| p.name == name).cloned()
    }

    pub fn update_policy(&mut self, name: &str, value: serde_json::Value, owner: &str) -> Option<PolicyChangeReceipt> {
        let idx = self.policies.iter().position(|p| p.name == name)?;
        let previous_value = self.policies[idx].value.clone();
        self.policies[idx].value = value.clone();
        self.policies[idx].owner = owner.to_string();
        self.policies[idx].effective_date = chrono::Utc::now().to_rfc3339();
        self.policies[idx].version += 1;
        self.version += 1;

        let receipt = PolicyChangeReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            policy_id: self.policies[idx].policy_id.clone(),
            previous_value,
            new_value: value,
            changed_by: owner.to_string(),
            changed_at: chrono::Utc::now().to_rfc3339(),
        };

        self.policies[idx].receipt_id = Some(receipt.receipt_id.clone());
        self.receipts.push(receipt.clone());
        self.persist();
        Some(receipt)
    }

    pub fn get_receipts(&self) -> Vec<PolicyChangeReceipt> {
        self.receipts.clone()
    }

    pub fn get_policy_value(&self, name: &str) -> Option<serde_json::Value> {
        self.policies.iter().find(|p| p.name == name).map(|p| p.value.clone())
    }
}
