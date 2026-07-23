use serde::{Deserialize, Serialize};

use super::difference::ClassifiedDifference;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationRequest {
    pub reconciliation_id: String,
    pub node_id: String,
    pub lkg_reference: String,
    pub initiated_at: String,
    pub initiated_by: String,
    pub phase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReconciliationReport {
    pub report_id: String,
    pub reconciliation_id: String,
    pub node_id: String,
    pub lkg_reference: String,
    pub custody_snapshot: String,
    pub differences: Vec<ClassifiedDifference>,
    pub total_differences: u32,
    pub generated_at: String,
    pub phase: String,
}
