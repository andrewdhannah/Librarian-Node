pub mod comparison;
pub mod dashboard;
pub mod report;

pub use comparison::{InsightComparison, TrendReport};
pub use dashboard::{
    AllocationQualitySummary, CapabilityHealthSummary, InsightDashboard, RecentActivitySummary,
    WorkloadTrendSummary,
};
pub use report::{AnomalyFindingSummary, InsightReport, OwnerRecommendation};
