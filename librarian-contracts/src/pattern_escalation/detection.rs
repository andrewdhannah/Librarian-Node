use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatternThreshold {
    pub category: String,
    pub min_findings: u32,
    pub time_window_hours: u32,
    pub min_severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatternDetectionConfig {
    pub thresholds: Vec<PatternThreshold>,
    pub default_min_findings: u32,
    pub default_time_window_hours: u32,
    pub expiration_days: u32,
    pub version: String,
}
