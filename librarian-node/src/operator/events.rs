//! Operator event stream — local runtime event recording.

use chrono::Utc;
use sha2::{Digest, Sha256};

use super::models::OperatorEvent;

/// In-memory event store (ring buffer, keeps last N events).
const MAX_EVENTS: usize = 100;

pub struct EventStore {
    events: Vec<OperatorEvent>,
}

impl EventStore {
    pub fn new() -> Self { Self { events: Vec::with_capacity(MAX_EVENTS) } }

    /// Record an event.
    pub fn record(&mut self, event_type: &str, model_id: Option<&str>, message: &str) {
        let timestamp = Utc::now().to_rfc3339();
        let mut h = Sha256::new();
        h.update(event_type.as_bytes());
        h.update(timestamp.as_bytes());
        let event_id = format!("{:x}", h.finalize());
        self.events.push(OperatorEvent { event_id, event_type: event_type.into(), model_id: model_id.map(String::from), message: message.into(), timestamp });
        if self.events.len() > MAX_EVENTS { self.events.remove(0); }
    }

    pub fn events(&self) -> &[OperatorEvent] { &self.events }
    pub fn recent(&self, n: usize) -> Vec<&OperatorEvent> { self.events.iter().rev().take(n).collect() }
    pub fn clear(&mut self) { self.events.clear(); }
}

impl Default for EventStore { fn default() -> Self { Self::new() } }
