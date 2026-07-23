//! Capability replay and regression detection.
//!
//! Detects capability changes over time without automatically judging
//! whether the change is acceptable. Evidence of change is produced.
//! Authority remains with Owner review.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::models::CapabilityEvidence;

/// Comparison result for a single fixture across time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RegressionResult {
    NoChange,
    PassToFail,
    PassToDegraded,
    FailToPass,
    DegradedToPass,
    DegradedToFail,
    DifferentFailures,
    FixtureVersionChanged,
    Incomparable,
}

impl RegressionResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoChange => "no_change",
            Self::PassToFail => "pass_to_fail",
            Self::PassToDegraded => "pass_to_degraded",
            Self::FailToPass => "fail_to_pass",
            Self::DegradedToPass => "degraded_to_pass",
            Self::DegradedToFail => "degraded_to_fail",
            Self::DifferentFailures => "different_failures",
            Self::FixtureVersionChanged => "fixture_version_changed",
            Self::Incomparable => "incomparable",
        }
    }
}

/// Per-fixture replay comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FixtureComparison {
    pub fixture_id: String,
    pub previous_result: String,
    pub current_result: String,
    pub regression: RegressionResult,
    pub previous_version: String,
    pub current_version: String,
    pub observation: String,
}

/// Complete replay comparison between two time periods.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityReplay {
    pub previous_label: String,
    pub current_label: String,
    pub comparisons: Vec<FixtureComparison>,
    pub regressions: usize,
    pub improvements: usize,
    pub fixture_version_changes: usize,
    pub content_hash: String,
}

impl CapabilityReplay {
    pub fn compute_content_hash(&self) -> String {
        let mut h = Sha256::new();
        for c in &self.comparisons {
            h.update(c.fixture_id.as_bytes());
            h.update(c.regression.as_str().as_bytes());
        }
        format!("{:x}", h.finalize())
    }
}

/// Deterministic regression detector.
pub struct CapabilityRegressionDetector;

impl CapabilityRegressionDetector {
    /// Compare two sets of capability evidence from different time periods.
    pub fn compare(
        previous_label: &str,
        current_label: &str,
        previous: &[CapabilityEvidence],
        current: &[CapabilityEvidence],
    ) -> CapabilityReplay {
        let mut comparisons = Vec::new();
        let prev_by_fixture: std::collections::HashMap<_, _> = previous.iter().map(|e| (e.fixture_identity.fixture_id.clone(), e)).collect();
        let curr_by_fixture: std::collections::HashMap<_, _> = current.iter().map(|e| (e.fixture_identity.fixture_id.clone(), e)).collect();

        for (fid, prev_ev) in &prev_by_fixture {
            let prev_result = prev_ev.result.as_str();
            let curr_ev = curr_by_fixture.get(fid);

            let (curr_result, regression, obs) = match curr_ev {
                Some(ev) => {
                    let cr = ev.result.as_str();
                    let reg = Self::classify(prev_result, cr);
                    let o = Self::describe(prev_result, cr, &reg);
                    (cr.to_string(), reg, o)
                }
                None => ("not_tested".to_string(), RegressionResult::Incomparable, "Fixture not found in current evidence".to_string()),
            };

            comparisons.push(FixtureComparison {
                fixture_id: fid.clone(),
                previous_result: prev_result.to_string(),
                current_result: curr_result,
                regression,
                previous_version: prev_ev.fixture_identity.fixture_version.clone(),
                current_version: curr_ev.map(|e| e.fixture_identity.fixture_version.clone()).unwrap_or_default(),
                observation: obs,
            });
        }

        let regressions = comparisons.iter().filter(|c| matches!(c.regression, RegressionResult::PassToFail | RegressionResult::PassToDegraded | RegressionResult::DegradedToFail)).count();
        let improvements = comparisons.iter().filter(|c| matches!(c.regression, RegressionResult::FailToPass | RegressionResult::DegradedToPass)).count();
        let fv_changes = comparisons.iter().filter(|c| c.previous_version != c.current_version).count();

        let mut r = CapabilityReplay {
            previous_label: previous_label.to_string(),
            current_label: current_label.to_string(),
            comparisons,
            regressions, improvements,
            fixture_version_changes: fv_changes,
            content_hash: String::new(),
        };
        r.content_hash = r.compute_content_hash();
        r
    }

    fn classify(prev: &str, curr: &str) -> RegressionResult {
        match (prev, curr) {
            ("pass", "fail") => RegressionResult::PassToFail,
            ("pass", "degraded") => RegressionResult::PassToDegraded,
            ("fail", "pass") => RegressionResult::FailToPass,
            ("degraded", "pass") => RegressionResult::DegradedToPass,
            ("degraded", "fail") => RegressionResult::DegradedToFail,
            _ if prev != curr => RegressionResult::DifferentFailures,
            _ => RegressionResult::NoChange,
        }
    }

    fn describe(prev: &str, curr: &str, reg: &RegressionResult) -> String {
        match reg {
            RegressionResult::NoChange => format!("No change ({} -> {})", prev, curr),
            RegressionResult::PassToFail => format!("Regression: {} -> {}", prev, curr),
            RegressionResult::PassToDegraded => format!("Degradation: {} -> {}", prev, curr),
            RegressionResult::FailToPass => format!("Improvement: {} -> {}", prev, curr),
            RegressionResult::DegradedToPass => format!("Improvement: {} -> {}", prev, curr),
            RegressionResult::DegradedToFail => format!("Degradation: {} -> {}", prev, curr),
            _ => format!("Changed: {} -> {}", prev, curr),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::models::*;
    use super::super::models::CapabilityResult::*;

    fn ev(fid: &str, r: CapabilityResult, v: &str) -> CapabilityEvidence {
        CapabilityEvidence {
            evidence_id: format!("e-{}", fid),
            model_identity: ModelIdentity { model_id: "m1".into(), model_sha256: "s".into(), quantization: "Q4".into(), model_version: "1".into() },
            runtime_configuration: RuntimeConfig { model_sha256: "s".into(), quantization: "Q4".into(), runtime_build: "b".into(), hardware_lane: "RX".into(), fixture_version: v.into() },
            evaluator_identity: EvaluatorIdentity { evaluator_id: "ope".into(), evaluator_version: "1".into(), upstream_project: "MQR".into() },
            fixture_identity: FixtureIdentity { fixture_id: fid.into(), fixture_version: v.into() },
            execution_context: ExecutionContext { timestamp: "2026-01-01".into(), hardware_lane: "RX".into(), runtime_build: "b".into() },
            result: r, failures: vec![],
            provenance_reference: ProvenanceReference { lineage_hash: None, lifecycle_event_id: None, model_identity_hash: "s".into() },
            evidence_hash: String::new(),
        }
    }

    #[test] fn test_no_change() {
        let r = CapabilityRegressionDetector::compare("old", "new", &[ev("f1", Pass, "1")], &[ev("f1", Pass, "1")]);
        assert_eq!(r.comparisons[0].regression, RegressionResult::NoChange);
        assert_eq!(r.regressions, 0);
    }

    #[test] fn test_pass_to_fail() {
        let r = CapabilityRegressionDetector::compare("old", "new", &[ev("f1", Pass, "1")], &[ev("f1", Fail, "1")]);
        assert_eq!(r.comparisons[0].regression, RegressionResult::PassToFail);
        assert_eq!(r.regressions, 1);
    }

    #[test] fn test_fail_to_pass() {
        let r = CapabilityRegressionDetector::compare("old", "new", &[ev("f1", Fail, "1")], &[ev("f1", Pass, "1")]);
        assert_eq!(r.comparisons[0].regression, RegressionResult::FailToPass);
        assert_eq!(r.improvements, 1);
    }

    #[test] fn test_pass_to_degraded() {
        let r = CapabilityRegressionDetector::compare("old", "new", &[ev("f1", Pass, "1")], &[ev("f1", Degraded, "1")]);
        assert_eq!(r.comparisons[0].regression, RegressionResult::PassToDegraded);
        assert_eq!(r.regressions, 1);
    }

    #[test] fn test_deterministic() {
        let a = CapabilityRegressionDetector::compare("o", "n", &[ev("f1", Pass, "1")], &[ev("f1", Fail, "1")]);
        let b = CapabilityRegressionDetector::compare("o", "n", &[ev("f1", Pass, "1")], &[ev("f1", Fail, "1")]);
        assert_eq!(a.content_hash, b.content_hash);
    }

    #[test] fn test_missing_fixture() {
        let r = CapabilityRegressionDetector::compare("o", "n", &[ev("f1", Pass, "1")], &[]);
        assert_eq!(r.comparisons[0].regression, RegressionResult::Incomparable);
    }

    #[test] fn test_version_change() {
        let r = CapabilityRegressionDetector::compare("o", "n", &[ev("f1", Pass, "1")], &[ev("f1", Pass, "2")]);
        assert_eq!(r.fixture_version_changes, 1);
    }

    #[test] fn test_authority_boundary() {
        let r = CapabilityRegressionDetector::compare("o", "n", &[ev("f1", Pass, "1")], &[ev("f1", Fail, "1")]);
        let j = serde_json::to_value(&r).unwrap();
        assert!(j.get("approve").is_none());
        assert!(j.get("reject").is_none());
        assert!(j.get("recommendation").is_none());
        assert!(j.get("score").is_none());
    }

    #[test] fn test_multiple_fixtures() {
        let p = vec![ev("f1", Pass, "1"), ev("f2", Pass, "1"), ev("f3", Pass, "1")];
        let c = vec![ev("f1", Fail, "1"), ev("f2", Pass, "1"), ev("f3", Fail, "1")];
        let r = CapabilityRegressionDetector::compare("o", "n", &p, &c);
        assert_eq!(r.regressions, 2);
    }
}
