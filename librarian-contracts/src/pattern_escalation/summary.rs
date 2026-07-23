use serde::{Deserialize, Serialize};

use super::pattern::PatternFinding;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatternSummary {
    pub total_patterns: u32,
    pub active_patterns: u32,
    pub pending_review: u32,
    pub acknowledged: u32,
    pub monitoring: u32,
    pub by_severity: PatternSeverityCounts,
    pub by_category: Vec<PatternCategoryCount>,
    pub latest_patterns: Vec<PatternFinding>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatternSeverityCounts {
    pub info: u32,
    pub notable: u32,
    pub warning: u32,
    pub critical: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatternCategoryCount {
    pub category: String,
    pub count: u32,
}
