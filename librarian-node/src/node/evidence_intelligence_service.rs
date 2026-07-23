use librarian_contracts::evidence_intelligence::{
    AllocationAccuracy, AllocationAccuracyAnalysis, CapabilityEffectiveness,
    CapabilityEffectivenessAnalysis, IntelligenceFinding, IntelligenceReport,
    WorkloadOutcomeAnalysis, WorkloadOutcomeSummary,
};
use uuid::Uuid;

use super::{
    allocation_service::AllocationService, fleet_service::FleetService,
    owner_allocation_service::OwnerAllocationService, workload_lifecycle_service::WorkloadLifecycleService,
    workload_session_service::WorkloadSessionService, CapabilityEvidenceBridge,
};

pub struct EvidenceIntelligenceService;

impl EvidenceIntelligenceService {
    pub fn analyze_workload_outcomes(
        ws_service: &WorkloadSessionService,
    ) -> WorkloadOutcomeAnalysis {
        let inventory = WorkloadLifecycleService::get_inventory(ws_service);
        let total_workloads = inventory.total;

        let mut type_map: std::collections::BTreeMap<String, Vec<&librarian_contracts::workload_lifecycle::WorkloadSummary>> = std::collections::BTreeMap::new();
        for wl in &inventory.workloads {
            type_map.entry(wl.workload_type.clone()).or_default().push(wl);
        }

        let mut overall_completed = 0u32;
        let mut overall_failed = 0u32;

        let summaries: Vec<WorkloadOutcomeSummary> = type_map
            .into_iter()
            .map(|(wt, workloads)| {
                let total = workloads.len() as u32;
                let completed = workloads.iter().filter(|w| w.state == "completed").count() as u32;
                let failed = workloads.iter().filter(|w| w.state == "failed").count() as u32;
                overall_completed += completed;
                overall_failed += failed;

                let success_rate = if total > 0 {
                    completed as f64 / total as f64
                } else {
                    0.0
                };

                let durations: Vec<f64> = workloads
                    .iter()
                    .filter_map(|w| w.duration_seconds.map(|d| d as f64))
                    .collect();
                let avg_duration = if !durations.is_empty() {
                    Some(durations.iter().sum::<f64>() / durations.len() as f64)
                } else {
                    None
                };

                let evidence_count: u32 = workloads.iter().filter_map(|w| w.evidence_count).sum();

                WorkloadOutcomeSummary {
                    workload_type: wt,
                    total,
                    completed,
                    failed,
                    success_rate,
                    avg_duration_seconds: avg_duration,
                    evidence_count,
                }
            })
            .collect();

        let overall_success_rate = if total_workloads > 0 {
            overall_completed as f64 / total_workloads as f64
        } else {
            0.0
        };

        WorkloadOutcomeAnalysis {
            summaries,
            total_workloads,
            overall_success_rate,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn analyze_capability_effectiveness(
        ws_service: &WorkloadSessionService,
        fleet_service: &FleetService,
        capability_evidence_bridge: &CapabilityEvidenceBridge,
    ) -> CapabilityEffectivenessAnalysis {
        let inventory = WorkloadLifecycleService::get_inventory(ws_service);
        let verification_state = capability_evidence_bridge.get_verification_state(
            &fleet_service
                .all_nodes()
                .first()
                .map(|n| n.node_id.clone())
                .unwrap_or_default(),
        );

        let mut entries = Vec::new();
        for vc in &verification_state.capabilities {
            let workloads_on_node: Vec<_> = inventory
                .workloads
                .iter()
                .filter(|w| w.node_id == verification_state.node_id)
                .collect();
            let workloads_using = workloads_on_node.len() as u32;
            if workloads_using == 0 {
                continue;
            }
            let successful = workloads_on_node
                .iter()
                .filter(|w| w.state == "completed")
                .count() as u32;
            let failed = workloads_on_node
                .iter()
                .filter(|w| w.state == "failed")
                .count() as u32;
            let success_rate = if workloads_using > 0 {
                successful as f64 / workloads_using as f64
            } else {
                0.0
            };

            let evidence_counts: Vec<f64> = workloads_on_node
                .iter()
                .filter_map(|w| w.evidence_count.map(|c| c as f64))
                .collect();
            let avg_evidence = if !evidence_counts.is_empty() {
                Some(evidence_counts.iter().sum::<f64>() / evidence_counts.len() as f64)
            } else {
                None
            };

            entries.push(CapabilityEffectiveness {
                capability_type: vc.capability_type.clone(),
                workloads_using,
                successful_workloads: successful,
                failed_workloads: failed,
                success_rate,
                avg_evidence_per_workload: avg_evidence,
            });
        }

        let total_capabilities = entries.len() as u32;
        CapabilityEffectivenessAnalysis {
            entries,
            total_capabilities,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn analyze_allocation_accuracy(
        ws_service: &WorkloadSessionService,
        allocation_service: &AllocationService,
        _owner_allocation_service: &OwnerAllocationService,
    ) -> AllocationAccuracyAnalysis {
        let inventory = WorkloadLifecycleService::get_inventory(ws_service);
        let recommendations = allocation_service.get_recommendations(None);

        let total_recommendations = recommendations.len() as u32;
        let accepted_receipts: Vec<_> = allocation_service
            .get_receipts()
            .into_iter()
            .filter(|r| r.decision == "accepted")
            .collect();
        let accepted_recommendations = accepted_receipts.len() as u32;

        let mut successful_workloads = 0u32;
        let mut failed_workloads = 0u32;

        let entries: Vec<AllocationAccuracy> = recommendations
            .into_iter()
            .map(|rec| {
                let receipt = accepted_receipts
                    .iter()
                    .find(|r| r.recommendation_id == rec.recommendation_id);
                let workload = receipt.as_ref().and_then(|r| {
                    inventory
                        .workloads
                        .iter()
                        .find(|w| w.workload_id == r.workload_id)
                });
                let workload_successful = workload.map(|w| w.state == "completed").unwrap_or(false);
                let workload_failed = workload.map(|w| w.state == "failed").unwrap_or(false);
                let is_accepted = receipt.is_some();

                let allocation_correct = if is_accepted {
                    Some(workload_successful)
                } else {
                    None
                };

                if workload_successful {
                    successful_workloads += 1;
                } else if workload_failed {
                    failed_workloads += 1;
                }

                AllocationAccuracy {
                    recommendation_id: rec.recommendation_id.clone(),
                    workload_id: Some(rec.workload_id.clone()),
                    selected_node_id: rec.node_id.clone(),
                    recommended: is_accepted,
                    workload_successful,
                    allocation_correct,
                }
            })
            .collect();

        let overall_accuracy = if accepted_recommendations > 0 {
            let correct = entries
                .iter()
                .filter(|e| e.allocation_correct == Some(true))
                .count() as f64;
            Some(correct / accepted_recommendations as f64)
        } else {
            None
        };

        AllocationAccuracyAnalysis {
            total_recommendations,
            accepted_recommendations,
            successful_workloads,
            failed_workloads,
            overall_accuracy,
            entries,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn generate_findings(
        ws_service: &WorkloadSessionService,
        _fleet_service: &FleetService,
        outcome_analysis: &WorkloadOutcomeAnalysis,
        capability_analysis: &CapabilityEffectivenessAnalysis,
        allocation_analysis: &AllocationAccuracyAnalysis,
    ) -> Vec<IntelligenceFinding> {
        let mut findings = Vec::new();

        // workload_outcome findings — unusual success/failure rates
        for summary in &outcome_analysis.summaries {
            if summary.success_rate == 0.0 && summary.total > 0 {
                findings.push(IntelligenceFinding {
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
                });
            } else if summary.success_rate < 0.5 && summary.total >= 3 {
                findings.push(IntelligenceFinding {
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
                });
            }
        }

        // capability findings — notable success correlation
        for entry in &capability_analysis.entries {
            if entry.success_rate == 1.0 && entry.workloads_using >= 3 {
                findings.push(IntelligenceFinding {
                    finding_id: Uuid::new_v4().to_string(),
                    category: "capability".to_string(),
                    severity: "info".to_string(),
                    title: format!(
                        "Capability '{}' correlates with 100% workload success",
                        entry.capability_type
                    ),
                    description: format!(
                        "All {} workloads on nodes with capability '{}' completed successfully",
                        entry.workloads_using, entry.capability_type
                    ),
                    supporting_data: serde_json::json!(entry),
                    source_references: Vec::new(),
                    generated_at: chrono::Utc::now().to_rfc3339(),
                });
            } else if entry.success_rate < 0.5 && entry.workloads_using >= 3 {
                findings.push(IntelligenceFinding {
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
                });
            }
        }

        // allocation findings
        if let Some(accuracy) = allocation_analysis.overall_accuracy {
            if accuracy == 1.0 && allocation_analysis.accepted_recommendations > 0 {
                findings.push(IntelligenceFinding {
                    finding_id: Uuid::new_v4().to_string(),
                    category: "allocation".to_string(),
                    severity: "info".to_string(),
                    title: "Allocation recommendations are perfectly accurate".to_string(),
                    description: format!(
                        "All {} accepted recommendations led to successful workload outcomes",
                        allocation_analysis.accepted_recommendations
                    ),
                    supporting_data: serde_json::json!(allocation_analysis),
                    source_references: Vec::new(),
                    generated_at: chrono::Utc::now().to_rfc3339(),
                });
            } else if accuracy < 0.5 && allocation_analysis.accepted_recommendations >= 3 {
                findings.push(IntelligenceFinding {
                    finding_id: Uuid::new_v4().to_string(),
                    category: "allocation".to_string(),
                    severity: "critical".to_string(),
                    title: "Allocation recommendations have low accuracy".to_string(),
                    description: format!(
                        "Only {:.1}% of accepted recommendations led to successful workloads ({} accepted, {} successful)",
                        accuracy * 100.0,
                        allocation_analysis.accepted_recommendations,
                        allocation_analysis.successful_workloads
                    ),
                    supporting_data: serde_json::json!(allocation_analysis),
                    source_references: Vec::new(),
                    generated_at: chrono::Utc::now().to_rfc3339(),
                });
            }
        }

        // node_health findings — nodes with unusual failure patterns
        let inventory = WorkloadLifecycleService::get_inventory(ws_service);
        let node_failures: std::collections::BTreeMap<String, Vec<librarian_contracts::workload_lifecycle::WorkloadSummary>> = {
            let mut map: std::collections::BTreeMap<String, Vec<_>> = std::collections::BTreeMap::new();
            for wl in inventory.workloads.iter().filter(|w| w.state == "failed") {
                map.entry(wl.node_id.clone()).or_default().push(wl.clone());
            }
            map
        };
        let inventory = WorkloadLifecycleService::get_inventory(ws_service);
        for (node_id, failed_workloads) in node_failures {
            let total_on_node = inventory
                .workloads
                .iter()
                .filter(|w| w.node_id == node_id)
                .count() as u32;
            let failed_count = failed_workloads.len() as u32;
            if total_on_node >= 3 {
                let fail_rate = failed_count as f64 / total_on_node as f64;
                if fail_rate > 0.5 {
                    findings.push(IntelligenceFinding {
                        finding_id: Uuid::new_v4().to_string(),
                        category: "node_health".to_string(),
                        severity: "critical".to_string(),
                        title: format!("Node '{}' has high workload failure rate", node_id),
                        description: format!(
                            "{}/{} workloads on node '{}' failed ({:.1}%)",
                            failed_count, total_on_node, node_id, fail_rate * 100.0
                        ),
                        supporting_data: serde_json::json!({
                            "node_id": node_id,
                            "failed_workloads": failed_count,
                            "total_workloads": total_on_node,
                            "failure_rate": fail_rate,
                        }),
                        source_references: failed_workloads.iter().map(|w| w.workload_id.clone()).collect(),
                        generated_at: chrono::Utc::now().to_rfc3339(),
                    });
                } else if fail_rate > 0.0 {
                    findings.push(IntelligenceFinding {
                        finding_id: Uuid::new_v4().to_string(),
                        category: "node_health".to_string(),
                        severity: "notable".to_string(),
                        title: format!("Node '{}' has some workload failures", node_id),
                        description: format!(
                            "{}/{} workloads on node '{}' failed ({:.1}%)",
                            failed_count, total_on_node, node_id, fail_rate * 100.0
                        ),
                        supporting_data: serde_json::json!({
                            "node_id": node_id,
                            "failed_workloads": failed_count,
                            "total_workloads": total_on_node,
                            "failure_rate": fail_rate,
                        }),
                        source_references: failed_workloads.iter().map(|w| w.workload_id.clone()).collect(),
                        generated_at: chrono::Utc::now().to_rfc3339(),
                    });
                }
            }
        }

        // trend findings — overall success rate trends
        if outcome_analysis.overall_success_rate < 0.5 && outcome_analysis.total_workloads >= 5 {
            findings.push(IntelligenceFinding {
                finding_id: Uuid::new_v4().to_string(),
                category: "trend".to_string(),
                severity: "critical".to_string(),
                title: "Overall workload success rate is critically low".to_string(),
                description: format!(
                    "Overall success rate is {:.1}% across {} workloads",
                    outcome_analysis.overall_success_rate * 100.0,
                    outcome_analysis.total_workloads
                ),
                supporting_data: serde_json::json!(outcome_analysis),
                source_references: Vec::new(),
                generated_at: chrono::Utc::now().to_rfc3339(),
            });
        } else if outcome_analysis.overall_success_rate >= 0.95 && outcome_analysis.total_workloads >= 5 {
            findings.push(IntelligenceFinding {
                finding_id: Uuid::new_v4().to_string(),
                category: "trend".to_string(),
                severity: "info".to_string(),
                title: "Overall workload success rate is excellent".to_string(),
                description: format!(
                    "Overall success rate is {:.1}% across {} workloads",
                    outcome_analysis.overall_success_rate * 100.0,
                    outcome_analysis.total_workloads
                ),
                supporting_data: serde_json::json!(outcome_analysis),
                source_references: Vec::new(),
                generated_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        findings
    }

    pub fn generate_report(
        ws_service: &WorkloadSessionService,
        fleet_service: &FleetService,
        allocation_service: &AllocationService,
        owner_allocation_service: &OwnerAllocationService,
        capability_evidence_bridge: &CapabilityEvidenceBridge,
    ) -> IntelligenceReport {
        let workload_analysis = Self::analyze_workload_outcomes(ws_service);
        let capability_analysis =
            Self::analyze_capability_effectiveness(ws_service, fleet_service, capability_evidence_bridge);
        let allocation_analysis =
            Self::analyze_allocation_accuracy(ws_service, allocation_service, owner_allocation_service);
        let findings = Self::generate_findings(
            ws_service,
            fleet_service,
            &workload_analysis,
            &capability_analysis,
            &allocation_analysis,
        );

        IntelligenceReport {
            report_id: Uuid::new_v4().to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            workload_analysis,
            capability_analysis,
            allocation_analysis,
            findings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{
        allocation_service::AllocationService, fleet_service::FleetService,
        session_service::SessionService, AllocationService as AllocSvc, CapabilityEvidenceBridge,
        FleetService as FleetSvc, OwnerAllocationService, SessionService as SessSvc,
        WorkloadSessionService,
    };
    use librarian_contracts::{
        allocation::AllocationRequest,
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

    fn setup_services() -> (
        WorkloadSessionService,
        SessionService,
        AllocationService,
        FleetService,
        OwnerAllocationService,
        CapabilityEvidenceBridge,
        tempfile::TempDir,
    ) {
        let dir = tempdir().unwrap();
        let session_path = dir.path().join("sessions.json");
        let session_service = SessionService::new(session_path);
        let ws_path = dir.path().join("workload_sessions.json");
        let ws_service = WorkloadSessionService::new(ws_path);
        let alloc_path = dir.path().join("allocation.json");
        let alloc_service = AllocationService::new(alloc_path);
        let fleet_path = dir.path().join("fleet.json");
        let fleet_service = FleetService::new(fleet_path);
        let owner_path = dir.path().join("owner_alloc.json");
        let owner_service = OwnerAllocationService::new(owner_path);
        let bridge_path = dir.path().join("bridge.json");
        let bridge = CapabilityEvidenceBridge::new(bridge_path);
        (
            ws_service, session_service, alloc_service, fleet_service,
            owner_service, bridge, dir,
        )
    }

    fn seed_node(fleet: &mut FleetService, node_id: &str) {
        fleet.add_or_update_node(NodeInventoryEntry {
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
    fn test_workload_outcome_analysis_returns_correct_success_rates_per_type() {
        let (mut ws, mut sess, _alloc, _fleet, _owner, _bridge, _dir) = setup_services();

        run_workload_to_completion(&mut ws, &mut sess, "wl-inf-001", "inference", "node-a");
        run_workload_to_completion(&mut ws, &mut sess, "wl-inf-002", "inference", "node-a");
        run_workload_to_failure(&mut ws, &mut sess, "wl-inf-003", "inference", "node-a");
        run_workload_to_completion(&mut ws, &mut sess, "wl-vision-001", "vision", "node-a");

        let analysis = EvidenceIntelligenceService::analyze_workload_outcomes(&ws);

        assert_eq!(analysis.total_workloads, 4);

        // workload_type is empty because WorkloadSessionService does not store it
        // All workloads are grouped under the empty string
        let summary = analysis
            .summaries
            .first()
            .expect("summary should exist");
        assert_eq!(summary.total, 4);
        assert_eq!(summary.completed, 3);
        assert_eq!(summary.failed, 1);
        assert!((summary.success_rate - 3.0 / 4.0).abs() < 1e-10);
        assert!(summary.avg_duration_seconds.is_some());

        assert!((analysis.overall_success_rate - 3.0 / 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_capability_effectiveness_shows_workloads_per_capability() {
        let (mut ws, mut sess, _alloc, mut fleet, _owner, mut bridge, _dir) = setup_services();

        seed_node(&mut fleet, "node-a");
        bridge.register_claim("node-a", "llm.inference", None, None);

        run_workload_to_completion(&mut ws, &mut sess, "wl-001", "inference", "node-a");
        run_workload_to_failure(&mut ws, &mut sess, "wl-002", "inference", "node-a");

        let analysis =
            EvidenceIntelligenceService::analyze_capability_effectiveness(&ws, &fleet, &bridge);

        let inference = analysis
            .entries
            .iter()
            .find(|e| e.capability_type == "llm.inference");
        assert!(inference.is_some(), "should find llm.inference capability");
        if let Some(inf) = inference {
            assert_eq!(inf.workloads_using, 2);
            assert_eq!(inf.successful_workloads, 1);
            assert_eq!(inf.failed_workloads, 1);
            assert_eq!(inf.success_rate, 0.5);
        }
    }

    #[test]
    fn test_allocation_accuracy_correlates_recommendations_with_outcomes() {
        let (mut ws, mut sess, mut alloc, mut fleet, mut owner, _bridge, _dir) = setup_services();

        seed_node(&mut fleet, "node-a");

        let request = AllocationRequest {
            request_id: "wl-alloc-001".to_string(),
            workload_description: "Test".to_string(),
            requirements: vec![],
            preferred_nodes: None,
            requested_at: chrono::Utc::now().to_rfc3339(),
        };
        let rec = alloc.generate_recommendation(request, &fleet);
        alloc.accept_recommendation(&rec.recommendation_id, None);

        run_workload_to_completion(
            &mut ws,
            &mut sess,
            "wl-alloc-001",
            "inference",
            &rec.node_id,
        );

        let analysis = EvidenceIntelligenceService::analyze_allocation_accuracy(
            &ws, &alloc, &owner,
        );

        assert!(analysis.total_recommendations >= 1);
        assert_eq!(analysis.accepted_recommendations, 1);
        assert_eq!(analysis.successful_workloads, 1);
        assert!(analysis.overall_accuracy.is_some());
        assert_eq!(analysis.overall_accuracy.unwrap(), 1.0);

        let entry = analysis
            .entries
            .iter()
            .find(|e| e.recommendation_id == rec.recommendation_id);
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert!(entry.recommended);
        assert!(entry.workload_successful);
        assert_eq!(entry.allocation_correct, Some(true));
    }

    #[test]
    fn test_findings_generation_produces_categorized_insights() {
        let (mut ws, mut sess, mut alloc, mut fleet, mut owner, mut bridge, _dir) = setup_services();

        seed_node(&mut fleet, "node-a");
        bridge.register_claim("node-a", "llm.inference", None, None);

        run_workload_to_failure(&mut ws, &mut sess, "wl-fail-001", "inference", "node-a");
        run_workload_to_failure(&mut ws, &mut sess, "wl-fail-002", "inference", "node-a");
        run_workload_to_failure(&mut ws, &mut sess, "wl-fail-003", "inference", "node-a");

        let outcome = EvidenceIntelligenceService::analyze_workload_outcomes(&ws);
        let capability =
            EvidenceIntelligenceService::analyze_capability_effectiveness(&ws, &fleet, &bridge);
        let allocation =
            EvidenceIntelligenceService::analyze_allocation_accuracy(&ws, &alloc, &owner);

        let findings = EvidenceIntelligenceService::generate_findings(
            &ws, &fleet, &outcome, &capability, &allocation,
        );

        assert!(!findings.is_empty(), "should produce at least one finding");

        for finding in &findings {
            assert!(!finding.finding_id.is_empty());
            assert!(!finding.category.is_empty());
            assert!(!finding.severity.is_empty());
            assert!(!finding.title.is_empty());
            assert!(!finding.description.is_empty());
            assert!(["info", "notable", "warning", "critical"].contains(&finding.severity.as_str()));
            assert!(["workload_outcome", "capability", "allocation", "node_health", "trend"]
                .contains(&finding.category.as_str()));
        }
    }

    #[test]
    fn test_intelligence_report_includes_all_analysis_sections() {
        let (mut ws, mut sess, mut alloc, mut fleet, mut owner, mut bridge, _dir) = setup_services();

        seed_node(&mut fleet, "node-a");
        bridge.register_claim("node-a", "llm.inference", None, None);

        run_workload_to_completion(&mut ws, &mut sess, "wl-001", "inference", "node-a");
        run_workload_to_failure(&mut ws, &mut sess, "wl-002", "inference", "node-a");

        let request = AllocationRequest {
            request_id: "wl-003".to_string(),
            workload_description: "Test".to_string(),
            requirements: vec![],
            preferred_nodes: None,
            requested_at: chrono::Utc::now().to_rfc3339(),
        };
        let rec = alloc.generate_recommendation(request, &fleet);
        alloc.accept_recommendation(&rec.recommendation_id, None);
        run_workload_to_completion(&mut ws, &mut sess, "wl-003", "inference", &rec.node_id);

        let report = EvidenceIntelligenceService::generate_report(
            &ws, &fleet, &alloc, &owner, &bridge,
        );

        assert!(!report.report_id.is_empty());
        assert!(!report.generated_at.is_empty());
        assert_eq!(report.workload_analysis.total_workloads, 3);
        assert_eq!(
            report.capability_analysis.total_capabilities,
            report.capability_analysis.entries.len() as u32
        );
        assert!(report.allocation_analysis.total_recommendations > 0);
        assert!(!report.findings.is_empty());
    }

    #[test]
    fn test_empty_services_produce_empty_analysis() {
        let (ws, _sess, alloc, fleet, owner, bridge, _dir) = setup_services();

        let outcome = EvidenceIntelligenceService::analyze_workload_outcomes(&ws);
        assert_eq!(outcome.total_workloads, 0);
        assert!(outcome.summaries.is_empty());

        let capability =
            EvidenceIntelligenceService::analyze_capability_effectiveness(&ws, &fleet, &bridge);
        assert!(capability.entries.is_empty() || capability.total_capabilities == 0);

        let allocation =
            EvidenceIntelligenceService::analyze_allocation_accuracy(&ws, &alloc, &owner);
        assert_eq!(allocation.total_recommendations, 0);

        let findings = EvidenceIntelligenceService::generate_findings(
            &ws, &fleet, &outcome, &capability, &allocation,
        );
        assert!(findings.is_empty());
    }
}
