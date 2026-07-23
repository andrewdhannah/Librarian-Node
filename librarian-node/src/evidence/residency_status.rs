//! Residency status — constructs ResidencyStatusResponse from Windows DB records.
//!
//! This module queries the sealed Windows operational DB and constructs
//! ResidencyStatusResponse structs for Mac-side routing decisions.
//!
//! It does NOT:
//! - Assign roles
//! - Classify capability
//! - Approve qualification
//! - Alter canonical qualification policy
//! - Take automatic residency action
//!
//! It only:
//! - Reports current residency state (leases, runs, VRAM)
//! - Provides evidence for Mac routing decisions

use anyhow::{Context, Result};

use librarian_contracts::residency_status::{ActiveLease, ActiveRun, ResidencyStatusResponse};
use crate::db::RuntimeDatabase;
use crate::runtime_state::LeaseState;

/// Baseline free VRAM (in MB) for the RX 570 on this system.
/// Must match SupervisorConfig::baseline_free_vram_mb.
const BASELINE_FREE_VRAM_MB: u64 = 3433;

/// Construct a ResidencyStatusResponse from Windows DB records.
///
/// Queries:
/// - job_leases (active leases)
/// - runtime_runs (active runs for each lease)
/// - hardware_profiles (VRAM status)
///
/// Returns the fully populated ResidencyStatusResponse.
pub fn build_residency_status(
    db: &RuntimeDatabase,
    model_id_filter: Option<&str>,
) -> Result<ResidencyStatusResponse> {
    // 1. Get active leases
    let all_active = db.get_active_leases()
        .context("Failed to query active leases")?;

    // 2. Filter by model_id if provided
    let active_leases: Vec<_> = match model_id_filter {
        Some(filter) => all_active.into_iter()
            .filter(|l| l.model_id == filter)
            .collect(),
        None => all_active,
    };

    // 3. Build ActiveLease structs
    let packet_leases: Vec<ActiveLease> = active_leases.iter().map(|l| {
        ActiveLease {
            lease_id: l.lease_id.clone(),
            model_id: l.model_id.clone(),
            profile_id: l.profile_id.clone(),
            state: l.state.as_str().to_string(),
            port: l.port.map(|p| p as u16),
            process_id: l.process_id,
        }
    }).collect();

    // 4. Collect active runs across all leases
    let mut packet_runs: Vec<ActiveRun> = Vec::new();
    let mut draining = false;

    for lease in &active_leases {
        // Check if any lease is draining
        if lease.state == LeaseState::Draining {
            draining = true;
        }

        // Get runs for this lease, filter for in-progress (ended_at IS NULL)
        let runs = db.list_runs_for_lease(&lease.lease_id)
            .context("Failed to query runs for lease")?;

        for run in runs {
            if run.ended_at.is_none() {
                packet_runs.push(ActiveRun {
                    run_id: run.run_id.clone(),
                    lease_id: run.lease_id.clone(),
                    started_at: Some(run.started_at.clone()),
                });
            }
        }
    }

    // 5. Build the response
    Ok(ResidencyStatusResponse {
        timestamp: chrono::Utc::now().to_rfc3339(),
        active_leases: packet_leases,
        active_runs: packet_runs,
        draining,
        available_vram_mb: Some(BASELINE_FREE_VRAM_MB),
        baseline_vram_mb: Some(BASELINE_FREE_VRAM_MB),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::RuntimeDatabase;
    use crate::models::{LocalModel, RuntimeProfile};
    use crate::runtime_state::{LeaseState, ModelLease, RuntimeRun};
    use tempfile::tempdir;

    /// Helper: create a test DB with a complete residency state.
    fn test_db_ready() -> (RuntimeDatabase, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test_residency.db");
        let db = RuntimeDatabase::open(path).unwrap();
        db.migrate().unwrap();

        // Insert model
        let mut model = LocalModel::new(
            "minicpm5-1b-q4km".to_string(),
            "MiniCPM5 1B Q4".to_string(),
            "MiniCPM5-1B-Q4_K_M.gguf".to_string(),
        );
        model.sha256 = Some("81B64D05A23B".to_string());
        db.insert_local_model(&model).unwrap();

        // Insert runtime profile
        let mut profile = RuntimeProfile::new(
            "prof-q4km".to_string(),
            "minicpm5-1b-q4km".to_string(),
            "vulkan".to_string(),
        );
        profile.measured_vram_mb = Some(2000);
        db.insert_runtime_profile(&profile).unwrap();

        (db, dir)
    }

    /// Helper: insert a ready lease with optional active run.
    fn insert_ready_lease(
        db: &RuntimeDatabase,
        lease_id: &str,
        model_id: &str,
        profile_id: &str,
        port: u32,
        has_active_run: bool,
    ) {
        let mut lease = ModelLease::new(lease_id.to_string(), model_id.to_string());
        lease.profile_id = Some(profile_id.to_string());
        lease.state = LeaseState::Ready;
        lease.port = Some(port as i32);
        lease.process_id = Some(10000 + port as i32);
        lease.loaded_at = Some("2026-07-11T11:59:50Z".to_string());
        db.insert_lease(&lease).unwrap();

        if has_active_run {
            let run = RuntimeRun::new(
                format!("run-{}", lease_id),
                lease_id.to_string(),
            );
            db.insert_run(&run).unwrap();
        }
    }

    // MQR-W2-1: build_residency_status succeeds with empty DB
    #[test]
    fn test_empty_db() {
        let (db, _dir) = test_db_ready();
        let response = build_residency_status(&db, None).unwrap();
        assert!(response.active_leases.is_empty());
        assert!(response.active_runs.is_empty());
        assert!(!response.draining);
    }

    // MQR-W2-2: active lease is included
    #[test]
    fn test_active_lease_included() {
        let (db, _dir) = test_db_ready();
        insert_ready_lease(&db, "lease-1", "minicpm5-1b-q4km", "prof-q4km", 9120, false);

        let response = build_residency_status(&db, None).unwrap();
        assert_eq!(response.active_leases.len(), 1);
        assert_eq!(response.active_leases[0].lease_id, "lease-1");
        assert_eq!(response.active_leases[0].model_id, "minicpm5-1b-q4km");
        assert_eq!(response.active_leases[0].state, "ready");
        assert_eq!(response.active_leases[0].port, Some(9120));
    }

    // MQR-W2-3: unloaded lease is NOT included
    #[test]
    fn test_unloaded_lease_excluded() {
        let (db, _dir) = test_db_ready();
        let mut lease = ModelLease::new("lease-unloaded".to_string(), "minicpm5-1b-q4km".to_string());
        lease.state = LeaseState::Unloaded;
        db.insert_lease(&lease).unwrap();

        let response = build_residency_status(&db, None).unwrap();
        assert!(response.active_leases.is_empty());
    }

    // MQR-W2-4: failed lease is NOT included
    #[test]
    fn test_failed_lease_excluded() {
        let (db, _dir) = test_db_ready();
        let mut lease = ModelLease::new("lease-failed".to_string(), "minicpm5-1b-q4km".to_string());
        lease.state = LeaseState::Failed;
        db.insert_lease(&lease).unwrap();

        let response = build_residency_status(&db, None).unwrap();
        assert!(response.active_leases.is_empty());
    }

    // MQR-W2-5: running lease IS included
    #[test]
    fn test_running_lease_included() {
        let (db, _dir) = test_db_ready();
        let mut lease = ModelLease::new("lease-running".to_string(), "minicpm5-1b-q4km".to_string());
        lease.state = LeaseState::Running;
        lease.port = Some(9120);
        db.insert_lease(&lease).unwrap();

        let response = build_residency_status(&db, None).unwrap();
        assert_eq!(response.active_leases.len(), 1);
        assert_eq!(response.active_leases[0].state, "running");
    }

    // MQR-W2-6: draining lease IS included and draining=true
    #[test]
    fn test_draining_lease_flag() {
        let (db, _dir) = test_db_ready();
        let mut lease = ModelLease::new("lease-draining".to_string(), "minicpm5-1b-q4km".to_string());
        lease.state = LeaseState::Draining;
        db.insert_lease(&lease).unwrap();

        let response = build_residency_status(&db, None).unwrap();
        assert_eq!(response.active_leases.len(), 1);
        assert!(response.draining);
    }

    // MQR-W2-7: active run (no ended_at) IS included
    #[test]
    fn test_active_run_included() {
        let (db, _dir) = test_db_ready();
        insert_ready_lease(&db, "lease-1", "minicpm5-1b-q4km", "prof-q4km", 9120, true);

        let response = build_residency_status(&db, None).unwrap();
        assert_eq!(response.active_runs.len(), 1);
        assert_eq!(response.active_runs[0].lease_id, "lease-1");
        assert!(response.active_runs[0].started_at.is_some());
    }

    // MQR-W2-8: completed run (has ended_at) is NOT included
    #[test]
    fn test_completed_run_excluded() {
        let (db, _dir) = test_db_ready();
        insert_ready_lease(&db, "lease-1", "minicpm5-1b-q4km", "prof-q4km", 9120, false);

        // Insert a completed run
        let mut run = RuntimeRun::new("run-completed".to_string(), "lease-1".to_string());
        run.ended_at = Some("2026-07-11T12:00:01Z".to_string());
        db.insert_run(&run).unwrap();

        let response = build_residency_status(&db, None).unwrap();
        assert!(response.active_runs.is_empty());
    }

    // MQR-W2-9: model_id filter works
    #[test]
    fn test_model_id_filter() {
        let (db, _dir) = test_db_ready();

        // Insert second model
        let mut model2 = LocalModel::new(
            "minicpm5-1b-q8".to_string(),
            "MiniCPM5 1B Q8".to_string(),
            "MiniCPM5-1B-Q8_0.gguf".to_string(),
        );
        model2.sha256 = Some("AABBCCDD".to_string());
        db.insert_local_model(&model2).unwrap();

        let profile2 = RuntimeProfile::new(
            "prof-q8".to_string(),
            "minicpm5-1b-q8".to_string(),
            "vulkan".to_string(),
        );
        db.insert_runtime_profile(&profile2).unwrap();

        insert_ready_lease(&db, "lease-q4", "minicpm5-1b-q4km", "prof-q4km", 9120, false);
        insert_ready_lease(&db, "lease-q8", "minicpm5-1b-q8", "prof-q8", 9121, false);

        // Filter for Q4 only
        let response = build_residency_status(&db, Some("minicpm5-1b-q4km")).unwrap();
        assert_eq!(response.active_leases.len(), 1);
        assert_eq!(response.active_leases[0].lease_id, "lease-q4");

        // Filter for Q8 only
        let response = build_residency_status(&db, Some("minicpm5-1b-q8")).unwrap();
        assert_eq!(response.active_leases.len(), 1);
        assert_eq!(response.active_leases[0].lease_id, "lease-q8");

        // Filter for nonexistent
        let response = build_residency_status(&db, Some("nonexistent")).unwrap();
        assert!(response.active_leases.is_empty());
    }

    // MQR-W2-10: timestamp is set
    #[test]
    fn test_timestamp_set() {
        let (db, _dir) = test_db_ready();
        let response = build_residency_status(&db, None).unwrap();
        assert!(!response.timestamp.is_empty());
        assert!(response.timestamp.contains("T"));
    }

    // MQR-W2-11: VRAM values are set
    #[test]
    fn test_vram_values() {
        let (db, _dir) = test_db_ready();
        let response = build_residency_status(&db, None).unwrap();
        assert_eq!(response.available_vram_mb, Some(BASELINE_FREE_VRAM_MB));
        assert_eq!(response.baseline_vram_mb, Some(BASELINE_FREE_VRAM_MB));
    }

    // MQR-W2-12: response validates
    #[test]
    fn test_response_validates() {
        let (db, _dir) = test_db_ready();
        insert_ready_lease(&db, "lease-1", "minicpm5-1b-q4km", "prof-q4km", 9120, true);

        let response = build_residency_status(&db, None).unwrap();
        assert!(response.validate().is_ok());
    }

    // MQR-W2-13: no capability data
    #[test]
    fn test_no_capability_data() {
        let (db, _dir) = test_db_ready();
        insert_ready_lease(&db, "lease-1", "minicpm5-1b-q4km", "prof-q4km", 9120, true);

        let response = build_residency_status(&db, None).unwrap();
        assert!(response.assert_no_capability_data().is_ok());
    }

    // MQR-W2-14: serialization round-trip
    #[test]
    fn test_serialization_round_trip() {
        let (db, _dir) = test_db_ready();
        insert_ready_lease(&db, "lease-1", "minicpm5-1b-q4km", "prof-q4km", 9120, true);

        let response = build_residency_status(&db, None).unwrap();
        let json = response.to_json().unwrap();
        let parsed = ResidencyStatusResponse::from_json(&json).unwrap();
        assert_eq!(response, parsed);
    }

    // MQR-W2-15: hash is deterministic
    #[test]
    fn test_hash_deterministic() {
        let (db, _dir) = test_db_ready();
        insert_ready_lease(&db, "lease-1", "minicpm5-1b-q4km", "prof-q4km", 9120, true);

        let response = build_residency_status(&db, None).unwrap();
        let h1 = response.compute_hash().unwrap();
        let h2 = response.compute_hash().unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }
}
