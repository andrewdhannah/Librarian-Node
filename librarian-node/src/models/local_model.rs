//! Local model record — represents a GGUF model file known to this runtime node.

use serde::{Deserialize, Serialize};

/// A model file installed on this machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModel {
    pub model_id: String,
    pub display_name: String,
    pub family: Option<String>,
    pub source_repository: Option<String>,
    pub filename: String,
    pub quantization: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub sha256: Option<String>,
    pub capability_classes_json: Option<String>,
    pub created_at: String,
}

impl LocalModel {
    /// Create a new LocalModel with the given required fields and defaults.
    pub fn new(model_id: String, display_name: String, filename: String) -> Self {
        Self {
            model_id,
            display_name,
            family: None,
            source_repository: None,
            filename,
            quantization: None,
            file_size_bytes: None,
            sha256: None,
            capability_classes_json: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}
