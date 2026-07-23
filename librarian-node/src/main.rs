//! ROUTER-RUST-HARDEN-1 — Hardened Rust router core for librarian-runtime-node.
//!
//! Preserves the Python router's HTTP contract and adds operational hardening.
//!
//! Usage:
//!     cargo run --release -- --port 9130
//!     cargo run --release -- --port 9130 --profiles <path-to-model-profiles.json>
//!     ROUTER_PORT=9130 cargo run --release
//! 

use librarian_node::config::{ProfileManager, RouterConfig};
use librarian_node::db::RuntimeDatabase;
use librarian_node::evidence::EvidenceWriter;
use librarian_node::node::{AllocationService, AnomalyDetectionService, BootstrapService, CoreIntegrationService, CustodyService, EvidenceClassificationService, EvidenceIntelligenceService, FleetService, FleetTrustService, ModelRuntimeService, NodeStateMachine, OperationsService, OwnerAllocationService, OwnerWorkflowService, PatternEscalationService, PolicyService, ReconciliationService, RecoveryCustodyService, RegistryApplyService, RegistryCandidateService, RegistryEnforcementService, RegistryMcpService, RegistryOwnerService, WorkloadLifecycleService, WorkloadSessionService};
use librarian_node::operator;
use librarian_node::platform::create_detector;
use librarian_node::residency::{ModelResidencySupervisor, SupervisorConfig, RuntimeStopStrategy};
use librarian_node::server::{build_router, AppState, start_health_poller, stop_health_poller};
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::EnvFilter;

/// Default port matching the Python router's convention.
const DEFAULT_PORT: u16 = 9130;

#[derive(Parser, Debug)]
#[command(name = "rust-router", version, about = "Hardened Rust router core for librarian-runtime-node")]
struct Args {
    /// Router host.
    #[arg(long)]
    host: Option<String>,

    /// Router HTTP port.
    #[arg(long)]
    port: Option<u16>,

    /// Path to model-profiles.json (overrides default sources).
    #[arg(long)]
    profiles: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    // Load config first to initialize logging
    let config = RouterConfig::from_env();

    // Initialize logging with optional file output
    let writer: BoxMakeWriter = if let Some(ref log_path) = config.log_path {
        let file = std::fs::File::create(log_path).expect("Failed to create log file");
        BoxMakeWriter::new(file)
    } else {
        BoxMakeWriter::new(std::io::stdout)
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .with_writer(writer)
        .init();

    let args = Args::parse();

    // Load profiles
    let profile_manager = match ProfileManager::load_from_config(&config) {
        Ok(pm) => pm,
        Err(e) => {
            error!("Failed to load profiles: {}", e);
            std::process::exit(1);
        }
    };

    info!(
        "Loaded {} profiles: {}",
        profile_manager.len(),
        profile_manager.aliases().join(", ")
    );

    // Initialize operational database — fails closed on error
    let db = RuntimeDatabase::open_from_config(&config).unwrap_or_else(|e| {
        error!("FATAL: Cannot initialize operational database: {}", e);
        std::process::exit(1);
    });

    db.migrate().unwrap_or_else(|e| {
        error!("FATAL: Database migration failed: {}", e);
        std::process::exit(1);
    });

    db.verify().unwrap_or_else(|e| {
        error!("FATAL: Database health check failed: {}", e);
        std::process::exit(1);
    });

    info!("Operational database initialized and verified");

    // Initialize model residency supervisor
    let supervisor_config = SupervisorConfig {
        stop_strategy: RuntimeStopStrategy::ProcessKill,
        baseline_free_vram_mb: 3433,
        release_tolerance_mb: 100,
        process_exit_timeout: std::time::Duration::from_secs(15),
        health_timeout: std::time::Duration::from_secs(60),
        health_poll_interval: std::time::Duration::from_millis(500),
    };
    let supervisor = ModelResidencySupervisor::new(supervisor_config, db.clone());

    // Startup reconciliation: clean stale leases and orphan processes
    match librarian_node::residency::reconciliation::reconcile_startup(&supervisor).await {
        Ok(report) => {
            if report.stale_leases_reconciled > 0 || report.orphan_processes_detected > 0 {
                info!(
                    "Startup reconciliation: {} stale leases, {} orphan PIDs, {} interrupted runs — {}",
                    report.stale_leases_reconciled,
                    report.orphan_processes_detected,
                    report.interrupted_runs_recorded,
                    report.summary
                );
            }
        }
        Err(e) => {
            error!("Startup reconciliation failed: {}", e);
            // Non-fatal: log and continue — supervisor will start in clean state
        }
    }

    // Backends are created on-demand via /backend/select
    let backends = tokio::sync::Mutex::new(std::collections::HashMap::new());

    // Evidence writer
    let evidence_writer = EvidenceWriter::new();

    // Initialize node identity service
    let identity_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("node-identity.json")
            } else {
                p.clone()
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/node-identity.json"));
    let node_identity_service = Arc::new(librarian_node::node::NodeIdentityService::new(identity_path));

    // Initialize node state machine
    let mut node_state_machine = NodeStateMachine::new();
    node_state_machine.set_state(librarian_contracts::node::NodeState::Registered);

    // Initialize registration service
    let registration_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("node-registration.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("node-registration.json"))
                    .unwrap_or_else(|| PathBuf::from("data/node-registration.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/node-registration.json"));
    let registration_service =
        librarian_node::node::RegistrationService::new(registration_path);

    // Initialize capability evidence bridge
    let evidence_bridge_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("capability-evidence.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("capability-evidence.json"))
                    .unwrap_or_else(|| PathBuf::from("data/capability-evidence.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/capability-evidence.json"));
    let capability_evidence_bridge =
        librarian_node::node::CapabilityEvidenceBridge::new(evidence_bridge_path.clone());

    // Initialize session service with its own evidence bridge instance
    let session_service =
        librarian_node::node::SessionService::new(
            config
                .evidence_path
                .as_ref()
                .map(|p| {
                    if p.is_dir() {
                        p.join("sessions.json")
                    } else {
                        p.parent()
                            .map(|parent| parent.join("sessions.json"))
                            .unwrap_or_else(|| PathBuf::from("data/sessions.json"))
                    }
                })
                .unwrap_or_else(|| PathBuf::from("data/sessions.json")),
        )
        .with_bridge(Arc::new(std::sync::Mutex::new(
            librarian_node::node::CapabilityEvidenceBridge::new(evidence_bridge_path.clone()),
        )));

    // Initialize custody service
    let custody_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("custody.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("custody.json"))
                    .unwrap_or_else(|| PathBuf::from("data/custody.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/custody.json"));
    let custody_service = CustodyService::new(custody_path);
    let custody_arc = Arc::new(std::sync::Mutex::new(custody_service));

    // Seed initial chain with existing node identity
    {
        let mut guard = custody_arc.lock().unwrap();
        let identity = node_identity_service.get_identity();
        let payload = serde_json::to_value(identity).unwrap_or_default();
        let metadata = librarian_contracts::custody::CustodyMetadata {
            source: "node".to_string(),
            version: "1".to_string(),
            notes: Some("Initial chain seeding on startup".to_string()),
        };
        guard.seed_identity(&identity.node_id, payload, metadata);
    }

    // Initialize bootstrap service — reuses the identity service instance
    let bootstrap_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("bootstrap.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("bootstrap.json"))
                    .unwrap_or_else(|| PathBuf::from("data/bootstrap.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/bootstrap.json"));
    let bootstrap_service = BootstrapService::new(
        bootstrap_path,
        node_identity_service.clone(),
        Arc::new(std::sync::Mutex::new(
            librarian_node::node::CapabilityEvidenceBridge::new(evidence_bridge_path.clone()),
        )),
        Arc::new(create_detector()),
    ).with_custody(custody_arc.clone());

    // Wire custody service into existing services
    let registration_service = registration_service.with_custody(custody_arc.clone());
    let session_service = session_service.with_custody(custody_arc.clone());
    let capability_evidence_bridge = capability_evidence_bridge.with_custody(custody_arc.clone());

    // Initialize core integration service (optional Core connection — offline by default)
    let core_integration_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("core-integration.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("core-integration.json"))
                    .unwrap_or_else(|| PathBuf::from("data/core-integration.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/core-integration.json"));
    let core_integration_service = CoreIntegrationService::new(
        core_integration_path,
        None, // core_endpoint — optional, defaults to offline
    );

    // Initialize owner workflow service
    let owner_workflow_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("owner-workflows.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("owner-workflows.json"))
                    .unwrap_or_else(|| PathBuf::from("data/owner-workflows.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/owner-workflows.json"));
    let owner_workflow_service = OwnerWorkflowService::new(owner_workflow_path)
        .with_custody(custody_arc.clone());

    // Initialize fleet service
    let fleet_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("fleet-inventory.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("fleet-inventory.json"))
                    .unwrap_or_else(|| PathBuf::from("data/fleet-inventory.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/fleet-inventory.json"));
    let mut fleet_service = FleetService::new(fleet_path);

    // Initialize fleet trust service
    let fleet_trust_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("fleet-trust.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("fleet-trust.json"))
                    .unwrap_or_else(|| PathBuf::from("data/fleet-trust.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/fleet-trust.json"));
    let fleet_trust_service = FleetTrustService::new(fleet_trust_path);

    // Initialize allocation service
    let allocation_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("allocation.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("allocation.json"))
                    .unwrap_or_else(|| PathBuf::from("data/allocation.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/allocation.json"));
    let allocation_service = AllocationService::new(allocation_path);

    // Initialize owner allocation service
    let owner_allocation_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("owner-allocation.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("owner-allocation.json"))
                    .unwrap_or_else(|| PathBuf::from("data/owner-allocation.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/owner-allocation.json"));
    let owner_allocation_service = OwnerAllocationService::new(owner_allocation_path);

    // Initialize workload session service
    let workload_session_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("workload-sessions.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("workload-sessions.json"))
                    .unwrap_or_else(|| PathBuf::from("data/workload-sessions.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/workload-sessions.json"));
    let workload_session_service = WorkloadSessionService::new(workload_session_path);

    // Seed local node into fleet inventory on startup
    {
        let identity = node_identity_service.get_identity().clone();
        let registration = serde_json::to_value(registration_service.get_record()).ok();
        let manifest = librarian_node::node::capabilities::detect_capabilities(
            &db,
            &identity.node_id,
            Some(&capability_evidence_bridge),
            None,
        );
        let capabilities_verified = manifest.capabilities.iter().all(|c| c.verification_status.as_deref() == Some("verified"));
        let capabilities = serde_json::to_value(&manifest).ok();
        let session_count = session_service.list_sessions(None).len() as u32;
        let bootstrap_completed = bootstrap_service.get_receipts().len() > 0;
        let (custody_envelope_count, last_integrity_hash) = {
            let custody = custody_arc.lock().unwrap();
            let chain = custody.get_chain();
            (
                chain.as_ref().map(|c| c.envelope_count).unwrap_or(0),
                chain.map(|c| c.last_chain_hash),
            )
        };
        let projection = core_integration_service.generate_projection(
            &identity,
            registration,
            capabilities,
            capabilities_verified,
            session_count,
            bootstrap_completed,
            custody_envelope_count,
            last_integrity_hash,
        );
        fleet_service.register_local_node(projection);
    }

    // Initialize evidence classification service
    let classification_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("classification.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("classification.json"))
                    .unwrap_or_else(|| PathBuf::from("data/classification.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/classification.json"));
    let evidence_classification_service = EvidenceClassificationService::new(classification_path);

    // Initialize anomaly detection service
    let anomaly_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("anomaly.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("anomaly.json"))
                    .unwrap_or_else(|| PathBuf::from("data/anomaly.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/anomaly.json"));
    let anomaly_detection_service = AnomalyDetectionService::new(anomaly_path);

    // Initialize pattern escalation service
    let pattern_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("pattern-escalation.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("pattern-escalation.json"))
                    .unwrap_or_else(|| PathBuf::from("data/pattern-escalation.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/pattern-escalation.json"));
    let pattern_escalation_service = PatternEscalationService::new(pattern_path);

    // Initialize reconciliation service (no session/registration/bridge deps — compare uses JSON)
    let reconciliation_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("reconciliation.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("reconciliation.json"))
                    .unwrap_or_else(|| PathBuf::from("data/reconciliation.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/reconciliation.json"));
    let reconciliation_service = ReconciliationService::new(reconciliation_path)
        .with_custody(custody_arc.clone());

    // Initialize recovery custody service
    let recovery_custody_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("recovery-custody.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("recovery-custody.json"))
                    .unwrap_or_else(|| PathBuf::from("data/recovery-custody.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/recovery-custody.json"));
    let recovery_custody_service = RecoveryCustodyService::new(recovery_custody_path)
        .with_custody(custody_arc.clone());

    // Initialize policy service
    let policy_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("policy.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("policy.json"))
                    .unwrap_or_else(|| PathBuf::from("data/policy.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/policy.json"));
    let policy_service = PolicyService::new(policy_path);
    let policy_service_arc = Arc::new(tokio::sync::Mutex::new(policy_service));

    // Wire policy into anomaly detection service
    let anomaly_detection_service = anomaly_detection_service.with_policy(policy_service_arc.clone());

    // Wire policy into pattern escalation service
    let pattern_escalation_service = pattern_escalation_service.with_policy(policy_service_arc.clone());

    // Wire policy into bootstrap service
    let bootstrap_service = bootstrap_service.with_policy(policy_service_arc.clone());

    // Initialize registry candidate service with startup recovery
    let registry_candidate_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("registry-candidates.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("registry-candidates.json"))
                    .unwrap_or_else(|| PathBuf::from("data/registry-candidates.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/registry-candidates.json"));
    let mut registry_candidate_service = RegistryCandidateService::new(registry_candidate_path);

    // Registry startup recovery: verify file integrity and recover stale candidates
    {
        let regenerated = registry_candidate_service.regenerate_if_corrupted();
        if regenerated {
            info!("Registry candidate data file was corrupted — regenerated from in-memory state");
        }
        let recovered = registry_candidate_service.recover_stale();
        if !recovered.is_empty() {
            info!(
                "Registry candidate recovery: {} stale candidates reverted from under_review state",
                recovered.len()
            );
        }
    }

    // Initialize registry MCP service
    let registry_mcp_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("registry-mcp.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("registry-mcp.json"))
                    .unwrap_or_else(|| PathBuf::from("data/registry-mcp.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/registry-mcp.json"));
    let registry_mcp_service = RegistryMcpService::new(registry_mcp_path);

    // Initialize registry apply service
    let registry_apply_path = config
        .evidence_path
        .as_ref()
        .map(|p| {
            if p.is_dir() {
                p.join("registry-apply.json")
            } else {
                p.parent()
                    .map(|parent| parent.join("registry-apply.json"))
                    .unwrap_or_else(|| PathBuf::from("data/registry-apply.json"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("data/registry-apply.json"));

    let state = Arc::new(AppState {
        profile_manager,
        config: config.clone(),
        backends,
        evidence_writer,
        start_time: std::time::Instant::now(),
        health_poller_handle: tokio::sync::Mutex::new(None),
        db,
        supervisor,
        operator: std::sync::Arc::new(tokio::sync::Mutex::new(operator::OperatorService::new())),
        node_identity_service,
        node_state: tokio::sync::Mutex::new(node_state_machine),
        registration_service: tokio::sync::Mutex::new(registration_service),
        capability_evidence_bridge: tokio::sync::Mutex::new(capability_evidence_bridge),
        session_service: tokio::sync::Mutex::new(session_service),
        bootstrap_service: tokio::sync::Mutex::new(bootstrap_service),
        custody_service: custody_arc.clone(),
        core_integration_service: tokio::sync::Mutex::new(core_integration_service),
        operations_service: OperationsService::new(),
        owner_workflow_service: tokio::sync::Mutex::new(owner_workflow_service),
        fleet_service: tokio::sync::Mutex::new(fleet_service),
        fleet_trust_service: tokio::sync::Mutex::new(fleet_trust_service),
        allocation_service: tokio::sync::Mutex::new(allocation_service),
        owner_allocation_service: tokio::sync::Mutex::new(owner_allocation_service),
        workload_session_service: tokio::sync::Mutex::new(workload_session_service),
        workload_lifecycle_service: WorkloadLifecycleService,
        evidence_intelligence_service: EvidenceIntelligenceService,
        evidence_classification_service: tokio::sync::Mutex::new(evidence_classification_service),
        anomaly_detection_service: tokio::sync::Mutex::new(anomaly_detection_service),
        pattern_escalation_service: tokio::sync::Mutex::new(pattern_escalation_service),
        reconciliation_service: tokio::sync::Mutex::new(reconciliation_service),
        recovery_custody_service: std::sync::Arc::new(std::sync::Mutex::new(recovery_custody_service)),
        policy_service: policy_service_arc,
        registry_candidate_service: tokio::sync::Mutex::new(registry_candidate_service),
        registry_enforcement_service: tokio::sync::Mutex::new(RegistryEnforcementService::new(
            config
                .evidence_path
                .as_ref()
                .map(|p| {
                    if p.is_dir() {
                        p.join("registry-enforcement.json")
                    } else {
                        p.parent()
                            .map(|parent| parent.join("registry-enforcement.json"))
                            .unwrap_or_else(|| PathBuf::from("data/registry-enforcement.json"))
                    }
                })
                .unwrap_or_else(|| PathBuf::from("data/registry-enforcement.json")),
        )),
        model_runtime_service: tokio::sync::Mutex::new(ModelRuntimeService::new()),
        registry_mcp_service: tokio::sync::Mutex::new(registry_mcp_service),
        registry_owner_service: tokio::sync::Mutex::new(
            RegistryOwnerService::new(
                config
                    .evidence_path
                    .as_ref()
                    .map(|p| {
                        if p.is_dir() {
                            p.join("registry-owner.json")
                        } else {
                            p.parent()
                                .map(|parent| parent.join("registry-owner.json"))
                                .unwrap_or_else(|| PathBuf::from("data/registry-owner.json"))
                        }
                    })
                    .unwrap_or_else(|| PathBuf::from("data/registry-owner.json")),
            )
            .with_custody(custody_arc.clone()),
        ),
        registry_apply_service: tokio::sync::Mutex::new(
            RegistryApplyService::new(registry_apply_path),
        ),
    });

    // Write startup evidence
    state.evidence_writer.write(
        "router-startup.json",
        &serde_json::json!({
            "status": "started",
            "port": args.port.unwrap_or(DEFAULT_PORT),
            "profiles_loaded": state.profile_manager.len(),
            "profiles": state.profile_manager.aliases(),
            "authority": "advisory_only",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    );

    // Start background health poller
    start_health_poller(state.clone(), config.health_poll_interval_secs).await;

    // Build router
    let app = build_router(state.clone());

    // Bind and serve
    let host = args.host.unwrap_or_else(|| config.router_host.clone());
    let port = args.port.unwrap_or(config.router_port);
    let addr = format!("{}:{}", host, port);
    let sep = "=".repeat(60);
    info!("{}", sep);
    info!("rust-router v0.1 (ROUTER-RUST-HARDEN-1)");
    info!("Listening on http://{}", addr);
    info!(
        "Profiles: {}",
        state.profile_manager.aliases().join(", ")
    );
    info!("Authority: advisory_only");
    info!("{}", sep);

    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind to {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    // Serve with graceful shutdown on ctrl-c
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(state.clone()))
        .await
        .unwrap_or_else(|e| {
            error!("Server error: {}", e);
        });
}

/// Handle graceful shutdown on Ctrl+C.
async fn shutdown_signal(state: Arc<AppState>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutting down... Cleaning up backends and health poller...");

    // Stop health poller
    stop_health_poller(&state).await;

    let backends = state.backends.lock().await;
    for (alias, bp) in backends.iter() {
        info!("Stopping backend '{}'...", alias);
        bp.stop().await;
    }

    info!("Shutdown complete.");
}
