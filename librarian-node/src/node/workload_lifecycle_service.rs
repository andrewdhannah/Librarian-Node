use librarian_contracts::workload_lifecycle::{
    WorkloadDecisionChain, WorkloadHistoryQuery, WorkloadHistoryResult, WorkloadInventory,
    WorkloadReview, WorkloadSummary, WorkloadTimeline, WorkloadTimelineEntry,
};
use uuid::Uuid;

use super::WorkloadSessionService;

pub struct WorkloadLifecycleService;

impl WorkloadLifecycleService {
    pub fn get_inventory(ws_service: &WorkloadSessionService) -> WorkloadInventory {
        let sessions = ws_service.list_workload_sessions();
        let receipts = ws_service.get_receipts();
        let total = sessions.len() as u32;

        let mut active = 0u32;
        let mut completed = 0u32;
        let mut failed = 0u32;
        let mut pending = 0u32;
        let mut cancelled = 0u32;

        let workloads: Vec<WorkloadSummary> = sessions
            .into_iter()
            .map(|s| {
                let state = s.state.clone();
                match state.as_str() {
                    "active" => active += 1,
                    "completed" => completed += 1,
                    "failed" => failed += 1,
                    "created" => pending += 1,
                    "cancelled" => cancelled += 1,
                    _ => {}
                }

                let receipt = receipts.iter().find(|r| r.workload_session_id == s.workload_session_id);
                let duration_seconds = receipt.and_then(|r| {
                    let created = chrono::DateTime::parse_from_rfc3339(&s.created_at).ok()?;
                    let completed = chrono::DateTime::parse_from_rfc3339(
                        r.completed_at.as_deref()?,
                    )
                    .ok()?;
                    Some((completed - created).num_seconds() as u64)
                });
                let has_receipt = receipt.is_some();
                let operations_executed = receipt.map(|r| r.operations_executed);
                let evidence_count = receipt.map(|r| r.evidence_ids.len() as u32);

                WorkloadSummary {
                    workload_id: s.workload_id,
                    workload_type: String::new(),
                    description: String::new(),
                    state,
                    node_id: s.node_id.clone(),
                    node_name: s.node_id.clone(),
                    session_id: s.session_id,
                    created_at: s.created_at,
                    completed_at: s.completed_at,
                    duration_seconds,
                    operations_executed,
                    evidence_count,
                    has_receipt,
                }
            })
            .collect();

        WorkloadInventory {
            total,
            active,
            completed,
            failed,
            pending,
            cancelled,
            workloads,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn get_timeline(
        ws_service: &WorkloadSessionService,
        workload_id: &str,
    ) -> Option<WorkloadTimeline> {
        let session = ws_service.list_workload_sessions()
            .into_iter()
            .find(|s| s.workload_id == workload_id)?;

        let receipts = ws_service.get_receipts();
        let mut entries: Vec<WorkloadTimelineEntry> = Vec::new();

        entries.push(WorkloadTimelineEntry {
            event_id: Uuid::new_v4().to_string(),
            workload_id: workload_id.to_string(),
            event_type: "created".to_string(),
            timestamp: session.created_at.clone(),
            details: Some("Workload session created".to_string()),
            associated_receipt_id: None,
        });

        match session.state.as_str() {
            "active" | "completed" | "failed" | "cancelled" => {
                let details = match session.state.as_str() {
                    "active" => Some("Workload session activated".to_string()),
                    "completed" => Some("Workload session completed".to_string()),
                    "failed" => Some("Workload session failed".to_string()),
                    "cancelled" => Some("Workload session cancelled".to_string()),
                    _ => None,
                };
                entries.push(WorkloadTimelineEntry {
                    event_id: Uuid::new_v4().to_string(),
                    workload_id: workload_id.to_string(),
                    event_type: session.state.clone(),
                    timestamp: session
                        .completed_at
                        .clone()
                        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
                    details,
                    associated_receipt_id: session.receipt_id.clone(),
                });
            }
            _ => {}
        }

        if let Some(receipt_id) = &session.receipt_id {
            if let Some(receipt) = receipts.iter().find(|r| &r.receipt_id == receipt_id) {
                entries.push(WorkloadTimelineEntry {
                    event_id: Uuid::new_v4().to_string(),
                    workload_id: workload_id.to_string(),
                    event_type: "receipt_generated".to_string(),
                    timestamp: receipt
                        .completed_at
                        .clone()
                        .unwrap_or_else(|| receipt.created_at.clone()),
                    details: Some(format!(
                        "Receipt generated with {} operations, {} evidence items",
                        receipt.operations_executed,
                        receipt.evidence_ids.len()
                    )),
                    associated_receipt_id: Some(receipt.receipt_id.clone()),
                });
            }
        }

        entries.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Some(WorkloadTimeline {
            workload_id: workload_id.to_string(),
            node_id: session.node_id.clone(),
            session_id: session.session_id.clone(),
            entries,
            generated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub fn query_history(
        ws_service: &WorkloadSessionService,
        query: WorkloadHistoryQuery,
    ) -> WorkloadHistoryResult {
        let inventory = Self::get_inventory(ws_service);
        let mut filtered: Vec<WorkloadSummary> = inventory.workloads;

        if let Some(ref node_id) = query.node_id {
            filtered.retain(|w| w.node_id == *node_id);
        }
        if let Some(ref state) = query.state {
            filtered.retain(|w| w.state == *state);
        }
        if let Some(ref workload_type) = query.workload_type {
            filtered.retain(|w| w.workload_type == *workload_type);
        }
        if let Some(ref from) = query.from_timestamp {
            filtered.retain(|w| w.created_at >= *from);
        }
        if let Some(ref to) = query.to_timestamp {
            filtered.retain(|w| w.created_at <= *to);
        }

        let total = filtered.len() as u32;
        if let Some(limit) = query.limit {
            let limit = limit as usize;
            if filtered.len() > limit {
                filtered.truncate(limit);
            }
        }

        WorkloadHistoryResult {
            total,
            returned: filtered.len() as u32,
            workloads: filtered,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn get_review(
        ws_service: &WorkloadSessionService,
        workload_id: &str,
    ) -> Option<WorkloadReview> {
        let timeline = Self::get_timeline(ws_service, workload_id)?;
        let session = ws_service.list_workload_sessions()
            .into_iter()
            .find(|s| s.workload_id == workload_id)?;

        let receipts = ws_service.get_receipts();
        let receipt = receipts.iter().find(|r| r.workload_session_id == session.workload_session_id);
        let evidence_count = receipt.map(|r| r.evidence_ids.len() as u32);

        let duration_seconds = receipt.and_then(|r| {
            let created = chrono::DateTime::parse_from_rfc3339(&session.created_at).ok()?;
            let completed = chrono::DateTime::parse_from_rfc3339(
                r.completed_at.as_deref()?,
            )
            .ok()?;
            Some((completed - created).num_seconds() as u64)
        });

        let link = ws_service.get_link(workload_id);
        let decision_chain = link.map(|l| WorkloadDecisionChain {
            allocation_recommendation_id: Some(l.allocation_recommendation_id).filter(|s| !s.is_empty()),
            allocation_decision_id: Some(l.allocation_decision_id).filter(|s| !s.is_empty()),
            owner_decision_summary: Some(format!(
                "Linked via allocation receipt {}",
                l.allocation_receipt_id
            )),
        });

        Some(WorkloadReview {
            workload_id: workload_id.to_string(),
            workload_type: String::new(),
            description: String::new(),
            state: session.state.clone(),
            node_id: session.node_id.clone(),
            created_at: session.created_at.clone(),
            duration_seconds,
            evidence_count,
            timeline: Some(timeline),
            decision_chain,
        })
    }

    pub fn get_active_count(ws_service: &WorkloadSessionService) -> u32 {
        ws_service
            .list_workload_sessions()
            .iter()
            .filter(|s| s.state == "active")
            .count() as u32
    }

    pub fn get_recent_completed(
        ws_service: &WorkloadSessionService,
        limit: usize,
    ) -> Vec<WorkloadSummary> {
        let inventory = Self::get_inventory(ws_service);
        let mut completed: Vec<WorkloadSummary> = inventory
            .workloads
            .into_iter()
            .filter(|w| w.state == "completed")
            .collect();
        completed.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        completed.truncate(limit);
        completed
    }

    pub fn get_failed_workloads(ws_service: &WorkloadSessionService) -> Vec<WorkloadSummary> {
        let inventory = Self::get_inventory(ws_service);
        inventory
            .workloads
            .into_iter()
            .filter(|w| w.state == "failed")
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::workload_session_service::WorkloadSessionService;
    use crate::node::SessionService;
    use librarian_contracts::workload_session::WorkloadDescriptor;
    use tempfile::tempdir;

    fn test_workload(id: &str) -> WorkloadDescriptor {
        WorkloadDescriptor {
            workload_id: id.to_string(),
            workload_type: "inference".to_string(),
            description: format!("Test workload {}", id),
            requirements: Some(vec!["llm.inference".to_string()]),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn setup() -> (WorkloadSessionService, SessionService, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let session_path = dir.path().join("sessions.json");
        let session_service = SessionService::new(session_path);
        let ws_path = dir.path().join("workload_sessions.json");
        let ws_service = WorkloadSessionService::new(ws_path);
        (ws_service, session_service, dir)
    }

    #[test]
    fn test_inventory_returns_correct_state_distribution() {
        let (mut ws, mut sess, _dir) = setup();

        // Create, activate, and complete one workload
        let ws1 = ws
            .create_workload_session(test_workload("wl-complete"), "r1", "node-a", None, None, &mut sess, None)
            .unwrap();
        ws.activate_workload_session(&ws1.workload_session_id, &mut sess)
            .unwrap();
        ws.complete_workload_session(&ws1.workload_session_id, 5, vec!["e1".to_string(), "e2".to_string()], &mut sess)
            .unwrap();

        // Create, activate, and fail one workload
        let ws2 = ws
            .create_workload_session(test_workload("wl-fail"), "r2", "node-a", None, None, &mut sess, None)
            .unwrap();
        ws.activate_workload_session(&ws2.workload_session_id, &mut sess)
            .unwrap();
        ws.fail_workload_session(&ws2.workload_session_id, "error", &mut sess)
            .unwrap();

        // Create but don't activate (stays as "created" → counted as pending)
        let _ws3 = ws
            .create_workload_session(test_workload("wl-pending"), "r3", "node-b", None, None, &mut sess, None)
            .unwrap();

        // Create, activate (stays active)
        let ws4 = ws
            .create_workload_session(test_workload("wl-active"), "r4", "node-b", None, None, &mut sess, None)
            .unwrap();
        ws.activate_workload_session(&ws4.workload_session_id, &mut sess)
            .unwrap();

        let inventory = WorkloadLifecycleService::get_inventory(&ws);

        assert_eq!(inventory.total, 4);
        assert_eq!(inventory.active, 1);
        assert_eq!(inventory.completed, 1);
        assert_eq!(inventory.failed, 1);
        assert_eq!(inventory.pending, 1); // "created" maps to pending
        assert_eq!(inventory.cancelled, 0);
        assert_eq!(inventory.workloads.len(), 4);
        assert!(!inventory.generated_at.is_empty());
    }

    #[test]
    fn test_timeline_shows_lifecycle_events_in_order() {
        let (mut ws, mut sess, _dir) = setup();

        let ws1 = ws
            .create_workload_session(test_workload("wl-timeline"), "r1", "node-a", None, None, &mut sess, None)
            .unwrap();
        ws.activate_workload_session(&ws1.workload_session_id, &mut sess)
            .unwrap();
        ws.complete_workload_session(&ws1.workload_session_id, 3, vec!["e1".to_string()], &mut sess)
            .unwrap();

        let timeline = WorkloadLifecycleService::get_timeline(&ws, "wl-timeline")
            .expect("timeline should exist");

        assert_eq!(timeline.workload_id, "wl-timeline");
        assert_eq!(timeline.node_id, "node-a");
        assert_eq!(timeline.session_id, ws1.session_id);

        // Should have at least 3 entries: created, completed, receipt_generated
        assert!(
            timeline.entries.len() >= 3,
            "Expected at least 3 entries, got {}",
            timeline.entries.len()
        );

        // Events should be in chronological order
        for i in 1..timeline.entries.len() {
            assert!(
                timeline.entries[i - 1].timestamp <= timeline.entries[i].timestamp,
                "Events not in chronological order"
            );
        }

        // Check specific event types exist
        let event_types: Vec<&str> = timeline.entries.iter().map(|e| e.event_type.as_str()).collect();
        assert!(event_types.contains(&"created"));
        assert!(event_types.contains(&"receipt_generated"));
    }

    #[test]
    fn test_history_query_filters_by_state_and_time_range() {
        let (mut ws, mut sess, _dir) = setup();

        let ws1 = ws
            .create_workload_session(test_workload("wl-001"), "r1", "node-a", None, None, &mut sess, None)
            .unwrap();
        ws.activate_workload_session(&ws1.workload_session_id, &mut sess)
            .unwrap();
        ws.complete_workload_session(&ws1.workload_session_id, 2, vec![], &mut sess)
            .unwrap();

        let ws2 = ws
            .create_workload_session(test_workload("wl-002"), "r2", "node-b", None, None, &mut sess, None)
            .unwrap();
        ws.activate_workload_session(&ws2.workload_session_id, &mut sess)
            .unwrap();
        ws.fail_workload_session(&ws2.workload_session_id, "error", &mut sess)
            .unwrap();

        let _ws3 = ws
            .create_workload_session(test_workload("wl-003"), "r3", "node-a", None, None, &mut sess, None)
            .unwrap();

        // Filter by state = "completed"
        let query = WorkloadHistoryQuery {
            node_id: None,
            state: Some("completed".to_string()),
            workload_type: None,
            from_timestamp: None,
            to_timestamp: None,
            limit: None,
        };
        let result = WorkloadLifecycleService::query_history(&ws, query);
        assert_eq!(result.total, 1);
        assert_eq!(result.returned, 1);
        assert_eq!(result.workloads[0].workload_id, "wl-001");

        // Filter by node_id
        let query = WorkloadHistoryQuery {
            node_id: Some("node-b".to_string()),
            state: None,
            workload_type: None,
            from_timestamp: None,
            to_timestamp: None,
            limit: None,
        };
        let result = WorkloadLifecycleService::query_history(&ws, query);
        assert_eq!(result.total, 1);
        assert_eq!(result.workloads[0].workload_id, "wl-002");

        // Filter with limit
        let query = WorkloadHistoryQuery {
            node_id: None,
            state: None,
            workload_type: None,
            from_timestamp: None,
            to_timestamp: None,
            limit: Some(1),
        };
        let result = WorkloadLifecycleService::query_history(&ws, query);
        assert_eq!(result.returned, 1);
        assert_eq!(result.total, 3);
    }

    #[test]
    fn test_review_includes_decision_chain() {
        let (mut ws, mut sess, _dir) = setup();

        let ws1 = ws
            .create_workload_session(
                test_workload("wl-review"),
                "receipt-decision",
                "node-a",
                Some("rec-001".to_string()),
                Some("dec-001".to_string()),
                &mut sess,
                None,
            )
            .unwrap();
        ws.activate_workload_session(&ws1.workload_session_id, &mut sess)
            .unwrap();
        ws.complete_workload_session(&ws1.workload_session_id, 4, vec!["e1".to_string(), "e2".to_string()], &mut sess)
            .unwrap();

        let review = WorkloadLifecycleService::get_review(&ws, "wl-review")
            .expect("review should exist");

        assert_eq!(review.workload_id, "wl-review");
        assert_eq!(review.state, "completed");
        assert_eq!(review.node_id, "node-a");
        assert!(review.duration_seconds.is_some());
        assert_eq!(review.evidence_count, Some(2));

        // Decision chain should be present
        let chain = review
            .decision_chain
            .expect("decision chain should exist for linked workload");
        assert_eq!(
            chain.allocation_recommendation_id,
            Some("rec-001".to_string())
        );
        assert_eq!(
            chain.allocation_decision_id,
            Some("dec-001".to_string())
        );
        assert!(chain.owner_decision_summary.is_some());
        assert!(chain.owner_decision_summary.unwrap().contains("receipt-decision"));

        // Timeline should be included
        let timeline = review.timeline.expect("timeline should be included");
        assert_eq!(timeline.workload_id, "wl-review");
    }

    #[test]
    fn test_active_and_failed_lists() {
        let (mut ws, mut sess, _dir) = setup();

        // Active workload
        let ws1 = ws
            .create_workload_session(test_workload("wl-active"), "r1", "node-a", None, None, &mut sess, None)
            .unwrap();
        ws.activate_workload_session(&ws1.workload_session_id, &mut sess)
            .unwrap();

        // Failed workload
        let ws2 = ws
            .create_workload_session(test_workload("wl-failed"), "r2", "node-a", None, None, &mut sess, None)
            .unwrap();
        ws.activate_workload_session(&ws2.workload_session_id, &mut sess)
            .unwrap();
        ws.fail_workload_session(&ws2.workload_session_id, "error", &mut sess)
            .unwrap();

        // Completed workload (should not appear in active or failed)
        let ws3 = ws
            .create_workload_session(test_workload("wl-completed"), "r3", "node-a", None, None, &mut sess, None)
            .unwrap();
        ws.activate_workload_session(&ws3.workload_session_id, &mut sess)
            .unwrap();
        ws.complete_workload_session(&ws3.workload_session_id, 1, vec![], &mut sess)
            .unwrap();

        let active_count = WorkloadLifecycleService::get_active_count(&ws);
        assert_eq!(active_count, 1);

        let failed = WorkloadLifecycleService::get_failed_workloads(&ws);
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].workload_id, "wl-failed");

        let recent = WorkloadLifecycleService::get_recent_completed(&ws, 5);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].workload_id, "wl-completed");
    }

    #[test]
    fn test_review_without_allocation_ids() {
        let (mut ws, mut sess, _dir) = setup();

        let ws1 = ws
            .create_workload_session(test_workload("wl-no-link"), "r1", "node-a", None, None, &mut sess, None)
            .unwrap();
        ws.activate_workload_session(&ws1.workload_session_id, &mut sess)
            .unwrap();
        ws.complete_workload_session(&ws1.workload_session_id, 0, vec![], &mut sess)
            .unwrap();

        let review = WorkloadLifecycleService::get_review(&ws, "wl-no-link")
            .expect("review should exist");

        // Decision chain exists (link is always created) but IDs are None
        let chain = review.decision_chain.expect("decision chain should exist");
        assert!(chain.allocation_recommendation_id.is_none());
        assert!(chain.allocation_decision_id.is_none());
        assert!(review.timeline.is_some());
    }

    #[test]
    fn test_timeline_for_nonexistent_workload() {
        let (ws, _sess, _dir) = setup();
        let timeline = WorkloadLifecycleService::get_timeline(&ws, "nonexistent");
        assert!(timeline.is_none());
    }

    #[test]
    fn test_review_for_nonexistent_workload() {
        let (ws, _sess, _dir) = setup();
        let review = WorkloadLifecycleService::get_review(&ws, "nonexistent");
        assert!(review.is_none());
    }

    #[test]
    fn test_timeline_for_failed_workload_has_failed_event() {
        let (mut ws, mut sess, _dir) = setup();

        let ws1 = ws
            .create_workload_session(test_workload("wl-fail-timeline"), "r1", "node-a", None, None, &mut sess, None)
            .unwrap();
        ws.activate_workload_session(&ws1.workload_session_id, &mut sess)
            .unwrap();
        ws.fail_workload_session(&ws1.workload_session_id, "error", &mut sess)
            .unwrap();

        let timeline = WorkloadLifecycleService::get_timeline(&ws, "wl-fail-timeline")
            .expect("timeline should exist");

        let event_types: Vec<&str> = timeline.entries.iter().map(|e| e.event_type.as_str()).collect();
        assert!(event_types.contains(&"failed"));
        assert!(event_types.contains(&"receipt_generated"));
    }
}
