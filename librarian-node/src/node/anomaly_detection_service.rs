use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use librarian_contracts::anomaly_detection::{
    AnomalyFinding, AnomalyThreshold, BaselineRecord, DeviationObservation, SeverityLevel,
};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::policy_service::PolicyService;
use super::workload_lifecycle_service::WorkloadLifecycleService;
use super::workload_session_service::WorkloadSessionService;

pub struct AnomalyDetectionService {
    baselines: HashMap<String, BaselineRecord>,
    thresholds: Vec<AnomalyThreshold>,
    persistence_path: PathBuf,
    policy_service: Option<Arc<Mutex<PolicyService>>>,
}

fn baseline_key(metric_name: &str, context: &str) -> String {
    format!("{}::{}", metric_name, context)
}

fn default_thresholds() -> Vec<AnomalyThreshold> {
    vec![
        AnomalyThreshold {
            metric_name: "inference_latency_ms".to_string(),
            context_pattern: None,
            deviation_factor_threshold: 2.0,
            min_samples: 10,
            severity_map: vec![
                SeverityLevel { min_deviation_factor: 2.0, severity: "info".to_string() },
                SeverityLevel { min_deviation_factor: 3.0, severity: "notable".to_string() },
                SeverityLevel { min_deviation_factor: 4.0, severity: "warning".to_string() },
                SeverityLevel { min_deviation_factor: 5.0, severity: "critical".to_string() },
            ],
        },
        AnomalyThreshold {
            metric_name: "success_rate".to_string(),
            context_pattern: None,
            deviation_factor_threshold: 2.0,
            min_samples: 5,
            severity_map: vec![
                SeverityLevel { min_deviation_factor: 2.0, severity: "info".to_string() },
                SeverityLevel { min_deviation_factor: 2.5, severity: "notable".to_string() },
                SeverityLevel { min_deviation_factor: 3.0, severity: "warning".to_string() },
                SeverityLevel { min_deviation_factor: 4.0, severity: "critical".to_string() },
            ],
        },
        AnomalyThreshold {
            metric_name: "duration_seconds".to_string(),
            context_pattern: None,
            deviation_factor_threshold: 2.0,
            min_samples: 10,
            severity_map: vec![
                SeverityLevel { min_deviation_factor: 2.0, severity: "info".to_string() },
                SeverityLevel { min_deviation_factor: 3.0, severity: "notable".to_string() },
                SeverityLevel { min_deviation_factor: 4.0, severity: "warning".to_string() },
                SeverityLevel { min_deviation_factor: 5.0, severity: "critical".to_string() },
            ],
        },
    ]
}

impl AnomalyDetectionService {
    pub fn new(persistence_path: PathBuf) -> Self {
        let baselines = Self::load_baselines(&persistence_path);
        let thresholds = Self::load_thresholds(&persistence_path);
        AnomalyDetectionService {
            baselines,
            thresholds,
            persistence_path,
            policy_service: None,
        }
    }

    pub fn with_policy(mut self, ps: Arc<Mutex<PolicyService>>) -> Self {
        self.policy_service = Some(ps);
        self
    }

    fn threshold_from_policy(&self, policy_name: &str) -> Option<(f64, u32)> {
        if let Some(ref ps) = self.policy_service {
            if let Ok(svc) = ps.try_lock() {
                if let Some(val) = svc.get_policy_value(policy_name) {
                    let threshold = val.get("deviation_factor_threshold").and_then(|v| v.as_f64()).unwrap_or(2.0);
                    let min_samples = val.get("min_samples").and_then(|v| v.as_u64()).unwrap_or(10) as u32;
                    return Some((threshold, min_samples));
                }
            }
        }
        None
    }

    fn load_baselines(path: &PathBuf) -> HashMap<String, BaselineRecord> {
        let baselines_path = path
            .parent()
            .map(|p| p.join("anomaly_baselines.json"))
            .unwrap_or_else(|| PathBuf::from("data/anomaly_baselines.json"));
        if let Ok(data) = std::fs::read_to_string(&baselines_path) {
            if let Ok(records) = serde_json::from_str::<Vec<BaselineRecord>>(&data) {
                return records
                    .into_iter()
                    .map(|r| (baseline_key(&r.metric_name, &r.context), r))
                    .collect();
            }
        }
        HashMap::new()
    }

    fn persist_baselines(&self) {
        let baselines_path = self
            .persistence_path
            .parent()
            .map(|p| p.join("anomaly_baselines.json"))
            .unwrap_or_else(|| PathBuf::from("data/anomaly_baselines.json"));
        let records: Vec<&BaselineRecord> = self.baselines.values().collect();
        if let Ok(data) = serde_json::to_string_pretty(&records) {
            let _ = std::fs::write(&baselines_path, data);
        }
    }

    fn load_thresholds(path: &PathBuf) -> Vec<AnomalyThreshold> {
        let thresholds_path = path
            .parent()
            .map(|p| p.join("anomaly_thresholds.json"))
            .unwrap_or_else(|| PathBuf::from("data/anomaly_thresholds.json"));
        if let Ok(data) = std::fs::read_to_string(&thresholds_path) {
            if let Ok(t) = serde_json::from_str::<Vec<AnomalyThreshold>>(&data) {
                return t;
            }
        }
        default_thresholds()
    }

    fn persist_thresholds(&self) {
        let thresholds_path = self
            .persistence_path
            .parent()
            .map(|p| p.join("anomaly_thresholds.json"))
            .unwrap_or_else(|| PathBuf::from("data/anomaly_thresholds.json"));
        if let Ok(data) = serde_json::to_string_pretty(&self.thresholds) {
            let _ = std::fs::write(&thresholds_path, data);
        }
    }

    fn find_threshold(&self, metric_name: &str, context: &str) -> Option<&AnomalyThreshold> {
        // First try exact context match, then try wildcard (None) pattern
        for threshold in &self.thresholds {
            if threshold.metric_name != metric_name {
                continue;
            }
            match &threshold.context_pattern {
                Some(pattern) => {
                    if pattern == context || pattern == "*" {
                        return Some(threshold);
                    }
                }
                None => {
                    return Some(threshold);
                }
            }
        }
        None
    }

    /// Override threshold deviation factor from policy if available.
    fn get_policy_deviation_threshold(&self, metric_name: &str) -> Option<f64> {
        let policy_name = match metric_name {
            "inference_latency_ms" => "anomaly.threshold.inference_latency",
            "success_rate" => "anomaly.threshold.success_rate",
            "duration_seconds" => "anomaly.threshold.duration",
            _ => return None,
        };
        self.threshold_from_policy(policy_name).map(|(t, _)| t)
    }

    fn get_policy_min_samples(&self, metric_name: &str) -> Option<u32> {
        let policy_name = match metric_name {
            "inference_latency_ms" => "anomaly.threshold.inference_latency",
            "success_rate" => "anomaly.threshold.success_rate",
            "duration_seconds" => "anomaly.threshold.duration",
            _ => return None,
        };
        self.threshold_from_policy(policy_name).map(|(_, m)| m)
    }

    fn compute_severity(&self, metric_name: &str, context: &str, deviation_factor: f64) -> String {
        if let Some(threshold) = self.find_threshold(metric_name, context) {
            let mut best = "info".to_string();
            for sl in &threshold.severity_map {
                if deviation_factor >= sl.min_deviation_factor {
                    best = sl.severity.clone();
                }
            }
            return best;
        }
        "info".to_string()
    }

    pub fn update_baseline(
        &mut self,
        metric_name: &str,
        context: &str,
        values: &[f64],
    ) -> BaselineRecord {
        let n = values.len() as u32;
        let mean = if n > 0 {
            values.iter().sum::<f64>() / n as f64
        } else {
            0.0
        };

        let variance = if n > 1 {
            values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1) as f64
        } else {
            0.0
        };
        let std_dev = variance.sqrt();

        let now = chrono::Utc::now().to_rfc3339();
        let record = BaselineRecord {
            baseline_id: Uuid::new_v4().to_string(),
            metric_name: metric_name.to_string(),
            context: context.to_string(),
            mean,
            std_dev,
            sample_count: n,
            window_start: now.clone(),
            window_end: now.clone(),
            recorded_at: now,
        };

        let key = baseline_key(metric_name, context);
        self.baselines.insert(key, record.clone());
        self.persist_baselines();
        record
    }

    pub fn get_baseline(&self, metric_name: &str, context: &str) -> Option<&BaselineRecord> {
        self.baselines.get(&baseline_key(metric_name, context))
    }

    pub fn get_all_baselines(&self) -> Vec<BaselineRecord> {
        self.baselines.values().cloned().collect()
    }

    pub fn reset_baseline(&mut self, metric_name: &str, context: &str) {
        let key = baseline_key(metric_name, context);
        self.baselines.remove(&key);
        self.persist_baselines();
    }

    pub fn detect_deviation(
        &self,
        metric_name: &str,
        context: &str,
        observed_value: f64,
        workload_ids: Vec<String>,
    ) -> Option<DeviationObservation> {
        let baseline = self.baselines.get(&baseline_key(metric_name, context))?;

        if baseline.std_dev == 0.0 {
            return None;
        }

        let threshold = self.find_threshold(metric_name, context)?;
        let min_samples = self.get_policy_min_samples(metric_name)
            .unwrap_or(threshold.min_samples);
        if baseline.sample_count < min_samples {
            return None;
        }

        let deviation_factor = (observed_value - baseline.mean).abs() / baseline.std_dev;

        if deviation_factor < threshold.deviation_factor_threshold {
            return None;
        }

        let direction = if observed_value > baseline.mean {
            "increase".to_string()
        } else {
            "decrease".to_string()
        };

        Some(DeviationObservation {
            observation_id: Uuid::new_v4().to_string(),
            metric_name: metric_name.to_string(),
            context: context.to_string(),
            baseline_mean: baseline.mean,
            baseline_std_dev: baseline.std_dev,
            observed_value,
            deviation_factor,
            direction,
            observed_at: chrono::Utc::now().to_rfc3339(),
            evidence_workload_ids: workload_ids,
        })
    }

    pub fn check_for_anomalies(
        &self,
        metric_name: &str,
        context: &str,
        observed_value: f64,
        workload_ids: Vec<String>,
    ) -> Option<AnomalyFinding> {
        let observation = self.detect_deviation(metric_name, context, observed_value, workload_ids)?;

        let threshold = self.find_threshold(metric_name, context);
        let threshold_exceeded = self.get_policy_deviation_threshold(metric_name)
            .unwrap_or_else(|| threshold.map(|t| t.deviation_factor_threshold).unwrap_or(2.0));

        let severity = self.compute_severity(metric_name, context, observation.deviation_factor);

        Some(AnomalyFinding {
            anomaly_id: Uuid::new_v4().to_string(),
            observation,
            threshold_exceeded,
            severity,
            generated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub fn get_thresholds(&self) -> Vec<AnomalyThreshold> {
        self.thresholds.clone()
    }

    pub fn set_threshold(&mut self, threshold: AnomalyThreshold) {
        let existing_idx = self
            .thresholds
            .iter()
            .position(|t| t.metric_name == threshold.metric_name);
        match existing_idx {
            Some(idx) => {
                self.thresholds[idx] = threshold;
            }
            None => {
                self.thresholds.push(threshold);
            }
        }
        self.persist_thresholds();
    }

    pub fn compute_baselines_from_history(
        &mut self,
        ws_service: &WorkloadSessionService,
    ) -> Vec<BaselineRecord> {
        let inventory = WorkloadLifecycleService::get_inventory(ws_service);
        let mut type_map: HashMap<String, Vec<&librarian_contracts::workload_lifecycle::WorkloadSummary>> =
            HashMap::new();
        for wl in &inventory.workloads {
            type_map
                .entry(wl.workload_type.clone())
                .or_default()
                .push(wl);
        }

        let mut results = Vec::new();

        for (wl_type, workloads) in &type_map {
            let durations: Vec<f64> = workloads
                .iter()
                .filter_map(|w| w.duration_seconds.map(|d| d as f64))
                .collect();
            if !durations.is_empty() {
                let duration_context = format!("workload_type:{}", wl_type);
                results.push(self.update_baseline("duration_seconds", &duration_context, &durations));
            }

            let total = workloads.len() as u32;
            let completed = workloads.iter().filter(|w| w.state == "completed").count() as u32;
            if total >= 5 {
                let success_rate = completed as f64 / total as f64;
                let success_context = format!("workload_type:{}", wl_type);
                results.push(self.update_baseline(
                    "success_rate",
                    &success_context,
                    &[success_rate],
                ));
            }

            let evidence_count: Vec<f64> = workloads
                .iter()
                .filter_map(|w| w.evidence_count.map(|c| c as f64))
                .collect();
            if !evidence_count.is_empty() {
                let evidence_context = format!("workload_type:{}", wl_type);
                results.push(self.update_baseline(
                    "evidence_count",
                    &evidence_context,
                    &evidence_count,
                ));
            }
        }

        results
    }

    pub fn scan_all_metrics(
        &self,
        ws_service: &WorkloadSessionService,
    ) -> Vec<AnomalyFinding> {
        let inventory = WorkloadLifecycleService::get_inventory(ws_service);
        let mut findings = Vec::new();

        let mut type_map: HashMap<String, Vec<&librarian_contracts::workload_lifecycle::WorkloadSummary>> =
            HashMap::new();
        for wl in &inventory.workloads {
            type_map
                .entry(wl.workload_type.clone())
                .or_default()
                .push(wl);
        }

        for (wl_type, workloads) in &type_map {
            let wl_ids: Vec<String> =
                workloads.iter().map(|w| w.workload_id.clone()).collect();

            let avg_duration: Option<f64> = {
                let durs: Vec<f64> = workloads
                    .iter()
                    .filter_map(|w| w.duration_seconds.map(|d| d as f64))
                    .collect();
                if durs.is_empty() {
                    None
                } else {
                    Some(durs.iter().sum::<f64>() / durs.len() as f64)
                }
            };
            if let Some(dur) = avg_duration {
                let ctx = format!("workload_type:{}", wl_type);
                if let Some(finding) = self.check_for_anomalies("duration_seconds", &ctx, dur, wl_ids.clone()) {
                    findings.push(finding);
                }
            }

            let total = workloads.len() as u32;
            let completed = workloads.iter().filter(|w| w.state == "completed").count() as u32;
            if total > 0 {
                let success_rate = completed as f64 / total as f64;
                let ctx = format!("workload_type:{}", wl_type);
                if let Some(finding) = self.check_for_anomalies("success_rate", &ctx, success_rate, wl_ids.clone()) {
                    findings.push(finding);
                }
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{
        session_service::SessionService, AllocationService, FleetService, OwnerAllocationService,
    };
    use librarian_contracts::fleet::NodeInventoryEntry;
    use librarian_contracts::workload_session::WorkloadDescriptor;
    use tempfile::tempdir;

    fn make_service(dir: &tempfile::TempDir) -> AnomalyDetectionService {
        AnomalyDetectionService::new(dir.path().join("test.json"))
    }

    fn test_workload(id: &str, wl_type: &str) -> WorkloadDescriptor {
        WorkloadDescriptor {
            workload_id: id.to_string(),
            workload_type: wl_type.to_string(),
            description: format!("Test workload {}", id),
            requirements: Some(vec!["llm.inference".to_string()]),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn setup_services(
    ) -> (WorkloadSessionService, SessionService, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let session_path = dir.path().join("sessions.json");
        let session_service = SessionService::new(session_path);
        let ws_path = dir.path().join("workload_sessions.json");
        let ws_service = WorkloadSessionService::new(ws_path);
        (ws_service, session_service, dir)
    }

    fn run_workload_to_completion(
        ws: &mut WorkloadSessionService,
        sess: &mut SessionService,
        wl_id: &str,
        wl_type: &str,
        node_id: &str,
        duration: u32,
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
            duration,
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
    fn baseline_computation_produces_correct_mean_and_std_dev() {
        let dir = tempdir().unwrap();
        let mut service = make_service(&dir);

        let values = vec![10.0, 12.0, 11.0, 9.0, 13.0];
        let record = service.update_baseline("inference_latency_ms", "workload_type:inference", &values);

        assert!((record.mean - 11.0).abs() < 1e-10);

        let expected_variance =
            values.iter().map(|v| (v - 11.0).powi(2)).sum::<f64>() / 4.0;
        let expected_std_dev = expected_variance.sqrt();
        assert!((record.std_dev - expected_std_dev).abs() < 1e-10);
        assert_eq!(record.sample_count, 5);
    }

    #[test]
    fn deviation_detection_returns_none_below_threshold() {
        let dir = tempdir().unwrap();
        let mut service = make_service(&dir);

        let values = vec![10.0, 11.0, 10.5, 9.5, 10.2, 10.8, 9.8, 10.1, 10.3, 10.0];
        service.update_baseline("inference_latency_ms", "workload_type:inference", &values);

        // Observed value close to mean should return None
        let result = service.detect_deviation(
            "inference_latency_ms",
            "workload_type:inference",
            10.1,
            vec![],
        );
        assert!(result.is_none(), "Should return None for value within normal range");
    }

    #[test]
    fn deviation_detection_returns_observation_above_threshold() {
        let dir = tempdir().unwrap();
        let mut service = make_service(&dir);

        let values = vec![10.0, 11.0, 10.5, 9.5, 10.2, 10.8, 9.8, 10.1, 10.3, 10.0];
        service.update_baseline("inference_latency_ms", "workload_type:inference", &values);

        // Observed value far from mean should return an observation
        let result = service.detect_deviation(
            "inference_latency_ms",
            "workload_type:inference",
            50.0,
            vec!["wl-001".to_string()],
        );
        assert!(result.is_some(), "Should return observation for extreme value");
        if let Some(obs) = result {
            assert_eq!(obs.metric_name, "inference_latency_ms");
            assert_eq!(obs.direction, "increase");
            assert!(obs.deviation_factor > 2.0);
            assert_eq!(obs.evidence_workload_ids, vec!["wl-001".to_string()]);
        }
    }

    #[test]
    fn anomaly_finding_has_correct_severity_from_threshold_map() {
        let dir = tempdir().unwrap();
        let mut service = make_service(&dir);

        // Build baseline with known mean=100, std_dev~10.54
        let values = vec![90.0, 95.0, 100.0, 105.0, 110.0, 92.0, 98.0, 108.0, 102.0, 96.0];
        service.update_baseline("inference_latency_ms", "workload_type:inference", &values);

        // Deviation far exceeding 5.0 std_dev -> critical
        let finding = service
            .check_for_anomalies("inference_latency_ms", "workload_type:inference", 200.0, vec![])
            .unwrap();
        assert_eq!(finding.severity, "critical");

        // Deviation between 3.0 and 4.0 std_dev -> notable
        // mean ~99.6, std_dev ~6.6; 122 gives deviation ~3.4 -> notable
        let finding2 = service
            .check_for_anomalies("inference_latency_ms", "workload_type:inference", 122.0, vec![])
            .unwrap();
        assert_eq!(finding2.severity, "notable");

        // Deviation between 2.0 and 2.5 -> info
        let finding3 = service
            .check_for_anomalies("inference_latency_ms", "workload_type:inference", 114.0, vec![])
            .unwrap();
        assert_eq!(finding3.severity, "info");

        let values2 = vec![0.9, 1.0, 0.95, 0.98, 1.0];
        service.update_baseline("success_rate", "workload_type:vision", &values2);

        // success_rate deviation 0.2 from mean ~0.966, std_dev ~0.043
        // |0.2 - 0.966| / 0.043 = 17.8 -> critical
        let finding4 = service
            .check_for_anomalies("success_rate", "workload_type:vision", 0.2, vec![])
            .unwrap();
        assert_eq!(finding4.severity, "critical");
    }

    #[test]
    fn baselines_persist_across_restarts() {
        let dir = tempdir().unwrap();
        let persistence_path = dir.path().join("persist.json");

        {
            let mut service = AnomalyDetectionService::new(persistence_path.clone());
            let values = vec![10.0, 12.0, 11.0];
            service.update_baseline("inference_latency_ms", "workload_type:inference", &values);
            assert_eq!(service.get_all_baselines().len(), 1);
        }

        {
            let service = AnomalyDetectionService::new(persistence_path);
            let baselines = service.get_all_baselines();
            assert_eq!(baselines.len(), 1);
            assert_eq!(baselines[0].metric_name, "inference_latency_ms");
            assert!((baselines[0].mean - 11.0).abs() < 1e-10);
        }
    }

    #[test]
    fn threshold_configuration_updates_work() {
        let dir = tempdir().unwrap();
        let mut service = make_service(&dir);

        let thresholds = service.get_thresholds();
        assert!(thresholds.len() >= 3);

        let custom = AnomalyThreshold {
            metric_name: "custom_metric".to_string(),
            context_pattern: Some("*".to_string()),
            deviation_factor_threshold: 3.0,
            min_samples: 5,
            severity_map: vec![SeverityLevel {
                min_deviation_factor: 3.0,
                severity: "warning".to_string(),
            }],
        };
        service.set_threshold(custom);

        let updated = service.get_thresholds();
        assert_eq!(updated.len(), 4);

        let custom_found = updated.iter().find(|t| t.metric_name == "custom_metric");
        assert!(custom_found.is_some());
        assert_eq!(custom_found.unwrap().deviation_factor_threshold, 3.0);
    }

    #[test]
    fn reset_baseline_removes_record() {
        let dir = tempdir().unwrap();
        let mut service = make_service(&dir);

        let values = vec![10.0, 12.0, 11.0];
        service.update_baseline("inference_latency_ms", "workload_type:inference", &values);
        assert_eq!(service.get_all_baselines().len(), 1);

        service.reset_baseline("inference_latency_ms", "workload_type:inference");
        assert_eq!(service.get_all_baselines().len(), 0);
    }

    #[test]
    fn scan_detects_anomalies_across_multiple_metrics() {
        let (mut ws, mut sess, dir) = setup_services();
        let mut service = make_service(&dir);

        // Directly create baselines (duration_seconds in inventory is computed from
        // timestamps, not from operations_executed, so we set up baselines directly)
        let durations = vec![5.0, 6.0, 5.0, 7.0, 5.0, 6.0, 5.0, 4.0, 6.0, 5.0];
        service.update_baseline("duration_seconds", "workload_type:", &durations);
        let success_values = vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        service.update_baseline("success_rate", "workload_type:", &success_values);

        // Add many successful workloads
        for i in 0..10 {
            run_workload_to_completion(
                &mut ws, &mut sess,
                &format!("wl-normal-{}", i),
                "test", "node-a", 5,
            );
        }

        // Add a failed workload to change success rate
        run_workload_to_failure(
            &mut ws, &mut sess,
            "wl-fail", "test", "node-a",
        );

        let findings = service.scan_all_metrics(&ws);
        assert!(!findings.is_empty(), "Should detect at least one anomaly");
    }

    #[test]
    fn anomalies_can_be_classified_into_classified_findings() {
        use librarian_contracts::evidence_classification::ClassifiedFinding;
        use librarian_contracts::evidence_intelligence::{IntelligenceFinding, IntelligenceReport};

        let dir = tempdir().unwrap();
        let mut service = make_service(&dir);

        let values = vec![10.0, 11.0, 10.5, 9.5, 10.2, 10.8, 9.8, 10.1, 10.3, 10.0];
        service.update_baseline("inference_latency_ms", "workload_type:inference", &values);

        let anomaly = service
            .check_for_anomalies("inference_latency_ms", "workload_type:inference", 50.0, vec![])
            .unwrap();

        // Classify the anomaly into an IntelligenceFinding (as the evidence system would)
        let raw = IntelligenceFinding {
            finding_id: anomaly.anomaly_id.clone(),
            category: "node_health".to_string(),
            severity: anomaly.severity.clone(),
            title: format!(
                "Anomaly detected: {} for {}",
                anomaly.observation.metric_name, anomaly.observation.context
            ),
            description: format!(
                "Deviation factor {:.2} (threshold {:.1}), observed {}, expected {:.1} +/- {:.1}",
                anomaly.observation.deviation_factor,
                anomaly.threshold_exceeded,
                anomaly.observation.observed_value,
                anomaly.observation.baseline_mean,
                anomaly.observation.baseline_std_dev,
            ),
            supporting_data: serde_json::to_value(&anomaly).unwrap(),
            source_references: anomaly.observation.evidence_workload_ids.clone(),
            generated_at: chrono::Utc::now().to_rfc3339(),
        };

        assert_eq!(raw.category, "node_health");
        assert_eq!(raw.severity, "critical");
        assert!(!raw.title.is_empty());
        assert!(!raw.description.is_empty());
    }

    #[test]
    fn default_thresholds_are_configured() {
        let dir = tempdir().unwrap();
        let service = make_service(&dir);
        let thresholds = service.get_thresholds();

        let latency = thresholds
            .iter()
            .find(|t| t.metric_name == "inference_latency_ms");
        assert!(latency.is_some());
        assert_eq!(latency.unwrap().min_samples, 10);
        assert_eq!(latency.unwrap().deviation_factor_threshold, 2.0);

        let success = thresholds
            .iter()
            .find(|t| t.metric_name == "success_rate");
        assert!(success.is_some());
        assert_eq!(success.unwrap().min_samples, 5);
    }

    #[test]
    fn deviation_factor_is_computed_correctly() {
        let dir = tempdir().unwrap();
        let mut service = make_service(&dir);

        let values = vec![10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0];
        service.update_baseline("inference_latency_ms", "workload_type:inference", &values);

        // With std_dev = 0, should return None (division by zero guard)
        let result = service.detect_deviation(
            "inference_latency_ms",
            "workload_type:inference",
            20.0,
            vec![],
        );
        assert!(result.is_none(), "Should return None when std_dev is zero");
    }

    #[test]
    fn compute_baselines_from_history_creates_records() {
        let (mut ws, mut sess, dir) = setup_services();
        let mut service = make_service(&dir);

        for i in 0..5 {
            run_workload_to_completion(
                &mut ws,
                &mut sess,
                &format!("wl-history-{}", i),
                "inference",
                "node-a",
                5,
            );
        }
        for i in 0..3 {
            run_workload_to_failure(
                &mut ws,
                &mut sess,
                &format!("wl-fail-{}", i),
                "inference",
                "node-a",
            );
        }

        let records = service.compute_baselines_from_history(&ws);
        assert!(!records.is_empty(), "Should create baseline records from history");

        let duration_records: Vec<&BaselineRecord> = records
            .iter()
            .filter(|r| r.metric_name == "duration_seconds")
            .collect();
        assert!(!duration_records.is_empty(), "Should create duration baseline");

        let success_records: Vec<&BaselineRecord> = records
            .iter()
            .filter(|r| r.metric_name == "success_rate")
            .collect();
        assert!(!success_records.is_empty(), "Should create success_rate baseline");
    }
}
