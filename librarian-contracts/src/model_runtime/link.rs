use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelRuntimeEvidenceLink {
    pub link_id: String,
    pub model_id: String,
    pub runtime_type: String,
    pub evidence_packet_id: String,
    pub qualification_run_id: String,
    pub linked_at: String,
}
