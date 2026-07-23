/// Task pack — Mac canonical.
///
/// Versioned work fixtures and prompts for qualification runs.
/// Each task_pack captures a specific test scenario with version control.

/// Task pack status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskPackStatus {
    /// Active — available for qualification runs.
    Active,
    /// Deprecated — no longer used for new runs.
    Deprecated,
}

impl TaskPackStatus {
    pub fn as_str(&self) -> &str {
        match self {
            TaskPackStatus::Active => "active",
            TaskPackStatus::Deprecated => "deprecated",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "active" => TaskPackStatus::Active,
            "deprecated" => TaskPackStatus::Deprecated,
            _ => TaskPackStatus::Active,
        }
    }
}

/// Task pack — versioned work fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskPack {
    /// Unique identifier for this task pack.
    pub task_pack_id: String,

    /// Version number (monotonically increasing).
    pub version: i64,

    /// Target role for this fixture (e.g., "implementer", "researcher").
    pub role: String,

    /// Human-readable description.
    pub description: Option<String>,

    /// SHA-256 hash of the fixture content.
    pub fixture_hash: String,

    /// Filesystem path to the fixture file.
    pub fixture_path: Option<String>,

    /// Task pack status.
    pub status: TaskPackStatus,

    /// When this task pack was created.
    pub created_at: String,
}

impl TaskPack {
    /// Create a new task pack.
    pub fn new(
        task_pack_id: String,
        version: i64,
        role: String,
        fixture_hash: String,
    ) -> Self {
        Self {
            task_pack_id,
            version,
            role,
            description: None,
            fixture_hash,
            fixture_path: None,
            status: TaskPackStatus::Active,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_task_pack() {
        let pack = TaskPack::new(
            "tp-1".to_string(),
            1,
            "implementer".to_string(),
            "abc123".to_string(),
        );
        assert_eq!(pack.task_pack_id, "tp-1");
        assert_eq!(pack.version, 1);
        assert_eq!(pack.role, "implementer");
        assert_eq!(pack.status, TaskPackStatus::Active);
    }

    #[test]
    fn test_task_pack_status_roundtrip() {
        let active = TaskPackStatus::Active;
        assert_eq!(TaskPackStatus::from_str(active.as_str()), active);

        let deprecated = TaskPackStatus::Deprecated;
        assert_eq!(TaskPackStatus::from_str(deprecated.as_str()), deprecated);
    }
}
