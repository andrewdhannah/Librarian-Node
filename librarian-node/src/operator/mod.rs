//! Operator surface — human-facing runtime models and controls.
//!
//! Provides the backend models and services that power the operator
//! dashboard, taskbar agent, and event stream. The operator surface
//! is advisory only — it reports state but does not make decisions.

pub mod events;
pub mod models;
pub mod service;

pub use events::EventStore;
pub use models::{ModelEntry, OperatorEvent, OperatorState, RuntimeIndicator, RuntimeSnapshot};
pub use service::OperatorService;
