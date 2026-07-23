use serde::{Deserialize, Serialize};

/// A single governed policy entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEntry {
    pub policy_id: String,
    pub name: String,
    pub scope: String,
    pub category: String,
    pub value: serde_json::Value,
    pub owner: String,
    pub effective_date: String,
    pub version: u32,
    pub receipt_id: Option<String>,
}

/// The full set of current policies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub policies: Vec<PolicyEntry>,
    pub version: u32,
    pub updated_at: String,
}

/// A receipt produced whenever a policy value changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyChangeReceipt {
    pub receipt_id: String,
    pub policy_id: String,
    pub previous_value: serde_json::Value,
    pub new_value: serde_json::Value,
    pub changed_by: String,
    pub changed_at: String,
}
