use std::collections::HashMap;
use std::time::Instant;

use librarian_contracts::operations::{
    ComponentHealth, DiagnosticCustodySummary, DiagnosticReport, DiagnosticSessionSummary,
    HealthSummary, NodeHealth, NodeOverview, ReceiptTypeCount,
};
use uuid::Uuid;

use super::{
    BootstrapService, CapabilityEvidenceBridge, CoreIntegrationService, CustodyService,
    NodeIdentityService, RegistrationService, SessionService,
};
use crate::db::RuntimeDatabase;

pub struct OperationsService {
    start_time: Instant,
}

impl OperationsService {
    pub fn new() -> Self {
        OperationsService {
            start_time: Instant::now(),
        }
    }

    pub fn get_health(
        &self,
        identity_service: &NodeIdentityService,
        registration_service: &RegistrationService,
        bridge: &CapabilityEvidenceBridge,
        session_service: &SessionService,
        bootstrap_service: &BootstrapService,
        custody_service: &CustodyService,
        core_integration_service: &CoreIntegrationService,
        db: &RuntimeDatabase,
    ) -> NodeHealth {
        let mut components = Vec::new();
        let mut overall = "healthy";

        // Identity
        let identity_ok = !identity_service.get_identity().node_id.is_empty();
        components.push(ComponentHealth {
            component: "identity".to_string(),
            status: if identity_ok { "healthy" } else { "unhealthy" } .to_string(),
            details: if identity_ok {
                None
            } else {
                Some("Node identity not loaded".to_string())
            },
        });
        if !identity_ok {
            overall = "unhealthy";
        }

        // Registration
        let reg_status = registration_service.get_record().registration_status.clone();
        let reg_healthy =
            reg_status == "registered" || reg_status == "registration_requested";
        if !reg_healthy && overall == "healthy" {
            overall = "degraded";
        }
        components.push(ComponentHealth {
            component: "registration".to_string(),
            status: if reg_healthy {
                "healthy".to_string()
            } else {
                "degraded".to_string()
            },
            details: Some(format!("status: {}", reg_status)),
        });

        // Capabilities
        let node_id = identity_service.get_identity().node_id.clone();
        let manifest =
            crate::node::capabilities::detect_capabilities(db, &node_id, Some(bridge), None);
        let capabilities_ok = !manifest.capabilities.is_empty();
        let verified_count = manifest
            .capabilities
            .iter()
            .filter(|c| c.verification_status.as_deref() == Some("verified"))
            .count();
        if !capabilities_ok && overall == "healthy" {
            overall = "degraded";
        }
        components.push(ComponentHealth {
            component: "capabilities".to_string(),
            status: if capabilities_ok {
                "healthy".to_string()
            } else {
                "degraded".to_string()
            },
            details: Some(format!(
                "{} capabilities, {} verified",
                manifest.capabilities.len(),
                verified_count
            )),
        });

        // Sessions
        let sessions_ok = true;
        components.push(ComponentHealth {
            component: "sessions".to_string(),
            status: if sessions_ok {
                "healthy".to_string()
            } else {
                "unhealthy".to_string()
            },
            details: Some(format!(
                "{} total sessions",
                session_service.list_sessions(None).len()
            )),
        });

        // Bootstrap
        let bootstrap_completed = bootstrap_service.get_receipts().len() > 0;
        components.push(ComponentHealth {
            component: "bootstrap".to_string(),
            status: if bootstrap_completed {
                "healthy".to_string()
            } else {
                "degraded".to_string()
            },
            details: Some(if bootstrap_completed {
                "Bootstrap completed".to_string()
            } else {
                "Bootstrap not yet completed".to_string()
            }),
        });

        // Custody
        let chain = custody_service.get_chain();
        let custody_exists = chain.is_some();
        let custody_integrity = if custody_exists {
            let report = custody_service.verify_integrity();
            report.verified
        } else {
            false
        };
        let custody_status = if custody_exists && custody_integrity {
            "healthy"
        } else if custody_exists {
            "degraded"
        } else {
            "not_available"
        };
        if custody_status == "degraded" && overall == "healthy" {
            overall = "degraded";
        }
        components.push(ComponentHealth {
            component: "custody".to_string(),
            status: custody_status.to_string(),
            details: Some(format!(
                "chain_exists: {}, integrity_verified: {}",
                custody_exists, custody_integrity
            )),
        });

        // Core integration
        let core_configured = core_integration_service.is_online();
        components.push(ComponentHealth {
            component: "core_integration".to_string(),
            status: if core_configured {
                "healthy".to_string()
            } else {
                "not_available".to_string()
            },
            details: Some(if core_configured {
                "Core endpoint configured".to_string()
            } else {
                "No Core endpoint configured (offline mode)".to_string()
            }),
        });

        NodeHealth {
            overall_status: overall.to_string(),
            components,
            checked_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn get_overview(
        &self,
        identity_service: &NodeIdentityService,
        registration_service: &RegistrationService,
        bridge: &CapabilityEvidenceBridge,
        session_service: &SessionService,
        bootstrap_service: &BootstrapService,
        custody_service: &CustodyService,
        core_integration_service: &CoreIntegrationService,
        db: &RuntimeDatabase,
    ) -> NodeOverview {
        let identity = identity_service.get_identity();
        let reg_record = registration_service.get_record();
        let node_id = identity.node_id.clone();

        let manifest =
            crate::node::capabilities::detect_capabilities(db, &node_id, Some(bridge), None);
        let capability_count = manifest.capabilities.len() as u32;
        let verified_capability_count = manifest
            .capabilities
            .iter()
            .filter(|c| c.verification_status.as_deref() == Some("verified"))
            .count() as u32;

        let all_sessions = session_service.list_sessions(None);
        let session_count = all_sessions.len() as u32;
        let active_session_count = all_sessions
            .iter()
            .filter(|s| s.state == "active")
            .count() as u32;

        let bootstrap_completed = bootstrap_service.get_receipts().len() > 0;
        let chain = custody_service.get_chain();
        let custody_envelope_count = chain.as_ref().map(|c| c.envelope_count).unwrap_or(0);

        let core_connected = core_integration_service.is_online();
        let last_sync_at = core_integration_service.get_last_sync_at();

        let registered =
            reg_record.registration_status == "registered"
                || reg_record.registration_status == "registration_requested";

        NodeOverview {
            node_id: identity.node_id.clone(),
            display_name: identity.display_name.clone(),
            status: if registered { "online".to_string() } else { "offline".to_string() },
            uptime_seconds: self.start_time.elapsed().as_secs(),
            state: reg_record.registration_status.clone(),
            registered,
            session_count,
            active_session_count,
            capability_count,
            verified_capability_count,
            bootstrap_completed,
            custody_envelope_count,
            core_connected,
            last_sync_at,
            observed_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn get_diagnostics(
        &self,
        identity_service: &NodeIdentityService,
        registration_service: &RegistrationService,
        bridge: &CapabilityEvidenceBridge,
        session_service: &SessionService,
        bootstrap_service: &BootstrapService,
        custody_service: &CustodyService,
        core_integration_service: &CoreIntegrationService,
        db: &RuntimeDatabase,
    ) -> DiagnosticReport {
        let health = self.get_health(
            identity_service,
            registration_service,
            bridge,
            session_service,
            bootstrap_service,
            custody_service,
            core_integration_service,
            db,
        );
        let overview = self.get_overview(
            identity_service,
            registration_service,
            bridge,
            session_service,
            bootstrap_service,
            custody_service,
            core_integration_service,
            db,
        );
        let session_summary = self.get_session_summary(session_service);
        let custody_summary = self.get_custody_summary(custody_service);

        DiagnosticReport {
            report_id: Uuid::new_v4().to_string(),
            requested_at: chrono::Utc::now().to_rfc3339(),
            health,
            overview,
            sessions: session_summary,
            custody: custody_summary,
        }
    }

    pub fn get_health_summary(&self, health: &NodeHealth) -> HealthSummary {
        let mut healthy = 0u32;
        let mut degraded = 0u32;
        let mut unhealthy = 0u32;

        for component in &health.components {
            match component.status.as_str() {
                "healthy" => healthy += 1,
                "degraded" | "not_available" => degraded += 1,
                _ => unhealthy += 1,
            }
        }

        HealthSummary {
            status: health.overall_status.clone(),
            healthy_count: healthy,
            degraded_count: degraded,
            unhealthy_count: unhealthy,
            total_components: health.components.len() as u32,
        }
    }

    pub fn get_session_summary(
        &self,
        session_service: &SessionService,
    ) -> DiagnosticSessionSummary {
        let sessions = session_service.list_sessions(None);
        let active_sessions = sessions.iter().filter(|s| s.state == "active").count() as u32;
        let closed_sessions = sessions
            .iter()
            .filter(|s| s.state == "closed")
            .count() as u32;

        let oldest_active = sessions
            .iter()
            .filter(|s| s.state == "active")
            .min_by_key(|s| &s.started_at)
            .map(|s| s.started_at.clone());

        let latest_closed = sessions
            .iter()
            .filter(|s| s.state == "closed")
            .max_by_key(|s| s.closed_at.as_deref().unwrap_or(""))
            .and_then(|s| s.closed_at.clone());

        DiagnosticSessionSummary {
            total_sessions: sessions.len() as u32,
            active_sessions,
            closed_sessions,
            oldest_active,
            latest_closed,
        }
    }

    pub fn get_custody_summary(
        &self,
        custody_service: &CustodyService,
    ) -> DiagnosticCustodySummary {
        let chain = custody_service.get_chain();
        let total_envelopes = chain.as_ref().map(|c| c.envelope_count).unwrap_or(0);

        let integrity_verified = if total_envelopes > 0 {
            custody_service.verify_integrity().verified
        } else {
            true
        };

        let all_envelopes = custody_service.get_envelopes_by_time_range(None, None);
        let first_envelope_at = all_envelopes.first().map(|e| e.timestamp.clone());
        let latest_envelope_at = all_envelopes.last().map(|e| e.timestamp.clone());

        let mut type_counts: HashMap<String, u32> = HashMap::new();
        for envelope in &all_envelopes {
            *type_counts
                .entry(envelope.receipt_type.clone())
                .or_insert(0) += 1;
        }
        let receipt_types: Vec<ReceiptTypeCount> = type_counts
            .into_iter()
            .map(|(receipt_type, count)| ReceiptTypeCount {
                receipt_type,
                count,
            })
            .collect();

        DiagnosticCustodySummary {
            total_envelopes,
            integrity_verified,
            first_envelope_at,
            latest_envelope_at,
            receipt_types,
        }
    }
}
