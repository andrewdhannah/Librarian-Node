//! Startup reconciliation — detects and recovers from inconsistent state.
//!
//! On Rust router startup, the supervisor compares persisted state with actual
//! process state and reconciles discrepancies.

use super::supervisor::ModelResidencySupervisor;
use super::state::ResidencyState;
use crate::runtime_state::lifecycle_evidence::LifecycleEventType;
use crate::runtime_state::model_lease::LeaseState;
use anyhow::Result;
use tracing::{info, warn};

/// Reconciliation result summary.
#[derive(Debug)]
pub struct ReconciliationResult {
    /// Number of stale leases detected and reconciled.
    pub stale_leases_reconciled: u32,
    /// Number of orphan processes detected.
    pub orphan_processes_detected: u32,
    /// Number of interrupted runs recorded.
    pub interrupted_runs_recorded: u32,
    /// Whether the supervisor ended in a clean Unloaded state.
    pub clean_startup: bool,
    /// Human-readable summary.
    pub summary: String,
}

/// Reconcile the supervisor state with actual process state.
///
/// Called once at startup before the HTTP server begins accepting requests.
pub async fn reconcile_startup(supervisor: &ModelResidencySupervisor) -> Result<ReconciliationResult> {
    let mut result = ReconciliationResult {
        stale_leases_reconciled: 0,
        orphan_processes_detected: 0,
        interrupted_runs_recorded: 0,
        clean_startup: false,
        summary: String::new(),
    };

    info!("Starting residency reconciliation...");

    // Get current supervisor state
    let current_state = supervisor.current_state().await;
    let snapshot = supervisor.status().await;

    // Case 1: Clean startup — no active lease, no active run
    if current_state == ResidencyState::Unloaded && snapshot.active_lease_id.is_none() {
        // Check for any lingering llama-server processes
        let orphans = detect_orphan_processes();
        if orphans.is_empty() {
            result.clean_startup = true;
            result.summary = "Clean startup: no active lease, no active run, no orphan processes".to_string();
            info!("{}", result.summary);
            return Ok(result);
        } else {
            // Orphan processes found
            result.orphan_processes_detected = orphans.len() as u32;
            warn!(
                "Startup: {} orphan llama-server process(es) detected: {:?}",
                orphans.len(),
                orphans
            );

            // Record evidence for each orphan
            for pid in &orphans {
                supervisor.record_evidence(
                    LifecycleEventType::OrphanProcessDetected,
                    None, None, None, None,
                    Some(*pid),
                    Some("startup_reconciliation"),
                    &format!("{{\"pid\":{},\"action\":\"detected_at_startup\"}}", pid),
                ).await;
            }

            result.summary = format!(
                "Startup: {} orphan process(es) detected. Model loading blocked until orphan ownership established.",
                orphans.len()
            );
            warn!("{}", result.summary);
            return Ok(result);
        }
    }

    // Case 2: Active lease exists — verify process is still alive
    if let Some(ref lease_id) = snapshot.active_lease_id {
        let pid = snapshot.active_process_id;

        match pid {
            Some(pid) if is_process_alive(pid) => {
                // Process still alive — this is a restart with a surviving process
                info!(
                    "Startup: active lease {} with PID {} still alive — resuming supervision",
                    lease_id, pid
                );

                // Record reconciliation evidence
                supervisor.record_evidence(
                    LifecycleEventType::RuntimeReconciled,
                    snapshot.active_model_id.as_deref(),
                    snapshot.active_profile_id.as_deref(),
                    Some(lease_id),
                    None,
                    Some(pid),
                    Some("resumed"),
                    &format!("{{\"action\":\"resumed_supervision\",\"pid\":{}}}", pid),
                ).await;

                result.summary = format!(
                    "Startup: resumed supervision of active lease {} (PID {})",
                    lease_id, pid
                );
                info!("{}", result.summary);
            }
            _ => {
                // Stale lease — process is gone
                let pid_display = pid.map(|p| p.to_string()).unwrap_or_else(|| "unknown".into());
                warn!(
                    "Startup: stale lease {} — PID {} no longer exists",
                    lease_id, pid_display
                );

                // Mark lease as released
                let _ = supervisor.db().update_lease_state(lease_id, LeaseState::Unloaded);

                // Record stale lease evidence
                supervisor.record_evidence(
                    LifecycleEventType::StaleLeaseDetected,
                    snapshot.active_model_id.as_deref(),
                    snapshot.active_profile_id.as_deref(),
                    Some(lease_id),
                    None,
                    pid,
                    Some("unloaded"),
                    &format!("{{\"action\":\"stale_lease_reconciled\",\"pid\":{}}}", pid_display),
                ).await;

                result.stale_leases_reconciled += 1;

                // Check for interrupted runs
                if let Some(ref run_id) = snapshot.active_run_id {
                    let now = chrono::Utc::now().to_rfc3339();
                    let sql = format!(
                        "UPDATE runtime_runs SET exit_status='interrupted', ended_at='{}' WHERE run_id='{}' AND ended_at IS NULL;",
                        now, run_id
                    );
                    let _ = supervisor.db().execute_sql(&sql);

                    supervisor.record_evidence(
                        LifecycleEventType::RunInterrupted,
                        snapshot.active_model_id.as_deref(),
                        snapshot.active_profile_id.as_deref(),
                        Some(lease_id),
                        Some(run_id),
                        pid,
                        Some("interrupted"),
                        &format!("{{\"action\":\"run_interrupted_at_startup\",\"run_id\":\"{}\"}}", run_id),
                    ).await;

                    result.interrupted_runs_recorded += 1;
                }

                // Reset supervisor to Unloaded
                supervisor.force_fail("stale lease at startup").await;
                if let Err(e) = supervisor.recover().await {
                    warn!("Failed to recover to Unloaded: {}", e);
                }

                result.summary = format!(
                    "Startup: reconciled stale lease {} (PID {} gone), {} interrupted run(s)",
                    lease_id, pid_display, result.interrupted_runs_recorded
                );
                info!("{}", result.summary);
            }
        }
    }

    Ok(result)
}

/// Detect orphan llama-server processes not managed by the supervisor.
///
/// Returns PIDs of processes that look like llama-server but aren't tracked.
fn detect_orphan_processes() -> Vec<u32> {
    use std::process::Command;

    let mut orphans = Vec::new();

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("tasklist")
            .args(&["/FI", "IMAGENAME eq llama-server.exe", "/NH"])
            .output();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if line.contains("llama-server") {
                    // Parse PID from tasklist output (columns: Image Name, PID, ...)
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(pid_str) = parts.get(1) {
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            orphans.push(pid);
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("pgrep")
            .arg("llama-server")
            .output();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if let Ok(pid) = line.trim().parse::<u32>() {
                    orphans.push(pid);
                }
            }
        }
    }

    orphans
}

/// Check if a process with the given PID is still alive.
fn is_process_alive(pid: u32) -> bool {
    use std::process::Command;

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("tasklist")
            .args(&["/FI", &format!("PID eq {}", pid), "/NH"])
            .output();
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                stdout.contains(&pid.to_string())
            }
            Err(_) => false,
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
}
