use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnomalyThreshold {
    pub metric_name: String,
    pub context_pattern: Option<String>,
    pub deviation_factor_threshold: f64,
    pub min_samples: u32,
    pub severity_map: Vec<SeverityLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeverityLevel {
    pub min_deviation_factor: f64,
    pub severity: String,
}
