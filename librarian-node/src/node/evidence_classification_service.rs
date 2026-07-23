use std::collections::HashMap;
use std::path::PathBuf;

use librarian_contracts::evidence_classification::{
    ClassifiedFinding, FindingCatalog, FindingCategory, FindingCategoryCount, FindingReviewAction,
    FindingReviewReceipt, FindingSeverityCounts, FindingSummary,
};
use librarian_contracts::evidence_intelligence::{
    AllocationAccuracyAnalysis, CapabilityEffectivenessAnalysis, IntelligenceFinding,
    WorkloadOutcomeAnalysis,
};
use uuid::Uuid;

fn predefined_catalog() -> FindingCatalog {
    FindingCatalog {
        categories: vec![
            FindingCategory {
                category: "performance_degradation".to_string(),
                display_name: "Performance Degradation".to_string(),
                description: "Workload duration or resource usage outside expected range".to_string(),
                severity_default: "warning".to_string(),
                requires_owner_review: true,
            },
            FindingCategory {
                category: "capability_mismatch".to_string(),
                display_name: "Capability Mismatch".to_string(),
                description:
                    "Workload requirements don't match node's verified capabilities".to_string(),
                severity_default: "critical".to_string(),
                requires_owner_review: true,
            },
            FindingCategory {
                category: "repeated_failure".to_string(),
                display_name: "Repeated Failure".to_string(),
                description: "Same workload type failing repeatedly on the same node".to_string(),
                severity_default: "warning".to_string(),
                requires_owner_review: true,
            },
            FindingCategory {
                category: "allocation_drift".to_string(),
                display_name: "Allocation Drift".to_string(),
                description:
                    "Allocation recommendations consistently selecting suboptimal nodes".to_string(),
                severity_default: "notable".to_string(),
                requires_owner_review: true,
            },
            FindingCategory {
                category: "node_instability".to_string(),
                display_name: "Node Instability".to_string(),
                description:
                    "Node health fluctuating or custody integrity issues".to_string(),
                severity_default: "critical".to_string(),
                requires_owner_review: true,
            },
        ],
        version: "1.0.0".to_string(),
    }
}

fn severity_from_context(category_default: &str, _context: &IntelligenceFinding) -> String {
    match _context.severity.as_str() {
        "critical" => "critical".to_string(),
        "warning" => "warning".to_string(),
        "notable" => "notable".to_string(),
        "info" => "info".to_string(),
        _ => category_default.to_string(),
    }
}

fn confidence_from_evidence_count(count: usize) -> String {
    if count >= 10 {
        "high".to_string()
    } else if count >= 5 {
        "medium".to_string()
    } else if count >= 1 {
        "low".to_string()
    } else {
        "low".to_string()
    }
}

fn detection_method_for_context(raw: &IntelligenceFinding) -> String {
    match raw.category.as_str() {
        "workload_outcome" => "outcome_analysis".to_string(),
        "capability" => "capability_effectiveness".to_string(),
        "allocation" => "allocation_accuracy".to_string(),
        "node_health" => "anomaly_detection".to_string(),
        "trend" => "outcome_analysis".to_string(),
        _ => "outcome_analysis".to_string(),
    }
}

fn affected_entity_for_context(raw: &IntelligenceFinding) -> (String, Option<String>) {
    match raw.category.as_str() {
        "workload_outcome" | "repeated_failure" => {
            ("workload_type".to_string(), raw.title.split("'").nth(1).map(|s| s.to_string()))
        }
        "capability" => {
            ("capability".to_string(), raw.title.split("'").nth(1).map(|s| s.to_string()))
        }
        "allocation" | "allocation_drift" => {
            ("workload".to_string(), None)
        }
        "node_health" | "node_instability" => {
            ("node".to_string(), raw.title.split("'").nth(1).map(|s| s.to_string()))
        }
        _ => ("workload".to_string(), None),
    }
}

pub struct EvidenceClassificationService {
    findings: HashMap<String, ClassifiedFinding>,
    receipts: Vec<FindingReviewReceipt>,
    catalog: FindingCatalog,
    persistence_path: PathBuf,
}

impl EvidenceClassificationService {
    pub fn new(persistence_path: PathBuf) -> Self {
        let catalog = predefined_catalog();
        let findings = Self::load_findings(&persistence_path);
        let receipts = Self::load_receipts(&persistence_path);
        EvidenceClassificationService {
            findings,
            receipts,
            catalog,
            persistence_path,
        }
    }

    fn load_findings(path: &PathBuf) -> HashMap<String, ClassifiedFinding> {
        let findings_path = path.parent().map(|p| p.join("classified_findings.json"))
            .unwrap_or_else(|| PathBuf::from("data/classified_findings.json"));
        if let Ok(data) = std::fs::read_to_string(&findings_path) {
            if let Ok(found) = serde_json::from_str::<Vec<ClassifiedFinding>>(&data) {
                return found.into_iter().map(|f| (f.finding_id.clone(), f)).collect();
            }
        }
        HashMap::new()
    }

    fn load_receipts(path: &PathBuf) -> Vec<FindingReviewReceipt> {
        let receipts_path = path.parent().map(|p| p.join("review_receipts.json"))
            .unwrap_or_else(|| PathBuf::from("data/review_receipts.json"));
        if let Ok(data) = std::fs::read_to_string(&receipts_path) {
            if let Ok(r) = serde_json::from_str::<Vec<FindingReviewReceipt>>(&data) {
                return r;
            }
        }
        Vec::new()
    }

    fn persist_findings(&self) {
        let findings_path = self.persistence_path.parent()
            .map(|p| p.join("classified_findings.json"))
            .unwrap_or_else(|| PathBuf::from("data/classified_findings.json"));
        let findings_vec: Vec<&ClassifiedFinding> = self.findings.values().collect();
        if let Ok(data) = serde_json::to_string_pretty(&findings_vec) {
            let _ = std::fs::write(&findings_path, data);
        }
    }

    fn persist_receipts(&self) {
        let receipts_path = self.persistence_path.parent()
            .map(|p| p.join("review_receipts.json"))
            .unwrap_or_else(|| PathBuf::from("data/review_receipts.json"));
        if let Ok(data) = serde_json::to_string_pretty(&self.receipts) {
            let _ = std::fs::write(&receipts_path, data);
        }
    }

    pub fn get_catalog(&self) -> &FindingCatalog {
        &self.catalog
    }

    pub fn classify_finding(
        &mut self,
        raw: IntelligenceFinding,
        _analysis_context: Option<serde_json::Value>,
    ) -> ClassifiedFinding {
        let category_def = self
            .catalog
            .categories
            .iter()
            .find(|c| {
                matches!(
                    (c.category.as_str(), raw.category.as_str()),
                    ("performance_degradation", "workload_outcome")
                        | ("capability_mismatch", "capability")
                        | ("repeated_failure", "workload_outcome")
                        | ("allocation_drift", "allocation")
                        | ("node_instability", "node_health")
                )
            })
            .unwrap_or_else(|| {
                self.catalog
                    .categories
                    .first()
                    .expect("catalog must have at least one category")
            });

        let severity = severity_from_context(&category_def.severity_default, &raw);
        let confidence = confidence_from_evidence_count(raw.source_references.len());
        let detection_method = detection_method_for_context(&raw);
        let (entity_type, entity_id) = affected_entity_for_context(&raw);

        let classified = ClassifiedFinding {
            finding_id: raw.finding_id,
            category: category_def.category.clone(),
            severity,
            title: raw.title,
            description: raw.description,
            confidence,
            detection_method,
            affected_entity_type: entity_type,
            affected_entity_id: entity_id,
            evidence_references: raw.source_references,
            owner_review_status: "pending".to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
        };

        let id = classified.finding_id.clone();
        self.findings.insert(id, classified.clone());
        self.persist_findings();
        classified
    }

    pub fn classify_workload_outcomes(
        &mut self,
        analysis: &WorkloadOutcomeAnalysis,
    ) -> Vec<ClassifiedFinding> {
        let mut results = Vec::new();
        for summary in &analysis.summaries {
            if summary.success_rate == 0.0 && summary.total > 0 {
                let raw = IntelligenceFinding {
                    finding_id: Uuid::new_v4().to_string(),
                    category: "workload_outcome".to_string(),
                    severity: "warning".to_string(),
                    title: format!("All workloads of type '{}' failed", summary.workload_type),
                    description: format!(
                        "{} out of {} workloads of type '{}' failed with 0% success rate",
                        summary.failed, summary.total, summary.workload_type
                    ),
                    supporting_data: serde_json::json!(summary),
                    source_references: Vec::new(),
                    generated_at: chrono::Utc::now().to_rfc3339(),
                };
                results.push(self.classify_finding(raw, None));
            } else if summary.success_rate < 0.5 && summary.total >= 3 {
                let raw = IntelligenceFinding {
                    finding_id: Uuid::new_v4().to_string(),
                    category: "workload_outcome".to_string(),
                    severity: "notable".to_string(),
                    title: format!(
                        "Workload type '{}' has low success rate",
                        summary.workload_type
                    ),
                    description: format!(
                        "Success rate {:.1}% for type '{}' ({}/{} completed)",
                        summary.success_rate * 100.0,
                        summary.workload_type,
                        summary.completed,
                        summary.total
                    ),
                    supporting_data: serde_json::json!(summary),
                    source_references: Vec::new(),
                    generated_at: chrono::Utc::now().to_rfc3339(),
                };
                results.push(self.classify_finding(raw, None));
            }
        }
        results
    }

    pub fn classify_capability_effectiveness(
        &mut self,
        analysis: &CapabilityEffectivenessAnalysis,
    ) -> Vec<ClassifiedFinding> {
        let mut results = Vec::new();
        for entry in &analysis.entries {
            if entry.success_rate < 0.5 && entry.workloads_using >= 3 {
                let raw = IntelligenceFinding {
                    finding_id: Uuid::new_v4().to_string(),
                    category: "capability".to_string(),
                    severity: "warning".to_string(),
                    title: format!(
                        "Capability '{}' correlates with low workload success",
                        entry.capability_type
                    ),
                    description: format!(
                        "Only {:.1}% success rate for {} workloads on nodes with capability '{}'",
                        entry.success_rate * 100.0,
                        entry.workloads_using,
                        entry.capability_type
                    ),
                    supporting_data: serde_json::json!(entry),
                    source_references: Vec::new(),
                    generated_at: chrono::Utc::now().to_rfc3339(),
                };
                results.push(self.classify_finding(raw, None));
            }
        }
        results
    }

    pub fn classify_allocation_accuracy(
        &mut self,
        analysis: &AllocationAccuracyAnalysis,
    ) -> Vec<ClassifiedFinding> {
        let mut results = Vec::new();
        if let Some(accuracy) = analysis.overall_accuracy {
            if accuracy < 0.5 && analysis.accepted_recommendations >= 3 {
                let raw = IntelligenceFinding {
                    finding_id: Uuid::new_v4().to_string(),
                    category: "allocation".to_string(),
                    severity: "critical".to_string(),
                    title: "Allocation recommendations have low accuracy".to_string(),
                    description: format!(
                        "Only {:.1}% of accepted recommendations led to successful workloads ({} accepted, {} successful)",
                        accuracy * 100.0,
                        analysis.accepted_recommendations,
                        analysis.successful_workloads
                    ),
                    supporting_data: serde_json::json!(analysis),
                    source_references: Vec::new(),
                    generated_at: chrono::Utc::now().to_rfc3339(),
                };
                results.push(self.classify_finding(raw, None));
            }
        }
        results
    }

    pub fn get_findings(
        &self,
        status_filter: Option<&str>,
        category_filter: Option<&str>,
    ) -> Vec<ClassifiedFinding> {
        self.findings
            .values()
            .filter(|f| {
                let status_ok = status_filter.map_or(true, |s| f.owner_review_status == s);
                let cat_ok = category_filter.map_or(true, |c| f.category == c);
                status_ok && cat_ok
            })
            .cloned()
            .collect()
    }

    pub fn get_findings_summary(&self) -> FindingSummary {
        let total_findings = self.findings.len() as u32;
        let pending_review = self
            .findings
            .values()
            .filter(|f| f.owner_review_status == "pending")
            .count() as u32;
        let acknowledged = self
            .findings
            .values()
            .filter(|f| f.owner_review_status == "acknowledged")
            .count() as u32;

        let mut by_severity = FindingSeverityCounts {
            info: 0,
            notable: 0,
            warning: 0,
            critical: 0,
        };
        let mut by_category_map: HashMap<String, u32> = HashMap::new();

        for f in self.findings.values() {
            match f.severity.as_str() {
                "info" => by_severity.info += 1,
                "notable" => by_severity.notable += 1,
                "warning" => by_severity.warning += 1,
                "critical" => by_severity.critical += 1,
                _ => {}
            }
            *by_category_map.entry(f.category.clone()).or_insert(0) += 1;
        }

        let by_category: Vec<FindingCategoryCount> = by_category_map
            .into_iter()
            .map(|(category, count)| FindingCategoryCount { category, count })
            .collect();

        let mut latest: Vec<ClassifiedFinding> = self.findings.values().cloned().collect();
        latest.sort_by(|a, b| b.generated_at.cmp(&a.generated_at));
        latest.truncate(10);

        FindingSummary {
            total_findings,
            pending_review,
            acknowledged,
            by_severity,
            by_category,
            latest_findings: latest,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn review_finding(&mut self, action: FindingReviewAction) -> FindingReviewReceipt {
        let previous_status = self
            .findings
            .get(&action.finding_id)
            .map(|f| f.owner_review_status.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let new_status = match action.action.as_str() {
            "acknowledge" => "acknowledged".to_string(),
            "resolve" => "resolved".to_string(),
            "dismiss" => "dismissed".to_string(),
            _ => previous_status.clone(),
        };

        if let Some(finding) = self.findings.get_mut(&action.finding_id) {
            finding.owner_review_status = new_status.clone();
        }

        let receipt = FindingReviewReceipt {
            receipt_id: Uuid::new_v4().to_string(),
            action_id: action.action_id.clone(),
            finding_id: action.finding_id.clone(),
            previous_status,
            new_status,
            action: action.action.clone(),
            note: action.note,
            acted_at: action.acted_at,
        };

        self.receipts.push(receipt.clone());
        self.persist_findings();
        self.persist_receipts();
        receipt
    }

    pub fn get_receipts(&self) -> Vec<FindingReviewReceipt> {
        self.receipts.clone()
    }

    pub fn findings_count(&self) -> usize {
        self.findings.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librarian_contracts::evidence_intelligence::{
        AllocationAccuracyAnalysis, CapabilityEffectiveness,
        CapabilityEffectivenessAnalysis, WorkloadOutcomeAnalysis, WorkloadOutcomeSummary,
    };
    use tempfile::tempdir;

    fn make_service(dir: &tempfile::TempDir) -> EvidenceClassificationService {
        EvidenceClassificationService::new(dir.path().join("test.json"))
    }

    #[test]
    fn catalog_contains_all_predefined_categories() {
        let dir = tempdir().unwrap();
        let service = make_service(&dir);
        let catalog = service.get_catalog();
        assert_eq!(catalog.categories.len(), 5);
        let names: Vec<&str> = catalog.categories.iter().map(|c| c.category.as_str()).collect();
        assert!(names.contains(&"performance_degradation"));
        assert!(names.contains(&"capability_mismatch"));
        assert!(names.contains(&"repeated_failure"));
        assert!(names.contains(&"allocation_drift"));
        assert!(names.contains(&"node_instability"));

        for c in &catalog.categories {
            assert!(!c.display_name.is_empty());
            assert!(!c.description.is_empty());
            assert!(["info", "notable", "warning", "critical"].contains(&c.severity_default.as_str()));
            assert!(c.requires_owner_review);
        }
    }

    #[test]
    fn classify_workload_outcomes_produces_correct_category_and_severity() {
        let dir = tempdir().unwrap();
        let mut service = make_service(&dir);

        let analysis = WorkloadOutcomeAnalysis {
            summaries: vec![WorkloadOutcomeSummary {
                workload_type: "inference".to_string(),
                total: 5,
                completed: 0,
                failed: 5,
                success_rate: 0.0,
                avg_duration_seconds: None,
                evidence_count: 8,
            }],
            total_workloads: 5,
            overall_success_rate: 0.0,
            generated_at: chrono::Utc::now().to_rfc3339(),
        };

        let results = service.classify_workload_outcomes(&analysis);
        assert!(!results.is_empty());
        let first = &results[0];
        assert_eq!(first.category, "performance_degradation");
        assert_eq!(first.severity, "warning");
        assert_eq!(first.affected_entity_type, "workload_type");
        assert_eq!(first.confidence, "low");
    }

    #[test]
    fn classify_capability_effectiveness_produces_correct_category() {
        let dir = tempdir().unwrap();
        let mut service = make_service(&dir);

        let analysis = CapabilityEffectivenessAnalysis {
            entries: vec![CapabilityEffectiveness {
                capability_type: "llm.inference".to_string(),
                workloads_using: 10,
                successful_workloads: 2,
                failed_workloads: 8,
                success_rate: 0.2,
                avg_evidence_per_workload: Some(3.0),
            }],
            total_capabilities: 1,
            generated_at: chrono::Utc::now().to_rfc3339(),
        };

        let results = service.classify_capability_effectiveness(&analysis);
        assert!(!results.is_empty());
        assert_eq!(results[0].category, "capability_mismatch");
        assert_eq!(results[0].affected_entity_type, "capability");
    }

    #[test]
    fn classify_allocation_accuracy_produces_correct_category() {
        let dir = tempdir().unwrap();
        let mut service = make_service(&dir);

        let analysis = AllocationAccuracyAnalysis {
            total_recommendations: 10,
            accepted_recommendations: 8,
            successful_workloads: 2,
            failed_workloads: 6,
            overall_accuracy: Some(0.25),
            entries: vec![],
            generated_at: chrono::Utc::now().to_rfc3339(),
        };

        let results = service.classify_allocation_accuracy(&analysis);
        assert!(!results.is_empty());
        assert_eq!(results[0].category, "allocation_drift");
        assert_eq!(results[0].affected_entity_type, "workload");
    }

    #[test]
    fn finding_review_transitions_status_correctly() {
        let dir = tempdir().unwrap();
        let mut service = make_service(&dir);

        let raw = IntelligenceFinding {
            finding_id: Uuid::new_v4().to_string(),
            category: "workload_outcome".to_string(),
            severity: "warning".to_string(),
            title: "Test finding".to_string(),
            description: "Test".to_string(),
            supporting_data: serde_json::json!({}),
            source_references: vec!["ref1".to_string()],
            generated_at: chrono::Utc::now().to_rfc3339(),
        };
        service.classify_finding(raw, None);

        let findings = service.get_findings(None, None);
        assert_eq!(findings[0].owner_review_status, "pending");

        let receipt = service.review_finding(FindingReviewAction {
            action_id: Uuid::new_v4().to_string(),
            finding_id: findings[0].finding_id.clone(),
            action: "acknowledge".to_string(),
            note: Some("Looks good".to_string()),
            acted_at: chrono::Utc::now().to_rfc3339(),
        });

        assert_eq!(receipt.previous_status, "pending");
        assert_eq!(receipt.new_status, "acknowledged");

        let updated = service.get_findings(None, None);
        assert_eq!(updated[0].owner_review_status, "acknowledged");
    }

    #[test]
    fn review_receipt_includes_previous_and_new_status() {
        let dir = tempdir().unwrap();
        let mut service = make_service(&dir);

        let raw = IntelligenceFinding {
            finding_id: Uuid::new_v4().to_string(),
            category: "workload_outcome".to_string(),
            severity: "warning".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            supporting_data: serde_json::json!({}),
            source_references: vec![],
            generated_at: chrono::Utc::now().to_rfc3339(),
        };
        service.classify_finding(raw, None);

        let findings = service.get_findings(None, None);
        let receipt = service.review_finding(FindingReviewAction {
            action_id: Uuid::new_v4().to_string(),
            finding_id: findings[0].finding_id.clone(),
            action: "dismiss".to_string(),
            note: None,
            acted_at: chrono::Utc::now().to_rfc3339(),
        });

        assert_eq!(receipt.previous_status, "pending");
        assert_eq!(receipt.new_status, "dismissed");
        assert_eq!(receipt.action, "dismiss");
        assert!(!receipt.receipt_id.is_empty());
    }

    #[test]
    fn findings_persist_across_restarts() {
        let dir = tempdir().unwrap();
        let persistence_path = dir.path().join("persist.json");

        let raw = IntelligenceFinding {
            finding_id: "persist-test-001".to_string(),
            category: "node_health".to_string(),
            severity: "critical".to_string(),
            title: "Persist test".to_string(),
            description: "Testing persistence".to_string(),
            supporting_data: serde_json::json!({}),
            source_references: vec!["ref-a".to_string(), "ref-b".to_string()],
            generated_at: chrono::Utc::now().to_rfc3339(),
        };

        {
            let mut service = EvidenceClassificationService::new(persistence_path.clone());
            service.classify_finding(raw, None);
            assert_eq!(service.findings_count(), 1);
        }

        {
            let service = EvidenceClassificationService::new(persistence_path);
            assert_eq!(service.findings_count(), 1);
            let findings = service.get_findings(None, None);
            assert_eq!(findings[0].finding_id, "persist-test-001");
        }
    }
}
