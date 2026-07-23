pub mod analyzer;
pub mod audit;
pub mod finding;
pub mod roster;

pub use analyzer::{compare_role, ComparisonInput, ComparisonResult, ComparisonThresholds, compute_comparison_hash};
pub use audit::{ComparisonAuditRecord, ArtifactReference, ThresholdSnapshot, ComparisonMethodology, ANALYZER_VERSION};
pub use finding::{ComparativeFinding, FindingSeverity, FindingType};
pub use roster::{evaluate_roster, RosterRecommendation, RosterPosition, SupersessionRecord, RejectionRecord, RetestTrigger};
