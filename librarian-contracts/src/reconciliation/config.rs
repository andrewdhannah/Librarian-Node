use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationConfig {
    pub auto_reconcile_on_reconnect: bool,
    pub quarantine_on_integrity_failure: bool,
    pub max_differences_per_report: u32,
    pub version: String,
}

impl Default for ReconciliationConfig {
    fn default() -> Self {
        ReconciliationConfig {
            auto_reconcile_on_reconnect: true,
            quarantine_on_integrity_failure: true,
            max_differences_per_report: 1000,
            version: "1".to_string(),
        }
    }
}
