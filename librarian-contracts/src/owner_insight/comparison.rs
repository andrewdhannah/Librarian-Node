use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InsightComparison {
    pub metric_name: String,
    pub period_a_label: String,
    pub period_a_value: f64,
    pub period_b_label: String,
    pub period_b_value: f64,
    pub change_pct: f64,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrendReport {
    pub report_id: String,
    pub comparisons: Vec<InsightComparison>,
    pub generated_at: String,
}
