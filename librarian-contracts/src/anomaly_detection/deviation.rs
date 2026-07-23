use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviationObservation {
    pub observation_id: String,
    pub metric_name: String,
    pub context: String,
    pub baseline_mean: f64,
    pub baseline_std_dev: f64,
    pub observed_value: f64,
    pub deviation_factor: f64,
    pub direction: String,
    pub observed_at: String,
    pub evidence_workload_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnomalyFinding {
    pub anomaly_id: String,
    pub observation: DeviationObservation,
    pub threshold_exceeded: f64,
    pub severity: String,
    pub generated_at: String,
}
