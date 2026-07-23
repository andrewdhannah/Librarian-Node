/// System profile — Mac canonical.
///
/// Describes the Mac-side system context for cross-machine comparison.
/// References Windows hardware_profiles for GPU-specific data.

/// System profile record — Mac canonical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemProfile {
    /// Unique identifier for this system profile.
    pub system_profile_id: String,

    /// Operating system (e.g., "macos-14.5", "windows-11-23H2").
    pub os: Option<String>,

    /// CPU description (e.g., "Intel Core i5-3570K", "Apple M1 Pro").
    pub cpu: Option<String>,

    /// System RAM in MB.
    pub ram_mb: Option<i64>,

    /// GPU description (free-text, may differ from hardware_profile.device_name).
    pub gpu_description: Option<String>,

    /// Free-text notes about the system configuration.
    pub notes: Option<String>,

    /// When this profile was created.
    pub created_at: String,
}

impl SystemProfile {
    /// Create a new system profile.
    pub fn new(system_profile_id: String) -> Self {
        Self {
            system_profile_id,
            os: None,
            cpu: None,
            ram_mb: None,
            gpu_description: None,
            notes: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_system_profile() {
        let profile = SystemProfile::new("sys-1".to_string());
        assert_eq!(profile.system_profile_id, "sys-1");
        assert!(profile.os.is_none());
        assert!(profile.cpu.is_none());
        assert!(profile.ram_mb.is_none());
        assert!(profile.gpu_description.is_none());
    }

    #[test]
    fn test_system_profile_with_fields() {
        let mut profile = SystemProfile::new("sys-2".to_string());
        profile.os = Some("windows-11".to_string());
        profile.cpu = Some("Intel Core i5-3570K".to_string());
        profile.ram_mb = Some(24268);
        profile.gpu_description = Some("AMD Radeon RX 570".to_string());
        profile.notes = Some("Big Pickle Windows node".to_string());

        assert_eq!(profile.os, Some("windows-11".to_string()));
        assert_eq!(profile.cpu, Some("Intel Core i5-3570K".to_string()));
        assert_eq!(profile.ram_mb, Some(24268));
    }
}
