//! # Runtime Supervisor
//!
//! Cross-platform runtime supervisor. Maps process lifecycle events to
//! governance primitives (ResidencyState, Evidence, Receipt, Custody).
//!
//! The supervisor is platform-agnostic. Platform-specific process management
//! is provided by the `RuntimeAdapter` trait.

use anyhow::Result;
use librarian_contracts::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

use crate::governance::db::GovernanceDb;
use crate::governance::evidence::EvidenceGenerator;
use crate::governance::receipts::ReceiptGenerator;

use super::adapter::{ProcessEvent, ProcessState, RuntimeAdapter};

/// A governed runtime instance — tracks a supervised process through governance.
#[derive(Debug, Clone)]
pub struct GovernedInstance {
    /// Unique instance identifier.
    pub instance_id: String,
    /// Component identifier (e.g., "model-phi-4", "router").
    pub component_id: String,
    /// Current residency state.
    pub residency: ResidencyState,
    /// Current adapter-reported state.
    pub adapter_state: ProcessState,
    /// ISO 8601 timestamp of last state change.
    pub last_event_at: String,
}

/// The cross-platform runtime supervisor.
pub struct RuntimeSupervisor {
    db: GovernanceDb,
    evidence_gen: EvidenceGenerator,
    receipt_gen: ReceiptGenerator,
    instances: std::sync::Mutex<HashMap<String, GovernedInstance>>,
}

impl RuntimeSupervisor {
    /// Create a new runtime supervisor.
    pub fn new(db: GovernanceDb) -> Self {
        let evidence_gen = EvidenceGenerator::new(db.clone(), "runtime-supervisor");
        let receipt_gen = ReceiptGenerator::new(db.clone());
        Self {
            db,
            evidence_gen,
            receipt_gen,
            instances: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Register a new component for supervision. Does not start it.
    pub fn register(&self, component_id: &str) -> GovernedInstance {
        let mut instances = self.instances.lock().unwrap();
        let instance = GovernedInstance {
            instance_id: format!("inst-{}", Uuid::new_v4()),
            component_id: component_id.to_string(),
            residency: ResidencyState::Released,
            adapter_state: ProcessState::Stopped,
            last_event_at: chrono::Utc::now().to_rfc3339(),
        };
        instances.insert(component_id.to_string(), instance.clone());
        instance
    }

    /// Process an event from the runtime adapter. Maps to governance primitives.
    pub fn process_event(&self, component_id: &str, event: ProcessEvent) -> Result<Option<GovernedInstance>> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut instances = self.instances.lock().unwrap();

        let instance = match instances.get_mut(component_id) {
            Some(i) => i,
            None => return Ok(None),
        };

        let (new_residency, transition_type) = Self::map_event_to_residency(&event);
        let old_residency = instance.residency;

        // Skip if no state change
        if new_residency == old_residency && event != ProcessEvent::HealthCheckPassed {
            return Ok(Some(instance.clone()));
        }

        instance.residency = new_residency;
        instance.adapter_state = event.to_process_state();
        instance.last_event_at = now.clone();

        // Generate evidence for the transition
        let evidence_payload = serde_json::json!({
            "component_id": component_id,
            "event": format!("{:?}", event),
            "from_residency": format!("{:?}", old_residency),
            "to_residency": format!("{:?}", new_residency),
            "adapter_state": format!("{:?}", instance.adapter_state),
        });

        let evidence = self.evidence_gen.generate(
            EvidenceCategory::ContractValidation,
            &format!("Runtime event: {:?} — {:?} → {:?}", event, old_residency, new_residency),
            &evidence_payload,
        )?;

        // Record custody event for the residency transition
        let custody_action = match &event {
            ProcessEvent::Started => CustodyAction::Claim,
            ProcessEvent::Stopped | ProcessEvent::Crashed => CustodyAction::Release,
            _ => CustodyAction::Validate,
        };

        let custody_event = CustodyEvent {
            event_id: format!("rt-{}-{}", component_id, Uuid::new_v4()),
            project_id: "runtime-supervision".into(),
            mcp_session_id: String::new(),
            node_id: "runtime-supervisor".into(),
            window_id: None,
            work_packet_id: None,
            tool_name: "runtime_supervisor".into(),
            authority_role: CustodyAuthorityRole::System,
            document_reference: format!("runtime://{}", component_id),
            custody_action,
            previous_custody_mode: Some(Self::residency_to_custody_mode(old_residency)),
            resulting_custody_mode: Some(Self::residency_to_custody_mode(new_residency)),
            mutation_allowance: Some(MutationAllowance::ReadOnly),
            decision_reference: None,
            provenance_receipt: None,
            refusal_reason: None,
            target_project_id: None,
            target_session_id: None,
            target_node_id: None,
            timestamp: now.clone(),
        };
        self.db.record_custody_event(&custody_event)?;

        // Generate receipt for lifecycle boundaries (start/stop/crash)
        if matches!(transition_type, TransitionType::LifecycleBoundary) {
            let action = match &event {
                ProcessEvent::Started => "runtime_start",
                ProcessEvent::Stopped => "runtime_stop",
                ProcessEvent::Crashed => "runtime_crash",
                _ => "runtime_state_change",
            };
            self.receipt_gen.equivalence_check(
                "runtime-spec",
                component_id,
                "PASS",
                1,
                0,
                &evidence.record_id,
            )?;
        }

        Ok(Some(instance.clone()))
    }

    /// Get the current state of a supervised instance.
    pub fn get_instance(&self, component_id: &str) -> Option<GovernedInstance> {
        let instances = self.instances.lock().unwrap();
        instances.get(component_id).cloned()
    }

    /// Get all supervised instances.
    pub fn all_instances(&self) -> Vec<GovernedInstance> {
        let instances = self.instances.lock().unwrap();
        instances.values().cloned().collect()
    }

    /// Get the count of active (resource-occupying) instances.
    pub fn active_count(&self) -> usize {
        let instances = self.instances.lock().unwrap();
        instances.values().filter(|i| i.residency.is_occupying_resources()).count()
    }

    // ========================================================================
    // Private helpers
    // ========================================================================

    /// Map a process event to a ResidencyState.
    fn map_event_to_residency(event: &ProcessEvent) -> (ResidencyState, TransitionType) {
        match event {
            ProcessEvent::StartRequested => (ResidencyState::Requested, TransitionType::Normal),
            ProcessEvent::Started => (ResidencyState::Active, TransitionType::LifecycleBoundary),
            ProcessEvent::HealthCheckPassed => (ResidencyState::Active, TransitionType::Normal),
            ProcessEvent::HealthCheckFailed => (ResidencyState::Active, TransitionType::Normal),
            ProcessEvent::Degraded => (ResidencyState::Active, TransitionType::Normal),
            ProcessEvent::StopRequested => (ResidencyState::Releasing, TransitionType::Normal),
            ProcessEvent::Stopped => (ResidencyState::Released, TransitionType::LifecycleBoundary),
            ProcessEvent::Crashed => (ResidencyState::Failed, TransitionType::LifecycleBoundary),
            ProcessEvent::Blocked => (ResidencyState::Blocked, TransitionType::LifecycleBoundary),
        }
    }

    /// Map ResidencyState to a CustodyMode for event recording.
    fn residency_to_custody_mode(state: ResidencyState) -> CustodyMode {
        match state {
            ResidencyState::Requested => CustodyMode::TransferPending,
            ResidencyState::Loading => CustodyMode::LocalWorkingCopy,
            ResidencyState::Loaded => CustodyMode::LocalCanonical,
            ResidencyState::Active => CustodyMode::LocalCanonical,
            ResidencyState::Releasing => CustodyMode::TransferPending,
            ResidencyState::Released => CustodyMode::AdvisoryContextOnly,
            ResidencyState::Failed => CustodyMode::AdvisoryContextOnly,
            ResidencyState::Blocked => CustodyMode::AdvisoryContextOnly,
        }
    }
}

/// Whether a transition is a lifecycle boundary (start/stop/crash).
#[derive(Debug, Clone, Copy, PartialEq)]
enum TransitionType {
    Normal,
    LifecycleBoundary,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::db::GovernanceDb;

    fn setup() -> RuntimeSupervisor {
        let db = GovernanceDb::open_in_memory().unwrap();
        RuntimeSupervisor::new(db)
    }

    #[test]
    fn test_register_instance() {
        let sup = setup();
        let inst = sup.register("model-phi-4");
        assert_eq!(inst.component_id, "model-phi-4");
        assert_eq!(inst.residency, ResidencyState::Released);
    }

    #[test]
    fn test_start_stop_lifecycle() {
        let sup = setup();
        sup.register("model-phi-4");

        // Request start
        let inst = sup.process_event("model-phi-4", ProcessEvent::StartRequested).unwrap().unwrap();
        assert_eq!(inst.residency, ResidencyState::Requested);

        // Started
        let inst = sup.process_event("model-phi-4", ProcessEvent::Started).unwrap().unwrap();
        assert_eq!(inst.residency, ResidencyState::Active);
        assert!(inst.residency.is_occupying_resources());

        // Health check
        let inst = sup.process_event("model-phi-4", ProcessEvent::HealthCheckPassed).unwrap().unwrap();
        assert_eq!(inst.residency, ResidencyState::Active);

        // Stop
        let inst = sup.process_event("model-phi-4", ProcessEvent::StopRequested).unwrap().unwrap();
        assert_eq!(inst.residency, ResidencyState::Releasing);

        let inst = sup.process_event("model-phi-4", ProcessEvent::Stopped).unwrap().unwrap();
        assert_eq!(inst.residency, ResidencyState::Released);
        assert!(!inst.residency.is_occupying_resources());
    }

    #[test]
    fn test_crash_detection() {
        let sup = setup();
        sup.register("model-phi-4");
        sup.process_event("model-phi-4", ProcessEvent::Started).unwrap();

        let inst = sup.process_event("model-phi-4", ProcessEvent::Crashed).unwrap().unwrap();
        assert_eq!(inst.residency, ResidencyState::Failed);
    }

    #[test]
    fn test_health_degradation() {
        let sup = setup();
        sup.register("model-phi-4");
        sup.process_event("model-phi-4", ProcessEvent::Started).unwrap();

        // Health check failures don't change state (stays Active)
        let inst = sup.process_event("model-phi-4", ProcessEvent::HealthCheckFailed).unwrap().unwrap();
        assert_eq!(inst.residency, ResidencyState::Active);

        // Degraded still active
        let inst = sup.process_event("model-phi-4", ProcessEvent::Degraded).unwrap().unwrap();
        assert_eq!(inst.residency, ResidencyState::Active);
    }

    #[test]
    fn test_active_count() {
        let sup = setup();
        sup.register("model-phi-4");
        sup.register("model-qwen");
        assert_eq!(sup.active_count(), 0);

        sup.process_event("model-phi-4", ProcessEvent::Started).unwrap();
        assert_eq!(sup.active_count(), 1);

        sup.process_event("model-qwen", ProcessEvent::Started).unwrap();
        assert_eq!(sup.active_count(), 2);

        sup.process_event("model-phi-4", ProcessEvent::Stopped).unwrap();
        assert_eq!(sup.active_count(), 1);
    }

    #[test]
    fn test_unregistered_component() {
        let sup = setup();
        let result = sup.process_event("nonexistent", ProcessEvent::Started).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_all_instances() {
        let sup = setup();
        sup.register("a");
        sup.register("b");
        sup.register("c");
        assert_eq!(sup.all_instances().len(), 3);
    }
}
