use serde::{Deserialize, Serialize};

use super::finding::{ClassifiedFinding, FindingCategory};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FindingCatalog {
    pub categories: Vec<FindingCategory>,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FindingSummary {
    pub total_findings: u32,
    pub pending_review: u32,
    pub acknowledged: u32,
    pub by_severity: FindingSeverityCounts,
    pub by_category: Vec<FindingCategoryCount>,
    pub latest_findings: Vec<ClassifiedFinding>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FindingSeverityCounts {
    pub info: u32,
    pub notable: u32,
    pub warning: u32,
    pub critical: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FindingCategoryCount {
    pub category: String,
    pub count: u32,
}
