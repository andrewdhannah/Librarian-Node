use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BaselineRecord {
    pub baseline_id: String,
    pub metric_name: String,
    pub context: String,
    pub mean: f64,
    pub std_dev: f64,
    pub sample_count: u32,
    pub window_start: String,
    pub window_end: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BaselineStore {
    pub node_id: String,
    pub baselines: Vec<BaselineRecord>,
    pub updated_at: String,
}
