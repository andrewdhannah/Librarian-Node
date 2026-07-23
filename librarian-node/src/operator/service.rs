use std::collections::HashMap;
use std::sync::Arc;

use crate::db::RuntimeDatabase;
use crate::process::BackendState;
use crate::residency::ModelResidencySupervisor;

use super::events::EventStore;
use super::models::{ModelEntry, OperatorState, RuntimeIndicator, RuntimeSnapshot};

pub struct OperatorService {
    pub events: EventStore,
}

impl OperatorService {
    pub fn new() -> Self { Self { events: EventStore::new() } }

    pub fn snapshot(
        &self,
        backends: &HashMap<String, Arc<crate::process::BackendProcess>>,
        _supervisor: &ModelResidencySupervisor,
        _db: &RuntimeDatabase,
    ) -> OperatorState {
        let runtime = self.runtime_snapshot(backends);
        let models = self.model_list(_db, backends);
        let events = self.events.events().to_vec();
        OperatorState { runtime, models, events, version: env!("CARGO_PKG_VERSION").to_string() }
    }

    fn runtime_snapshot(&self, backends: &HashMap<String, Arc<crate::process::BackendProcess>>) -> RuntimeSnapshot {
        for (_alias, proc) in backends.iter() {
            if let Ok(state) = proc.state.try_lock() {
                match *state {
                    BackendState::Healthy => {
                        return RuntimeSnapshot {
                            status: RuntimeIndicator::Running,
                            active_model: Some(proc.alias.clone()),
                            process_id: None,
                            gpu_vram_used_mb: None, gpu_vram_total_mb: None,
                            generation_speed: None, uptime_seconds: None, load_duration_ms: None,
                        };
                    }
                    BackendState::Starting => {
                        return RuntimeSnapshot {
                            status: RuntimeIndicator::Loading, active_model: None,
                            process_id: None, gpu_vram_used_mb: None, gpu_vram_total_mb: None,
                            generation_speed: None, uptime_seconds: None, load_duration_ms: None,
                        };
                    }
                    _ => {}
                }
            }
        }
        RuntimeSnapshot { status: RuntimeIndicator::Unavailable, active_model: None, process_id: None, gpu_vram_used_mb: None, gpu_vram_total_mb: None, generation_speed: None, uptime_seconds: None, load_duration_ms: None }
    }

    fn model_list(&self, _db: &RuntimeDatabase, backends: &HashMap<String, Arc<crate::process::BackendProcess>>) -> Vec<ModelEntry> {
        backends.iter().map(|(_, proc)| {
            let loaded = proc.state.try_lock().map(|s| *s == BackendState::Healthy).unwrap_or(false);
            ModelEntry { model_id: proc.alias.clone(), filename: String::new(), quantization: String::new(), qualified: true, loaded, active: loaded, gpu_vram_mb: 0, context_length: 8192 }
        }).collect()
    }
}

impl Default for OperatorService { fn default() -> Self { Self::new() } }
