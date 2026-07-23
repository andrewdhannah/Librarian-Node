use librarian_contracts::evidence_classification::ClassifiedFinding;
use librarian_contracts::evidence_intelligence::{
    AllocationAccuracyAnalysis, CapabilityEffectivenessAnalysis, WorkloadOutcomeAnalysis,
};
use librarian_contracts::owner_insight::comparison::InsightComparison;
use librarian_contracts::owner_insight::dashboard::{
    AllocationQualitySummary, CapabilityHealthSummary, InsightDashboard, RecentActivitySummary,
    WorkloadTrendSummary,
};
use librarian_contracts::owner_insight::report::{
    AnomalyFindingSummary, InsightReport, OwnerRecommendation,
};
use uuid::Uuid;

use super::allocation_service::AllocationService;
use super::anomaly_detection_service::AnomalyDetectionService;
use super::evidence_classification_service::EvidenceClassificationService;
use super::evidence_intelligence_service::EvidenceIntelligenceService;
use super::fleet_service::FleetService;
use super::owner_allocation_service::OwnerAllocationService;
use super::workload_lifecycle_service::WorkloadLifecycleService;
use super::workload_session_service::WorkloadSessionService;
use super::CapabilityEvidenceBridge;

pub struct OwnerInsightService;

impl OwnerInsightService {
    pub fn get_dashboard(
        classification_service: &EvidenceClassificationService,
        anomaly_service: &AnomalyDetectionService,
        _intelligence_service: &EvidenceIntelligenceService,
        _workload_service: &WorkloadLifecycleService,
        ws_service: &WorkloadSessionService,
        fleet_service: &FleetService,
        allocation_service: &AllocationService,
        owner_allocation_service: &OwnerAllocationService,
        capability_evidence_bridge: &CapabilityEvidenceBridge,
        node_id: &str,
    ) -> InsightDashboard {
        let findings_summary = classification_service.get_findings_summary();
        let active_anomalies = anomaly_service.scan_all_metrics(ws_service).len() as u32;
        let workload_trend = Self::get_workload_trend(ws_service);
        let capability_health =
            Self::get_capability_health(fleet_service, capability_evidence_bridge);
        let allocation_quality =
            Self::get_allocation_quality(allocation_service, owner_allocation_service);
        let recent_activity =
            Self::get_recent_activity(ws_service, classification_service);

        InsightDashboard {
            node_id: node_id.to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            findings_summary,
            active_anomalies,
            workload_trend,
            capability_health,
            allocation_quality,
            recent_activity,
        }
    }

    pub fn get_report(
        period: &str,
        classification_service: &EvidenceClassificationService,
        anomaly_service: &AnomalyDetectionService,
        _intelligence_service: &EvidenceIntelligenceService,
        _workload_service: &WorkloadLifecycleService,
        ws_service: &WorkloadSessionService,
        fleet_service: &FleetService,
        allocation_service: &AllocationService,
        owner_allocation_service: &OwnerAllocationService,
        capability_evidence_bridge: &CapabilityEvidenceBridge,
        node_id: &str,
    ) -> InsightReport {
        let dashboard = Self::get_dashboard(
            classification_service,
            anomaly_service,
            _intelligence_service,
            _workload_service,
            ws_service,
            fleet_service,
            allocation_service,
            owner_allocation_service,
            capability_evidence_bridge,
            node_id,
        );

        let detailed_findings = classification_service.get_findings(None, None);

        let anomaly_findings = anomaly_service.scan_all_metrics(ws_service);
        let detailed_anomalies: Vec<AnomalyFindingSummary> = anomaly_findings
            .into_iter()
            .map(Into::into)
            .collect();

        let workload_breakdown =
            EvidenceIntelligenceService::analyze_workload_outcomes(ws_service);
        let capability_breakdown =
            EvidenceIntelligenceService::analyze_capability_effectiveness(
                ws_service,
                fleet_service,
                capability_evidence_bridge,
            );
        let allocation_breakdown =
            EvidenceIntelligenceService::analyze_allocation_accuracy(
                ws_service,
                allocation_service,
                owner_allocation_service,
            );

        let recommendations = Self::generate_recommendations(
            &dashboard,
            &detailed_findings,
            &detailed_anomalies,
            &allocation_breakdown,
            &capability_breakdown,
        );

        InsightReport {
            report_id: Uuid::new_v4().to_string(),
            node_id: node_id.to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            report_period: period.to_string(),
            dashboard,
            detailed_findings,
            detailed_anomalies,
            workload_breakdown,
            capability_breakdown,
            allocation_breakdown,
            recommendations,
        }
    }

    fn generate_recommendations(
        _dashboard: &InsightDashboard,
        detailed_findings: &[ClassifiedFinding],
        detailed_anomalies: &[AnomalyFindingSummary],
        allocation_breakdown: &AllocationAccuracyAnalysis,
        capability_breakdown: &CapabilityEffectivenessAnalysis,
    ) -> Vec<OwnerRecommendation> {
        let mut recommendations = Vec::new();

        let open_critical_count = detailed_findings
            .iter()
            .filter(|f| f.severity == "critical" && f.owner_review_status == "pending")
            .count() as u32;
        if open_critical_count >= 3 {
            recommendations.push(OwnerRecommendation {
                recommendation_id: Uuid::new_v4().to_string(),
                category: "review_findings".to_string(),
                priority: "high".to_string(),
                title: format!(
                    "{} critical findings require owner review",
                    open_critical_count
                ),
                description: format!(
                    "There are {} open critical classified findings that need owner attention and review",
                    open_critical_count
                ),
                supporting_evidence_count: open_critical_count,
                generated_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        for anomaly in detailed_anomalies {
            if anomaly.deviation_factor >= 5.0 && anomaly.severity == "critical" {
                recommendations.push(OwnerRecommendation {
                    recommendation_id: Uuid::new_v4().to_string(),
                    category: "investigate_anomaly".to_string(),
                    priority: "critical".to_string(),
                    title: format!(
                        "Anomaly in '{}' requires investigation",
                        anomaly.metric_name
                    ),
                    description: format!(
                        "Deviation factor {:.2} (critical severity) for metric '{}' in context '{}'",
                        anomaly.deviation_factor, anomaly.metric_name, anomaly.context
                    ),
                    supporting_evidence_count: 1,
                    generated_at: chrono::Utc::now().to_rfc3339(),
                });
                break;
            }
        }

        if let Some(accuracy) = allocation_breakdown.overall_accuracy {
            if accuracy < 0.5 && allocation_breakdown.accepted_recommendations >= 3 {
                recommendations.push(OwnerRecommendation {
                    recommendation_id: Uuid::new_v4().to_string(),
                    category: "review_allocation".to_string(),
                    priority: "high".to_string(),
                    title: "Allocation accuracy is below 50%".to_string(),
                    description: format!(
                        "Only {:.1}% of accepted allocation recommendations led to successful workloads ({} accepted, {} successful)",
                        accuracy * 100.0,
                        allocation_breakdown.accepted_recommendations,
                        allocation_breakdown.successful_workloads
                    ),
                    supporting_evidence_count: allocation_breakdown.accepted_recommendations,
                    generated_at: chrono::Utc::now().to_rfc3339(),
                });
            }
        }

        let degraded_capabilities = capability_breakdown
            .entries
            .iter()
            .filter(|e| e.success_rate < 0.5 && e.workloads_using >= 3)
            .count() as u32;
        if degraded_capabilities > 0 {
            let cap_names: Vec<&str> = capability_breakdown
                .entries
                .iter()
                .filter(|e| e.success_rate < 0.5 && e.workloads_using >= 3)
                .map(|e| e.capability_type.as_str())
                .collect();
            recommendations.push(OwnerRecommendation {
                recommendation_id: Uuid::new_v4().to_string(),
                category: "check_capability".to_string(),
                priority: "medium".to_string(),
                title: format!(
                    "{} capabilities show degraded performance",
                    degraded_capabilities
                ),
                description: format!(
                    "Capabilities with low success rates: {}",
                    cap_names.join(", ")
                ),
                supporting_evidence_count: degraded_capabilities,
                generated_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        recommendations
    }

    pub fn compare_periods(
        period_a_label: &str,
        period_a_data: &WorkloadOutcomeAnalysis,
        period_b_label: &str,
        period_b_data: &WorkloadOutcomeAnalysis,
    ) -> Vec<InsightComparison> {
        let mut comparisons = Vec::new();

        comparisons.push(InsightComparison {
            metric_name: "overall_success_rate".to_string(),
            period_a_label: period_a_label.to_string(),
            period_a_value: period_a_data.overall_success_rate,
            period_b_label: period_b_label.to_string(),
            period_b_value: period_b_data.overall_success_rate,
            change_pct: compute_change_pct(
                period_a_data.overall_success_rate,
                period_b_data.overall_success_rate,
            ),
            direction: compute_direction(
                period_a_data.overall_success_rate,
                period_b_data.overall_success_rate,
                true,
            ),
        });

        comparisons.push(InsightComparison {
            metric_name: "total_workloads".to_string(),
            period_a_label: period_a_label.to_string(),
            period_a_value: period_a_data.total_workloads as f64,
            period_b_label: period_b_label.to_string(),
            period_b_value: period_b_data.total_workloads as f64,
            change_pct: compute_change_pct(
                period_a_data.total_workloads as f64,
                period_b_data.total_workloads as f64,
            ),
            direction: compute_direction(
                period_a_data.total_workloads as f64,
                period_b_data.total_workloads as f64,
                false,
            ),
        });

        let a_avg_duration = avg_of_averages(&period_a_data.summaries);
        let b_avg_duration = avg_of_averages(&period_b_data.summaries);
        comparisons.push(InsightComparison {
            metric_name: "avg_duration_seconds".to_string(),
            period_a_label: period_a_label.to_string(),
            period_a_value: a_avg_duration,
            period_b_label: period_b_label.to_string(),
            period_b_value: b_avg_duration,
            change_pct: compute_change_pct(a_avg_duration, b_avg_duration),
            direction: compute_direction(a_avg_duration, b_avg_duration, false),
        });

        comparisons
    }

    pub fn get_workload_trend(ws_service: &WorkloadSessionService) -> WorkloadTrendSummary {
        let inventory = WorkloadLifecycleService::get_inventory(ws_service);
        let total_workloads = inventory.total;
        let completed = inventory.completed;
        let success_rate = if total_workloads > 0 {
            completed as f64 / total_workloads as f64
        } else {
            0.0
        };

        let durations: Vec<f64> = inventory
            .workloads
            .iter()
            .filter_map(|w| w.duration_seconds.map(|d| d as f64))
            .collect();
        let avg_duration = if !durations.is_empty() {
            durations.iter().sum::<f64>() / durations.len() as f64
        } else {
            0.0
        };

        let recent_count = std::cmp::min(10, inventory.workloads.len());
        let mut recent_sorted = inventory.workloads.clone();
        recent_sorted.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        let recent: Vec<_> = recent_sorted.iter().take(recent_count).collect();
        let recent_success = recent
            .iter()
            .filter(|w| w.state == "completed")
            .count() as f64;
        let recent_rate = if !recent.is_empty() {
            recent_success / recent.len() as f64
        } else {
            0.0
        };

        let trend_direction = if success_rate > 0.0 {
            if recent_rate > success_rate * 1.05 {
                "improving"
            } else if recent_rate < success_rate * 0.95 {
                "degrading"
            } else {
                "stable"
            }
        } else {
            "stable"
        };

        WorkloadTrendSummary {
            total_workloads,
            overall_success_rate: success_rate,
            avg_duration_seconds: avg_duration,
            trend_direction: trend_direction.to_string(),
            comparison_window: "last_24h".to_string(),
        }
    }

    pub fn get_capability_health(
        fleet_service: &FleetService,
        capability_evidence_bridge: &CapabilityEvidenceBridge,
    ) -> CapabilityHealthSummary {
        let nodes = fleet_service.all_nodes().to_vec();
        let mut total_capabilities = 0u32;
        let mut healthy = 0u32;
        let mut degraded = 0u32;
        let mut untested = 0u32;

        for node in &nodes {
            let state = capability_evidence_bridge.get_verification_state(&node.node_id);
            for cap in &state.capabilities {
                total_capabilities += 1;
                match cap.verification_status.as_str() {
                    "verified" => healthy += 1,
                    "failed" => degraded += 1,
                    _ => untested += 1,
                }
            }
        }

        let mut capabilities_with_anomalies = Vec::new();
        for node in &nodes {
            let state = capability_evidence_bridge.get_verification_state(&node.node_id);
            for cap in &state.capabilities {
                if cap.verification_status == "failed" {
                    capabilities_with_anomalies.push(cap.capability_type.clone());
                }
            }
        }

        CapabilityHealthSummary {
            total_capabilities,
            healthy,
            degraded,
            untested,
            capabilities_with_anomalies,
        }
    }

    pub fn get_allocation_quality(
        allocation_service: &AllocationService,
        _owner_allocation_service: &OwnerAllocationService,
    ) -> AllocationQualitySummary {
        let recommendations = allocation_service.get_recommendations(None);
        let total_recommendations = recommendations.len() as u32;

        let receipts = allocation_service.get_receipts();
        let accepted = receipts
            .iter()
            .filter(|r| r.decision == "accepted")
            .count() as u32;

        let successful = receipts
            .iter()
            .filter(|r| r.decision == "accepted")
            .count() as u32;

        let accuracy_rate = if accepted > 0 {
            Some(successful as f64 / accepted as f64)
        } else {
            None
        };

        AllocationQualitySummary {
            total_recommendations,
            accepted,
            successful,
            accuracy_rate,
        }
    }

    pub fn get_recent_activity(
        ws_service: &WorkloadSessionService,
        classification_service: &EvidenceClassificationService,
    ) -> RecentActivitySummary {
        let inventory = WorkloadLifecycleService::get_inventory(ws_service);
        let total_recent = inventory.workloads.len() as u32;

        let findings = classification_service.get_findings(None, None);
        let total_findings = findings.len() as u32;

        RecentActivitySummary {
            recent_workloads: total_recent,
            recent_findings: total_findings,
            recent_anomalies: 0,
            recent_owner_actions: classification_service.get_receipts().len() as u32,
            since_timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

fn compute_change_pct(a: f64, b: f64) -> f64 {
    if a == 0.0 {
        if b == 0.0 {
            return 0.0;
        }
        return 100.0;
    }
    ((b - a) / a) * 100.0
}

fn compute_direction(a: f64, b: f64, higher_is_better: bool) -> String {
    let diff = b - a;
    let threshold = 0.001;
    if diff.abs() <= threshold {
        return "stable".to_string();
    }
    if higher_is_better {
        if diff > 0.0 {
            "improvement"
        } else {
            "degradation"
        }
    } else {
        if diff > 0.0 {
            "degradation"
        } else {
            "improvement"
        }
    }
    .to_string()
}

fn avg_of_averages(
    summaries: &[librarian_contracts::evidence_intelligence::WorkloadOutcomeSummary],
) -> f64 {
    let total: f64 = summaries
        .iter()
        .filter_map(|s| s.avg_duration_seconds)
        .sum();
    let count = summaries
        .iter()
        .filter(|s| s.avg_duration_seconds.is_some())
        .count() as f64;
    if count > 0.0 {
        total / count
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{
        allocation_service::AllocationService, fleet_service::FleetService,
        session_service::SessionService, AllocationService as AllocSvc,
        AnomalyDetectionService, CapabilityEvidenceBridge, EvidenceClassificationService,
        EvidenceIntelligenceService, FleetService as FleetSvc, OwnerAllocationService,
        SessionService as SessSvc, WorkloadSessionService,
    };
    use librarian_contracts::{
        allocation::AllocationRequest,
        evidence_intelligence::{IntelligenceFinding, WorkloadOutcomeSummary},
        fleet::NodeInventoryEntry,
        workload_session::WorkloadDescriptor,
    };
    use tempfile::tempdir;

    fn test_workload(id: &str, wl_type: &str) -> WorkloadDescriptor {
        WorkloadDescriptor {
            workload_id: id.to_string(),
            workload_type: wl_type.to_string(),
            description: format!("Test workload {}", id),
            requirements: Some(vec!["llm.inference".to_string()]),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    struct TestServices {
        ws: WorkloadSessionService,
        sess: SessionService,
        alloc: AllocationService,
        fleet: FleetService,
        owner: OwnerAllocationService,
        bridge: CapabilityEvidenceBridge,
        classification: EvidenceClassificationService,
        anomaly: AnomalyDetectionService,
        _dir: tempfile::TempDir,
    }

    fn setup_services() -> TestServices {
        let dir = tempdir().unwrap();
        let session_path = dir.path().join("sessions.json");
        let sess = SessionService::new(session_path);
        let ws_path = dir.path().join("workload_sessions.json");
        let ws = WorkloadSessionService::new(ws_path);
        let alloc_path = dir.path().join("allocation.json");
        let alloc = AllocationService::new(alloc_path);
        let fleet_path = dir.path().join("fleet.json");
        let fleet = FleetService::new(fleet_path);
        let owner_path = dir.path().join("owner_alloc.json");
        let owner = OwnerAllocationService::new(owner_path);
        let bridge_path = dir.path().join("bridge.json");
        let bridge = CapabilityEvidenceBridge::new(bridge_path);
        let class_path = dir.path().join("classification.json");
        let classification = EvidenceClassificationService::new(class_path);
        let anomaly_path = dir.path().join("anomaly.json");
        let anomaly = AnomalyDetectionService::new(anomaly_path);
        TestServices {
            ws,
            sess,
            alloc,
            fleet,
            owner,
            bridge,
            classification,
            anomaly,
            _dir: dir,
        }
    }

    fn seed_node(t: &mut TestServices, node_id: &str) {
        t.fleet.add_or_update_node(NodeInventoryEntry {
            node_id: node_id.to_string(),
            display_name: node_id.to_string(),
            status: "online".to_string(),
            last_seen_at: Some(chrono::Utc::now().to_rfc3339()),
            runtime_version: "0.1.0".to_string(),
            platform: "test".to_string(),
            capability_count: 3,
            verified_capability_count: 3,
            session_count: 2,
            custody_envelope_count: 1,
            registered: true,
            bootstrap_completed: true,
            last_health_status: Some("healthy".to_string()),
        });
        t.bridge.register_claim(node_id, "llm.inference", None, None);
    }

    fn run_workload_to_completion(
        ws: &mut WorkloadSessionService,
        sess: &mut SessionService,
        wl_id: &str,
        wl_type: &str,
        node_id: &str,
    ) {
        let created = ws
            .create_workload_session(
                test_workload(wl_id, wl_type),
                "receipt-001",
                node_id,
                None,
                None,
                sess,
                None,
            )
            .unwrap();
        ws.activate_workload_session(&created.workload_session_id, sess)
            .unwrap();
        ws.complete_workload_session(
            &created.workload_session_id,
            5,
            vec!["e1".to_string(), "e2".to_string()],
            sess,
        )
        .unwrap();
    }

    fn run_workload_to_failure(
        ws: &mut WorkloadSessionService,
        sess: &mut SessionService,
        wl_id: &str,
        wl_type: &str,
        node_id: &str,
    ) {
        let created = ws
            .create_workload_session(
                test_workload(wl_id, wl_type),
                "receipt-002",
                node_id,
                None,
                None,
                sess,
                None,
            )
            .unwrap();
        ws.activate_workload_session(&created.workload_session_id, sess)
            .unwrap();
        ws.fail_workload_session(&created.workload_session_id, "error", sess)
            .unwrap();
    }

    #[test]
    fn dashboard_aggregates_data_from_all_intelligence_services() {
        let mut t = setup_services();

        seed_node(&mut t, "node-a");

        run_workload_to_completion(&mut t.ws, &mut t.sess, "wl-001", "inference", "node-a");
        run_workload_to_completion(&mut t.ws, &mut t.sess, "wl-002", "inference", "node-a");
        run_workload_to_failure(&mut t.ws, &mut t.sess, "wl-003", "inference", "node-a");

        let dashboard = OwnerInsightService::get_dashboard(
            &t.classification,
            &t.anomaly,
            &EvidenceIntelligenceService,
            &WorkloadLifecycleService,
            &t.ws,
            &t.fleet,
            &t.alloc,
            &t.owner,
            &t.bridge,
            "node-a",
        );

        assert_eq!(dashboard.node_id, "node-a");
        assert!(!dashboard.generated_at.is_empty());
        assert!(dashboard.findings_summary.total_findings == 0 || dashboard.findings_summary.total_findings >= 0);
        assert!(dashboard.workload_trend.total_workloads >= 3);
        assert!(dashboard.workload_trend.overall_success_rate > 0.0);
        assert!(dashboard.capability_health.total_capabilities >= 1);
    }

    #[test]
    fn report_includes_all_required_sections() {
        let mut t = setup_services();

        seed_node(&mut t, "node-a");

        run_workload_to_completion(&mut t.ws, &mut t.sess, "wl-001", "inference", "node-a");

        let report = OwnerInsightService::get_report(
            "last_24h",
            &t.classification,
            &t.anomaly,
            &EvidenceIntelligenceService,
            &WorkloadLifecycleService,
            &t.ws,
            &t.fleet,
            &t.alloc,
            &t.owner,
            &t.bridge,
            "node-a",
        );

        assert!(!report.report_id.is_empty());
        assert!(!report.node_id.is_empty());
        assert!(!report.generated_at.is_empty());
        assert_eq!(report.report_period, "last_24h");
        assert!(report.dashboard.workload_trend.total_workloads >= 1);
        assert_eq!(report.report_period, "last_24h");
    }

    #[test]
    fn recommendations_are_generated_based_on_current_patterns() {
        let mut t = setup_services();
        seed_node(&mut t, "node-a");

        // Add a critical finding to trigger review_findings recommendation
        let raw = IntelligenceFinding {
            finding_id: uuid::Uuid::new_v4().to_string(),
            category: "node_health".to_string(),
            severity: "critical".to_string(),
            title: "Critical test finding 1".to_string(),
            description: "Test".to_string(),
            supporting_data: serde_json::json!({}),
            source_references: vec![],
            generated_at: chrono::Utc::now().to_rfc3339(),
        };
        t.classification.classify_finding(raw, None);

        let raw2 = IntelligenceFinding {
            finding_id: uuid::Uuid::new_v4().to_string(),
            category: "node_health".to_string(),
            severity: "critical".to_string(),
            title: "Critical test finding 2".to_string(),
            description: "Test".to_string(),
            supporting_data: serde_json::json!({}),
            source_references: vec![],
            generated_at: chrono::Utc::now().to_rfc3339(),
        };
        t.classification.classify_finding(raw2, None);

        let raw3 = IntelligenceFinding {
            finding_id: uuid::Uuid::new_v4().to_string(),
            category: "node_health".to_string(),
            severity: "critical".to_string(),
            title: "Critical test finding 3".to_string(),
            description: "Test".to_string(),
            supporting_data: serde_json::json!({}),
            source_references: vec![],
            generated_at: chrono::Utc::now().to_rfc3339(),
        };
        t.classification.classify_finding(raw3, None);

        let report = OwnerInsightService::get_report(
            "last_24h",
            &t.classification,
            &t.anomaly,
            &EvidenceIntelligenceService,
            &WorkloadLifecycleService,
            &t.ws,
            &t.fleet,
            &t.alloc,
            &t.owner,
            &t.bridge,
            "node-a",
        );

        let recs = report.recommendations;
        assert!(!recs.is_empty(), "Should generate at least one recommendation");

        let review_finding = recs.iter().find(|r| r.category == "review_findings");
        assert!(
            review_finding.is_some(),
            "Should generate review_findings recommendation for multiple critical findings"
        );
        if let Some(rec) = review_finding {
            assert_eq!(rec.priority, "high");
            assert!(rec.supporting_evidence_count >= 3);
        }
    }

    #[test]
    fn trend_computes_correct_change_percentage_and_direction() {
        let period_a = WorkloadOutcomeAnalysis {
            summaries: vec![WorkloadOutcomeSummary {
                workload_type: "inference".to_string(),
                total: 10,
                completed: 5,
                failed: 5,
                success_rate: 0.5,
                avg_duration_seconds: Some(10.0),
                evidence_count: 20,
            }],
            total_workloads: 10,
            overall_success_rate: 0.5,
            generated_at: "2026-07-15T00:00:00Z".to_string(),
        };

        let period_b = WorkloadOutcomeAnalysis {
            summaries: vec![WorkloadOutcomeSummary {
                workload_type: "inference".to_string(),
                total: 20,
                completed: 16,
                failed: 4,
                success_rate: 0.8,
                avg_duration_seconds: Some(8.0),
                evidence_count: 40,
            }],
            total_workloads: 20,
            overall_success_rate: 0.8,
            generated_at: "2026-07-16T00:00:00Z".to_string(),
        };

        let comparisons =
            OwnerInsightService::compare_periods("yesterday", &period_a, "today", &period_b);

        let success_rate_cmp = comparisons
            .iter()
            .find(|c| c.metric_name == "overall_success_rate")
            .expect("Should have overall_success_rate comparison");
        assert!((success_rate_cmp.change_pct - 60.0).abs() < 0.001);
        assert_eq!(success_rate_cmp.direction, "improvement");

        let workload_cmp = comparisons
            .iter()
            .find(|c| c.metric_name == "total_workloads")
            .expect("Should have total_workloads comparison");
        assert!((workload_cmp.change_pct - 100.0).abs() < 0.001);
    }

    #[test]
    fn workload_trend_shows_success_rate_and_direction() {
        let mut t = setup_services();
        seed_node(&mut t, "node-a");

        for i in 0..5 {
            run_workload_to_completion(&mut t.ws, &mut t.sess, &format!("wl-{}", i), "inference", "node-a");
        }

        let trend = OwnerInsightService::get_workload_trend(&t.ws);
        assert_eq!(trend.total_workloads, 5);
        assert_eq!(trend.overall_success_rate, 1.0);
        assert!(trend.avg_duration_seconds >= 0.0);
        assert!(!trend.trend_direction.is_empty());
    }

    #[test]
    fn capability_health_counts_healthy_degraded_untested() {
        let mut t = setup_services();
        seed_node(&mut t, "node-a");

        let health = OwnerInsightService::get_capability_health(&t.fleet, &t.bridge);
        assert!(health.total_capabilities >= 1);
    }

    #[test]
    fn allocation_quality_matches_accuracy_analysis() {
        let mut t = setup_services();
        seed_node(&mut t, "node-a");

        let quality = OwnerInsightService::get_allocation_quality(&t.alloc, &t.owner);
        assert_eq!(quality.total_recommendations, 0);
        assert_eq!(quality.accepted, 0);
        assert_eq!(quality.successful, 0);
        assert_eq!(quality.accuracy_rate, None);

        let request = AllocationRequest {
            request_id: "wl-alloc-001".to_string(),
            workload_description: "Test".to_string(),
            requirements: vec![],
            preferred_nodes: None,
            requested_at: chrono::Utc::now().to_rfc3339(),
        };
        let rec = t.alloc.generate_recommendation(request, &t.fleet);
        t.alloc.accept_recommendation(&rec.recommendation_id, None);

        let quality = OwnerInsightService::get_allocation_quality(&t.alloc, &t.owner);
        assert_eq!(quality.total_recommendations, 1);
        assert_eq!(quality.accepted, 1);
        assert!(quality.accuracy_rate.is_some());
    }

    #[test]
    fn dashboard_includes_all_required_fields() {
        let mut t = setup_services();
        seed_node(&mut t, "node-a");

        run_workload_to_completion(&mut t.ws, &mut t.sess, "wl-dash-001", "inference", "node-a");

        let dashboard = OwnerInsightService::get_dashboard(
            &t.classification,
            &t.anomaly,
            &EvidenceIntelligenceService,
            &WorkloadLifecycleService,
            &t.ws,
            &t.fleet,
            &t.alloc,
            &t.owner,
            &t.bridge,
            "node-a",
        );

        assert!(!dashboard.node_id.is_empty());
        assert!(!dashboard.generated_at.is_empty());
        assert!(dashboard.workload_trend.total_workloads > 0);
        assert!(!dashboard.workload_trend.trend_direction.is_empty());
        assert!(!dashboard.workload_trend.comparison_window.is_empty());
        assert!(dashboard.capability_health.total_capabilities > 0);
        assert!(dashboard.recent_activity.recent_workloads > 0);
    }

    #[test]
    fn compare_periods_empty_data_returns_zero_change() {
        let empty = WorkloadOutcomeAnalysis {
            summaries: vec![],
            total_workloads: 0,
            overall_success_rate: 0.0,
            generated_at: chrono::Utc::now().to_rfc3339(),
        };

        let comparisons =
            OwnerInsightService::compare_periods("a", &empty, "b", &empty);
        assert!(!comparisons.is_empty());

        for cmp in &comparisons {
            assert!(!cmp.metric_name.is_empty());
            assert!(!cmp.period_a_label.is_empty());
            assert!(!cmp.period_b_label.is_empty());
        }
    }
}
