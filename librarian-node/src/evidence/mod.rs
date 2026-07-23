//! Evidence subsystem — writes router evidence and exports EvidencePacket data.
//!
//! - `EvidenceWriter`: appends router events as JSON evidence files
//! - `export`: constructs EvidencePacket from Windows DB records for Mac intake

pub mod export;
pub mod residency_status;

use std::path::PathBuf;
use std::sync::Mutex;
use tracing::info;

/// Appends router evidence as JSON files.
///
/// Each write creates a file in the evidence directory with the given
/// filename. Unlike the Python writer which overwrites, the Rust writer
/// appends a timestamp to avoid conflicts in long-running sessions.
pub struct EvidenceWriter {
    directory: PathBuf,
    /// Simple counter for unique filenames within the same filename.
    counter: Mutex<std::collections::HashMap<String, u32>>,
}

impl EvidenceWriter {
    /// Default evidence directory matching the Python router.
    pub fn new() -> Self {
        let directory = PathBuf::from(
            r"G:\openwork\thelibrarian\fixtures\windows-runtime-node\router-impl",
        );
        std::fs::create_dir_all(&directory).ok();
        info!("Evidence directory: {}", directory.display());
        EvidenceWriter {
            directory,
            counter: Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Write a plain-text evidence file (e.g. process-before-after.txt).
    /// Creates the file directly in the evidence directory without counter suffixing.
    pub fn write_text(&self, filename: &str, content: &str) -> String {
        let path = self.directory.join(filename);
        match std::fs::write(&path, content) {
            Ok(_) => info!("Evidence written: {}", path.display()),
            Err(e) => tracing::warn!("Failed to write evidence {}: {}", path.display(), e),
        }
        path.to_string_lossy().to_string()
    }

    /// Write a JSON value as evidence.
    pub fn write(&self, filename: &str, data: &serde_json::Value) -> String {
        // Add a counter suffix to avoid overwriting
        let mut counter_map = self.counter.lock().unwrap();
        let count = counter_map.entry(filename.to_string()).or_insert(0);
        *count += 1;

        // Use a numbered filename for uniqueness
        let stamped_name = if *count == 1 {
            filename.to_string()
        } else {
            let dot = filename.rfind('.').unwrap_or(filename.len());
            format!("{}-{}{}", &filename[..dot], count, &filename[dot..])
        };

        let path = self.directory.join(&stamped_name);
        let content = serde_json::to_string_pretty(data).unwrap_or_default();
        match std::fs::write(&path, &content) {
            Ok(_) => info!("Evidence written: {}", path.display()),
            Err(e) => tracing::warn!("Failed to write evidence {}: {}", path.display(), e),
        }
        path.to_string_lossy().to_string()
    }
}

impl Default for EvidenceWriter {
    fn default() -> Self {
        Self::new()
    }
}
