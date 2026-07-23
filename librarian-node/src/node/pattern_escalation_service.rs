use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use librarian_contracts::evidence_classification::ClassifiedFinding;
use librarian_contracts::pattern_escalation::{
    PatternCategoryCount, PatternDetectionConfig, PatternFinding, PatternProvenance,
    PatternReviewReceipt, PatternSeverityCounts, PatternSummary,
    PatternThreshold,
};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::anomaly_detection_service::AnomalyDetectionService;
use super::evidence_classification_service::EvidenceClassificationService;
use super::policy_service::PolicyService;

fn default_config() -> PatternDetectionConfig {
    PatternDetectionConfig {
        thresholds: vec![
            PatternThreshold {
                category: "performance_degradation".to_string(),
                min_findings: 3,
                time_window_hours: 24,
                min_severity: "notable".to_string(),
            },
            PatternThreshold {
                category: "capability_mismatch".to_string(),
                min_findings: 2,
                time_window_hours: 24,
                min_severity: "notable".to_string(),
            },
            PatternThreshold {
                category: "repeated_failure".to_string(),
                min_findings: 3,
                time_window_hours: 24,
                min_severity: "notable".to_string(),
            },
            PatternThreshold {
                category: "allocation_drift".to_string(),
                min_findings: 3,
                time_window_hours: 24,
                min_severity: "notable".to_string(),
            },
            PatternThreshold {
                category: "node_instability".to_string(),
                min_findings: 2,
                time_window_hours: 12,
                min_severity: "warning".to_string(),
            },
        ],
        default_min_findings: 3,
        default_time_window_hours: 24,
        expiration_days: 7,
        version: "1.0.0".to_string(),
    }
}

fn merge_severity(a: &str, b: &str) -> String {
    let order: [&str; 4] = ["info", "notable", "warning", "critical"];
    let a_idx = order.iter().position(|&s| s == a).unwrap_or(0);
    let b_idx = order.iter().position(|&s| s == b).unwrap_or(0);
    order[a_idx.max(b_idx)].to_string()
}

fn merge_confidence(confidences: &[String]) -> String {
    let order: [&str; 3] = ["low", "medium", "high"];
    let mut best = 0usize;
    for c in confidences {
        if let Some(idx) = order.iter().position(|&s| s == c) {
            if idx > best {
                best = idx;
            }
        }
    }
    order[best].to_string()
}

fn status_from_review_status(review_status: &str) -> String {
    match review_status {
        "pending" => "detected".to_string(),
        "acknowledged" => "acknowledged".to_string(),
        "resolved" => "resolved".to_string(),
        "dismissed" => "dismissed".to_string(),
        _ => "detected".to_string(),
    }
}

pub struct PatternEscalationService {
    patterns: HashMap<String, PatternFinding>,
    receipts: Vec<PatternReviewReceipt>,
    config: PatternDetectionConfig,
    persistence_path: PathBuf,
    policy_service: Option<Arc<Mutex<PolicyService>>>,
}

impl PatternEscalationService {
    pub fn new(persistence_path: PathBuf) -> Self {
        let config = Self::load_config(&persistence_path);
        let patterns = Self::load_patterns(&persistence_path);
        let receipts = Self::load_receipts(&persistence_path);
        PatternEscalationService {
            patterns,
            receipts,
            config,
            persistence_path,
            policy_service: None,
        }
    }

    pub fn with_policy(mut self, ps: Arc<Mutex<PolicyService>>) -> Self {
        self.policy_service = Some(ps);
        self
    }

    // --- Persistence ---

    fn load_patterns(path: &PathBuf) -> HashMap<String, PatternFinding> {
        let p = path
            .parent()
            .map(|parent| parent.join("patterns.json"))
            .unwrap_or_else(|| PathBuf::from("data/patterns.json"));
        if let Ok(data) = std::fs::read_to_string(&p) {
            if let Ok(found) = serde_json::from_str::<Vec<PatternFinding>>(&data) {
                return found.into_iter().map(|f| (f.pattern_id.clone(), f)).collect();
            }
        }
        HashMap::new()
    }

    fn persist_patterns(&self) {
        let p = self
            .persistence_path
            .parent()
            .map(|parent| parent.join("patterns.json"))
            .unwrap_or_else(|| PathBuf::from("data/patterns.json"));
        let vec: Vec<&PatternFinding> = self.patterns.values().collect();
        if let Ok(data) = serde_json::to_string_pretty(&vec) {
            let _ = std::fs::write(&p, data);
        }
    }

    fn load_receipts(path: &PathBuf) -> Vec<PatternReviewReceipt> {
        let p = path
            .parent()
            .map(|parent| parent.join("pattern_receipts.json"))
            .unwrap_or_else(|| PathBuf::from("data/pattern_receipts.json"));
        if let Ok(data) = std::fs::read_to_string(&p) {
            if let Ok(r) = serde_json::from_str::<Vec<PatternReviewReceipt>>(&data) {
                return r;
            }
        }
        Vec::new()
    }

    fn persist_receipts(&self) {
        let p = self
            .persistence_path
            .parent()
            .map(|parent| parent.join("pattern_receipts.json"))
            .unwrap_or_else(|| PathBuf::from("data/pattern_receipts.json"));
        if let Ok(data) = serde_json::to_string_pretty(&self.receipts) {
            let _ = std::fs::write(&p, data);
        }
    }

    fn load_config(path: &PathBuf) -> PatternDetectionConfig {
        let p = path
            .parent()
            .map(|parent| parent.join("pattern_config.json"))
            .unwrap_or_else(|| PathBuf::from("data/pattern_config.json"));
        if let Ok(data) = std::fs::read_to_string(&p) {
            if let Ok(c) = serde_json::from_str::<PatternDetectionConfig>(&data) {
                return c;
            }
        }
        default_config()
    }

    fn persist_config(&self) {
        let p = self
            .persistence_path
            .parent()
            .map(|parent| parent.join("pattern_config.json"))
            .unwrap_or_else(|| PathBuf::from("data/pattern_config.json"));
        if let Ok(data) = serde_json::to_string_pretty(&self.config) {
            let _ = std::fs::write(&p, data);
        }
    }

    // --- Configuration ---

    pub fn get_config(&self) -> PatternDetectionConfig {
        self.config.clone()
    }

    pub fn update_config(&mut self, config: PatternDetectionConfig) {
        self.config = config;
        self.persist_config();
    }

    fn effective_min_findings(&self) -> u32 {
        if let Some(ref ps) = self.policy_service {
            if let Ok(svc) = ps.try_lock() {
                if let Some(val) = svc.get_policy_value("pattern.min_findings") {
                    if let Some(n) = val.as_u64() {
                        return n as u32;
                    }
                }
            }
        }
        self.config.default_min_findings
    }

    fn effective_time_window_hours(&self) -> u32 {
        if let Some(ref ps) = self.policy_service {
            if let Ok(svc) = ps.try_lock() {
                if let Some(val) = svc.get_policy_value("pattern.time_window_hours") {
                    if let Some(n) = val.as_u64() {
                        return n as u32;
                    }
                }
            }
        }
        self.config.default_time_window_hours
    }

    fn effective_expiration_days(&self) -> u32 {
        if let Some(ref ps) = self.policy_service {
            if let Ok(svc) = ps.try_lock() {
                if let Some(val) = svc.get_policy_value("pattern.expiration_days") {
                    if let Some(n) = val.as_u64() {
                        return n as u32;
                    }
                }
            }
        }
        self.config.expiration_days
    }

    // --- Pattern Detection ---

    pub fn detect_patterns(
        &mut self,
        classification_service: &EvidenceClassificationService,
        _anomaly_service: &AnomalyDetectionService,
    ) -> Vec<PatternFinding> {
        let now = chrono::Utc::now().to_rfc3339();
        let node_id = "self".to_string();

        // Gather active findings (status != resolved/dismissed)
        let findings = classification_service.get_findings(None, None);
        let active_findings: Vec<&ClassifiedFinding> = findings
            .iter()
            .filter(|f| {
                f.owner_review_status != "resolved" && f.owner_review_status != "dismissed"
            })
            .collect();

        // Group classified findings by category + affected entity
        let mut finding_groups: HashMap<(String, String), Vec<&ClassifiedFinding>> = HashMap::new();
        for f in &active_findings {
            let entity_key = f
                .affected_entity_id
                .clone()
                .unwrap_or_else(|| f.affected_entity_type.clone());
            finding_groups
                .entry((f.category.clone(), entity_key))
                .or_default()
                .push(f);
        }

        let mut new_patterns = Vec::new();

        for ((category, entity_key), group) in &finding_groups {
            let threshold = self
                .config
                .thresholds
                .iter()
                .find(|t| t.category == *category);

            let min_findings = threshold
                .map(|t| t.min_findings)
                .unwrap_or_else(|| self.effective_min_findings());
            let time_window = threshold
                .map(|t| t.time_window_hours)
                .unwrap_or_else(|| self.effective_time_window_hours());

            if group.len() < min_findings as usize {
                continue;
            }

            // Check time window
            let cutoff = chrono::Utc::now()
                - chrono::Duration::hours(time_window as i64);
            let cutoff_str = cutoff.to_rfc3339();

            let in_window: Vec<&&ClassifiedFinding> = group
                .iter()
                .filter(|f| f.generated_at >= cutoff_str)
                .collect();

            if in_window.len() < min_findings as usize {
                continue;
            }

            // Check severity threshold
            let min_sev = threshold
                .map(|t| t.min_severity.as_str())
                .unwrap_or("notable");
            let meets_severity = group.iter().any(|f| {
                let order: [&str; 4] = ["info", "notable", "warning", "critical"];
                let f_idx = order.iter().position(|&s| s == f.severity.as_str()).unwrap_or(0);
                let min_idx = order.iter().position(|&s| s == min_sev).unwrap_or(1);
                f_idx >= min_idx
            });
            if !meets_severity {
                continue;
            }

            // Build constituent IDs
            let finding_ids: Vec<String> =
                group.iter().map(|f| f.finding_id.clone()).collect();
            let anomaly_ids: Vec<String> = Vec::new();

            // Determine highest severity among constituents
            let highest_severity = group
                .iter()
                .fold("info".to_string(), |acc, f| merge_severity(&acc, &f.severity));

            // Confidence
            let confidences: Vec<String> =
                group.iter().map(|f| f.confidence.clone()).collect();
            let pattern_confidence = merge_confidence(&confidences);

            // Evidence references
            let mut evidence_refs: Vec<String> = Vec::new();
            let workload_ids: Vec<String> = Vec::new();
            for f in group.iter() {
                for r in &f.evidence_references {
                    if !evidence_refs.contains(r) {
                        evidence_refs.push(r.clone());
                    }
                }
            }

            let provenance = PatternProvenance {
                evidence_references: evidence_refs,
                workload_ids,
                session_ids: Vec::new(),
                custody_envelope_ids: Vec::new(),
            };

            // Check for overlap with existing patterns (same category + entity key)
            let existing_key = format!("{}::{}", category, entity_key);
            let existing_pattern = self.patterns.values().find(|p| {
                let p_key = format!(
                    "{}::{}",
                    p.category,
                    p.affected_entity_id
                        .clone()
                        .unwrap_or_else(|| p.affected_entity_type.clone())
                );
                p_key == existing_key
                    && p.status != "resolved"
                    && p.status != "dismissed"
            });

            if let Some(existing) = existing_pattern {
                // Merge: update with higher severity, extend finding IDs, reset timer
                let merged_severity = merge_severity(&existing.severity, &highest_severity);
                let merged_confidence = merge_confidence(&[existing.confidence.clone(), pattern_confidence]);

                let mut merged_finding_ids = existing.constituent_finding_ids.clone();
                for fid in &finding_ids {
                    if !merged_finding_ids.contains(fid) {
                        merged_finding_ids.push(fid.clone());
                    }
                }

                let updated = PatternFinding {
                    pattern_id: existing.pattern_id.clone(),
                    category: existing.category.clone(),
                    title: existing.title.clone(),
                    description: existing.description.clone(),
                    severity: merged_severity,
                    status: existing.status.clone(),
                    affected_node_id: existing.affected_node_id.clone(),
                    affected_entity_type: existing.affected_entity_type.clone(),
                    affected_entity_id: existing.affected_entity_id.clone(),
                    constituent_finding_ids: merged_finding_ids,
                    constituent_anomaly_ids: existing.constituent_anomaly_ids.clone(),
                    first_detected_at: existing.first_detected_at.clone(),
                    last_observed_at: now.clone(),
                    finding_count: existing.finding_count + in_window.len() as u32,
                    time_window_hours: time_window,
                    confidence: merged_confidence,
                    provenance: existing.provenance.clone(),
                    owner_review_status: existing.owner_review_status.clone(),
                };

                self.patterns.insert(existing.pattern_id.clone(), updated.clone());
                new_patterns.push(updated);
            } else {
                // Create new pattern
                let title = format!("Pattern: {} on {}", category, entity_key);
                let description = format!(
                    "{} findings of category '{}' affecting '{}' within {} hour window",
                    in_window.len(),
                    category,
                    entity_key,
                    time_window
                );

                let pattern = PatternFinding {
                    pattern_id: Uuid::new_v4().to_string(),
                    category: category.clone(),
                    title,
                    description,
                    severity: highest_severity,
                    status: "detected".to_string(),
                    affected_node_id: node_id.clone(),
                    affected_entity_type: group[0].affected_entity_type.clone(),
                    affected_entity_id: Some(entity_key.clone()),
                    constituent_finding_ids: finding_ids,
                    constituent_anomaly_ids: anomaly_ids,
                    first_detected_at: now.clone(),
                    last_observed_at: now.clone(),
                    finding_count: in_window.len() as u32,
                    time_window_hours: time_window,
                    confidence: pattern_confidence,
                    provenance,
                    owner_review_status: "pending".to_string(),
                };

                let id = pattern.pattern_id.clone();
                self.patterns.insert(id, pattern.clone());
                new_patterns.push(pattern);
            }
        }

        if !new_patterns.is_empty() {
            self.persist_patterns();
        }

        new_patterns
    }

    // --- Pattern Lifecycle ---

    pub fn acknowledge_pattern(
        &mut self,
        pattern_id: &str,
        note: Option<String>,
    ) -> Option<PatternReviewReceipt> {
        let pattern = self.patterns.get_mut(pattern_id)?;
        let previous_status = pattern.owner_review_status.clone();

        if previous_status != "pending" {
            return None;
        }

        let new_status = "acknowledged".to_string();
        pattern.owner_review_status = new_status.clone();
        pattern.status = status_from_review_status(&new_status);

        let receipt = PatternReviewReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            action_id: Uuid::new_v4().to_string(),
            pattern_id: pattern_id.to_string(),
            previous_status,
            new_status,
            action: "acknowledge".to_string(),
            note,
            acted_at: chrono::Utc::now().to_rfc3339(),
        };

        self.receipts.push(receipt.clone());
        self.persist_patterns();
        self.persist_receipts();
        Some(receipt)
    }

    pub fn resolve_pattern(
        &mut self,
        pattern_id: &str,
        note: Option<String>,
    ) -> Option<PatternReviewReceipt> {
        let pattern = self.patterns.get_mut(pattern_id)?;
        let previous_status = pattern.owner_review_status.clone();

        if previous_status != "acknowledged" && previous_status != "monitoring" {
            return None;
        }

        let new_status = "resolved".to_string();
        pattern.owner_review_status = new_status.clone();
        pattern.status = status_from_review_status(&new_status);

        let receipt = PatternReviewReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            action_id: Uuid::new_v4().to_string(),
            pattern_id: pattern_id.to_string(),
            previous_status,
            new_status,
            action: "resolve".to_string(),
            note,
            acted_at: chrono::Utc::now().to_rfc3339(),
        };

        self.receipts.push(receipt.clone());
        self.persist_patterns();
        self.persist_receipts();
        Some(receipt)
    }

    pub fn dismiss_pattern(
        &mut self,
        pattern_id: &str,
        note: Option<String>,
    ) -> Option<PatternReviewReceipt> {
        let pattern = self.patterns.get_mut(pattern_id)?;
        let previous_status = pattern.owner_review_status.clone();

        if previous_status != "pending" && previous_status != "acknowledged" {
            return None;
        }

        let new_status = "dismissed".to_string();
        pattern.owner_review_status = new_status.clone();
        pattern.status = status_from_review_status(&new_status);

        let receipt = PatternReviewReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            action_id: Uuid::new_v4().to_string(),
            pattern_id: pattern_id.to_string(),
            previous_status,
            new_status,
            action: "dismiss".to_string(),
            note,
            acted_at: chrono::Utc::now().to_rfc3339(),
        };

        self.receipts.push(receipt.clone());
        self.persist_patterns();
        self.persist_receipts();
        Some(receipt)
    }

    // --- Queries ---

    pub fn get_patterns(
        &self,
        status_filter: Option<&str>,
        category_filter: Option<&str>,
    ) -> Vec<PatternFinding> {
        self.patterns
            .values()
            .filter(|p| {
                let status_ok = status_filter.map_or(true, |s| p.status == s);
                let cat_ok = category_filter.map_or(true, |c| p.category == c);
                status_ok && cat_ok
            })
            .cloned()
            .collect()
    }

    pub fn get_pattern(&self, pattern_id: &str) -> Option<PatternFinding> {
        self.patterns.get(pattern_id).cloned()
    }

    pub fn get_summary(&self) -> PatternSummary {
        let total = self.patterns.len() as u32;
        let active = self
            .patterns
            .values()
            .filter(|p| p.status == "detected" || p.status == "pending_review" || p.status == "acknowledged" || p.status == "monitoring")
            .count() as u32;
        let pending = self
            .patterns
            .values()
            .filter(|p| p.owner_review_status == "pending")
            .count() as u32;
        let acknowledged = self
            .patterns
            .values()
            .filter(|p| p.owner_review_status == "acknowledged")
            .count() as u32;
        let monitoring = self
            .patterns
            .values()
            .filter(|p| p.owner_review_status == "monitoring")
            .count() as u32;

        let mut by_severity = PatternSeverityCounts {
            info: 0,
            notable: 0,
            warning: 0,
            critical: 0,
        };
        let mut by_category_map: HashMap<String, u32> = HashMap::new();

        for p in self.patterns.values() {
            match p.severity.as_str() {
                "info" => by_severity.info += 1,
                "notable" => by_severity.notable += 1,
                "warning" => by_severity.warning += 1,
                "critical" => by_severity.critical += 1,
                _ => {}
            }
            *by_category_map.entry(p.category.clone()).or_insert(0) += 1;
        }

        let by_category: Vec<PatternCategoryCount> = by_category_map
            .into_iter()
            .map(|(category, count)| PatternCategoryCount { category, count })
            .collect();

        let mut latest: Vec<PatternFinding> = self.patterns.values().cloned().collect();
        latest.sort_by(|a, b| b.last_observed_at.cmp(&a.last_observed_at));
        latest.truncate(10);

        PatternSummary {
            total_patterns: total,
            active_patterns: active,
            pending_review: pending,
            acknowledged,
            monitoring,
            by_severity,
            by_category,
            latest_patterns: latest,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn get_receipts(&self) -> Vec<PatternReviewReceipt> {
        self.receipts.clone()
    }

    // --- Expiration ---

    pub fn expire_old_patterns(&mut self) -> Vec<PatternFinding> {
        let now = chrono::Utc::now();
        let exp_days = self.effective_expiration_days();
        let expiration = chrono::Duration::days(exp_days as i64);
        let mut expired = Vec::new();

        let to_expire: Vec<String> = self
            .patterns
            .iter()
            .filter(|(_, p)| {
                if p.status == "resolved" || p.status == "dismissed" {
                    return false;
                }
                let last = match chrono::DateTime::parse_from_rfc3339(&p.last_observed_at) {
                    Ok(dt) => dt.with_timezone(&chrono::Utc),
                    Err(_) => return false,
                };
                let elapsed = now - last;
                elapsed >= expiration
            })
            .map(|(id, _)| id.clone())
            .collect();

        for id in to_expire {
            if let Some(pattern) = self.patterns.get_mut(&id) {
                pattern.owner_review_status = "resolved".to_string();
                pattern.status = "resolved".to_string();
                expired.push(pattern.clone());
            }
        }

        if !expired.is_empty() {
            self.persist_patterns();
        }

        expired
    }

    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_contracts::evidence_classification::ClassifiedFinding;
    use librarian_contracts::evidence_intelligence::IntelligenceFinding;
    use tempfile::tempdir;

    fn make_classification_service(dir: &tempfile::TempDir) -> EvidenceClassificationService {
        EvidenceClassificationService::new(dir.path().join("classify.json"))
    }

    fn make_anomaly_service(dir: &tempfile::TempDir) -> AnomalyDetectionService {
        AnomalyDetectionService::new(dir.path().join("anomaly.json"))
    }

    fn make_service(dir: &tempfile::TempDir) -> PatternEscalationService {
        PatternEscalationService::new(dir.path().join("patterns.json"))
    }

    fn seed_active_findings(svc: &mut EvidenceClassificationService, category: &str, entity_id: &str, count: u32, severity: &str) {
        for i in 0..count {
            let (intel_category, title_fmt) = match category {
                "performance_degradation" => (
                    "workload_outcome",
                    format!("Workload type '{}' has performance issues", entity_id),
                ),
                "capability_mismatch" => (
                    "capability",
                    format!("Capability '{}' mismatch", entity_id),
                ),
                "repeated_failure" => (
                    "workload_outcome",
                    format!("Workload type '{}' repeatedly failing", entity_id),
                ),
                "allocation_drift" => (
                    "allocation",
                    format!("Allocation drift for '{}'", entity_id),
                ),
                "node_instability" => (
                    "node_health",
                    format!("Node '{}' instability", entity_id),
                ),
                _ => ("workload_outcome", format!("Workload type '{}' issue", entity_id)),
            };
            let raw = IntelligenceFinding {
                finding_id: uuid::Uuid::new_v4().to_string(),
                category: intel_category.to_string(),
                severity: severity.to_string(),
                title: title_fmt,
                description: format!("Test finding {} for pattern detection", i),
                supporting_data: serde_json::json!({}),
                source_references: vec![format!("ref-{}-{}", category, i)],
                generated_at: chrono::Utc::now().to_rfc3339(),
            };
            svc.classify_finding(raw, None);
        }
    }

    // --- Test 1: Pattern detection groups findings by category + entity ---

    #[test]
    fn pattern_detection_groups_findings_by_category_and_entity() {
        let dir = tempdir().unwrap();
        let mut class_svc = make_classification_service(&dir);
        let anomaly_svc = make_anomaly_service(&dir);
        let mut pattern_svc = make_service(&dir);

        seed_active_findings(&mut class_svc, "performance_degradation", "inference", 4, "warning");
        seed_active_findings(&mut class_svc, "performance_degradation", "embedding", 2, "warning");

        let results = pattern_svc.detect_patterns(&class_svc, &anomaly_svc);

        // 4 findings -> 1 pattern (inference meets threshold of 3), embedding only 2 -> no pattern
        assert_eq!(results.len(), 1, "Should create one pattern for inference (4 findings >= 3)");
        assert_eq!(results[0].category, "performance_degradation");
        assert_eq!(results[0].affected_entity_id.as_deref(), Some("inference"));
        assert_eq!(results[0].finding_count, 4);
    }

    // --- Test 2: Pattern detection respects time window and min_findings threshold ---

    #[test]
    fn pattern_detection_respects_time_window_and_min_findings() {
        let dir = tempdir().unwrap();
        let mut class_svc = make_classification_service(&dir);
        let anomaly_svc = make_anomaly_service(&dir);
        let mut pattern_svc = make_service(&dir);

        // Only 2 findings (below default 3)
        seed_active_findings(&mut class_svc, "performance_degradation", "test", 2, "warning");

        let results = pattern_svc.detect_patterns(&class_svc, &anomaly_svc);
        assert!(results.is_empty(), "2 findings should not trigger pattern with min 3");
    }

    // --- Test 3: Overlapping patterns are merged correctly ---

    #[test]
    fn overlapping_patterns_merged_into_highest_severity() {
        let dir = tempdir().unwrap();
        let mut class_svc = make_classification_service(&dir);
        let anomaly_svc = make_anomaly_service(&dir);
        let mut pattern_svc = make_service(&dir);

        // First detection
        seed_active_findings(&mut class_svc, "node_instability", "node-a", 3, "warning");
        let first = pattern_svc.detect_patterns(&class_svc, &anomaly_svc);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].severity, "warning");
        let first_id = first[0].pattern_id.clone();

        // Add more severe findings and detect again
        seed_active_findings(&mut class_svc, "node_instability", "node-a", 2, "critical");
        let second = pattern_svc.detect_patterns(&class_svc, &anomaly_svc);
        assert!(!second.is_empty(), "Should produce merged pattern");
        let merged = &second[0];

        // Should have merged into highest severity (critical)
        assert_eq!(merged.severity, "critical", "Should merge to highest severity");
        assert_eq!(merged.pattern_id, first_id, "Should reuse same pattern ID on merge");
        // Should now have 5 constituent findings
        assert_eq!(merged.constituent_finding_ids.len(), 5, "Should include all finding IDs after merge");
    }

    // --- Test 4: Pattern review lifecycle ---

    #[test]
    fn pattern_review_lifecycle_acknowledge_resolve_dismiss() {
        let dir = tempdir().unwrap();
        let mut class_svc = make_classification_service(&dir);
        let anomaly_svc = make_anomaly_service(&dir);
        let mut pattern_svc = make_service(&dir);

        seed_active_findings(&mut class_svc, "performance_degradation", "test", 4, "warning");
        let results = pattern_svc.detect_patterns(&class_svc, &anomaly_svc);
        assert_eq!(results.len(), 1);
        let pattern_id = results[0].pattern_id.clone();
        assert_eq!(results[0].status, "detected");
        assert_eq!(results[0].owner_review_status, "pending");

        // Acknowledge
        let receipt = pattern_svc.acknowledge_pattern(&pattern_id, Some("Reviewed".to_string()));
        assert!(receipt.is_some());
        assert_eq!(receipt.as_ref().unwrap().previous_status, "pending");
        assert_eq!(receipt.as_ref().unwrap().new_status, "acknowledged");

        let p = pattern_svc.get_pattern(&pattern_id).unwrap();
        assert_eq!(p.owner_review_status, "acknowledged");
        assert_eq!(p.status, "acknowledged");

        // Cannot acknowledge again (must be pending)
        let second = pattern_svc.acknowledge_pattern(&pattern_id, None);
        assert!(second.is_none(), "Should not allow acknowledge from non-pending status");

        // Resolve
        let resolve_receipt = pattern_svc.resolve_pattern(&pattern_id, Some("Fixed".to_string()));
        assert!(resolve_receipt.is_some());
        assert_eq!(resolve_receipt.as_ref().unwrap().previous_status, "acknowledged");
        assert_eq!(resolve_receipt.as_ref().unwrap().new_status, "resolved");

        let p2 = pattern_svc.get_pattern(&pattern_id).unwrap();
        assert_eq!(p2.owner_review_status, "resolved");

        // Dismiss should fail from resolved
        let dismiss = pattern_svc.dismiss_pattern(&pattern_id, None);
        assert!(dismiss.is_none(), "Should not allow dismiss from resolved");

        // Test dismiss from pending
        let dir2 = tempdir().unwrap();
        let mut class_svc2 = make_classification_service(&dir2);
        let anomaly_svc2 = make_anomaly_service(&dir2);
        let mut pattern_svc2 = make_service(&dir2);

        seed_active_findings(&mut class_svc2, "allocation_drift", "test", 4, "notable");
        let results2 = pattern_svc2.detect_patterns(&class_svc2, &anomaly_svc2);
        let pid2 = results2[0].pattern_id.clone();

        let dismiss2 = pattern_svc2.dismiss_pattern(&pid2, Some("Not relevant".to_string()));
        assert!(dismiss2.is_some());
        assert_eq!(dismiss2.as_ref().unwrap().new_status, "dismissed");
        let p3 = pattern_svc2.get_pattern(&pid2).unwrap();
        assert_eq!(p3.owner_review_status, "dismissed");
    }

    // --- Test 5: Pattern expiration ---

    #[test]
    fn pattern_expiration_works() {
        let dir = tempdir().unwrap();
        let mut class_svc = make_classification_service(&dir);
        let anomaly_svc = make_anomaly_service(&dir);
        let mut pattern_svc = make_service(&dir);

        seed_active_findings(&mut class_svc, "performance_degradation", "old", 4, "warning");
        let results = pattern_svc.detect_patterns(&class_svc, &anomaly_svc);
        assert_eq!(results.len(), 1);
        let pattern_id = results[0].pattern_id.clone();

        // Manually set last_observed_at to be 8+ days ago
        let old_time = (chrono::Utc::now() - chrono::Duration::days(8)).to_rfc3339();
        if let Some(p) = pattern_svc.patterns.get_mut(&pattern_id) {
            p.last_observed_at = old_time;
        }

        let expired = pattern_svc.expire_old_patterns();
        assert_eq!(expired.len(), 1, "Should expire the old pattern");
        let p = pattern_svc.get_pattern(&pattern_id).unwrap();
        assert_eq!(p.status, "resolved");
        assert_eq!(p.owner_review_status, "resolved");
    }

    // --- Test 6: Patterns persist across restarts ---

    #[test]
    fn patterns_persist_across_restarts() {
        let dir = tempdir().unwrap();
        let persistence_path = dir.path().join("persist.json");

        {
            let mut class_svc = make_classification_service(&dir);
            let anomaly_svc = make_anomaly_service(&dir);
            let mut pattern_svc = PatternEscalationService::new(persistence_path.clone());

            seed_active_findings(&mut class_svc, "performance_degradation", "test", 4, "warning");
            pattern_svc.detect_patterns(&class_svc, &anomaly_svc);
            assert_eq!(pattern_svc.pattern_count(), 1);
        }

        {
            let pattern_svc = PatternEscalationService::new(persistence_path);
            assert_eq!(pattern_svc.pattern_count(), 1);
            let patterns = pattern_svc.get_patterns(None, None);
            assert_eq!(patterns[0].category, "performance_degradation");
            assert_eq!(patterns[0].finding_count, 4);
        }
    }

    // --- Test 7: Configuration update persists ---

    #[test]
    fn configuration_update_persists() {
        let dir = tempdir().unwrap();
        let persistence_path = dir.path().join("config_persist.json");

        {
            let mut pattern_svc = PatternEscalationService::new(persistence_path.clone());
            let config = pattern_svc.get_config();
            assert_eq!(config.default_min_findings, 3);
            assert_eq!(config.expiration_days, 7);

            let mut updated = config.clone();
            updated.default_min_findings = 5;
            updated.expiration_days = 14;
            pattern_svc.update_config(updated);
        }

        {
            let pattern_svc = PatternEscalationService::new(persistence_path);
            let config = pattern_svc.get_config();
            assert_eq!(config.default_min_findings, 5);
            assert_eq!(config.expiration_days, 14);
        }
    }

    // --- Test 8: Summary query ---

    #[test]
    fn get_summary_returns_correct_counts() {
        let dir = tempdir().unwrap();
        let mut class_svc = make_classification_service(&dir);
        let anomaly_svc = make_anomaly_service(&dir);
        let mut pattern_svc = make_service(&dir);

        seed_active_findings(&mut class_svc, "performance_degradation", "type-a", 4, "warning");
        seed_active_findings(&mut class_svc, "node_instability", "node-1", 3, "critical");

        pattern_svc.detect_patterns(&class_svc, &anomaly_svc);

        let summary = pattern_svc.get_summary();
        assert_eq!(summary.total_patterns, 2);
        assert_eq!(summary.pending_review, 2);
        assert_eq!(summary.by_severity.warning, 1);
        assert_eq!(summary.by_severity.critical, 1);
        assert_eq!(summary.latest_patterns.len(), 2);
    }

    // --- Test 9: Pattern without severity threshold fails to detect ---

    #[test]
    fn detection_respects_min_severity_threshold() {
        let dir = tempdir().unwrap();
        let mut class_svc = make_classification_service(&dir);
        let anomaly_svc = make_anomaly_service(&dir);
        let mut pattern_svc = make_service(&dir);

        // Add 4 findings with "info" severity - below the default "notable" threshold
        seed_active_findings(&mut class_svc, "performance_degradation", "low-sev", 4, "info");

        let results = pattern_svc.detect_patterns(&class_svc, &anomaly_svc);
        // Should be empty because severity "info" < "notable" (default min_severity)
        assert!(results.is_empty(), "Info severity findings should not create pattern with notable minimum");
    }

    // --- Test 10: Phase 2 Safety Gate — no allocation, no action triggers ---

    #[test]
    fn pattern_contains_no_allocation_or_action_triggers() {
        // This test verifies the API surface of PatternFinding does not include
        // allocation fields, action fields, threshold modifications, or session creation.
        let pf = PatternFinding {
            pattern_id: "test".to_string(),
            category: "test".to_string(),
            title: "test".to_string(),
            description: "test".to_string(),
            severity: "info".to_string(),
            status: "detected".to_string(),
            affected_node_id: "node".to_string(),
            affected_entity_type: "workload_type".to_string(),
            affected_entity_id: Some("test".to_string()),
            constituent_finding_ids: vec![],
            constituent_anomaly_ids: vec![],
            first_detected_at: "now".to_string(),
            last_observed_at: "now".to_string(),
            finding_count: 0,
            time_window_hours: 24,
            confidence: "low".to_string(),
            provenance: PatternProvenance {
                evidence_references: vec![],
                workload_ids: vec![],
                session_ids: vec![],
                custody_envelope_ids: vec![],
            },
            owner_review_status: "pending".to_string(),
        };
        // Ensure NO allocation-related fields exist
        let json_val = serde_json::to_value(&pf).unwrap();
        let map = json_val.as_object().unwrap();
        assert!(!map.contains_key("target_node_id"), "Pattern must not contain target_node_id");
        assert!(!map.contains_key("action"), "Pattern must not contain action field");
        assert!(!map.contains_key("allocation_id"), "Pattern must not contain allocation_id");
        assert!(!map.contains_key("session_id"), "Pattern must not contain top-level session_id");
        assert!(!map.contains_key("threshold_modification"), "Pattern must not contain threshold_modification");
    }
}
