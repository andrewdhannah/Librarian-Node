use serde::{Deserialize, Serialize};

use super::health::NodeHealth;
use super::overview::NodeOverview;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiagnosticReport {
    pub report_id: String,
    pub requested_at: String,
    pub health: NodeHealth,
    pub overview: NodeOverview,
    pub sessions: DiagnosticSessionSummary,
    pub custody: DiagnosticCustodySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiagnosticSessionSummary {
    pub total_sessions: u32,
    pub active_sessions: u32,
    pub closed_sessions: u32,
    pub oldest_active: Option<String>,
    pub latest_closed: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiagnosticCustodySummary {
    pub total_envelopes: u32,
    pub integrity_verified: bool,
    pub first_envelope_at: Option<String>,
    pub latest_envelope_at: Option<String>,
    pub receipt_types: Vec<ReceiptTypeCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReceiptTypeCount {
    pub receipt_type: String,
    pub count: u32,
}
