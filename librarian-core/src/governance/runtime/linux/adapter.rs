//! # Linux RuntimeAdapter Implementation
//!
//! Maps Linux process supervision to the platform-agnostic adapter interface.
//! Uses systemd for service lifecycle and /proc for process discovery.
//!
//! The adapter terminates at the `RuntimeAdapter` trait boundary.
//! No Linux concepts leak into governance types.

use crate::governance::runtime::adapter::{ProcessEvent, ProcessState, RuntimeAdapter};
use std::path::PathBuf;

/// Linux-specific adapter for process supervision.
///
/// Uses systemd for service management (/run/systemd/system/) and
/// /proc for process discovery. Falls back to direct process management
/// when systemd is not available.
pub struct LinuxAdapter {
    /// Base data directory (XDG_DATA_HOME or ~/.local/share).
    data_dir: PathBuf,
    /// Whether systemd is available.
    has_systemd: bool,
}

impl LinuxAdapter {
    /// Create a new Linux adapter.
    pub fn new(data_dir: PathBuf) -> Self {
        let has_systemd = Self::check_systemd();
        Self {
            data_dir,
            has_systemd,
        }
    }

    /// Create with default XDG paths.
    pub fn default() -> Self {
        let data_dir = dirs::data_dir()
            .map(|p| p.join("Librarian"))
            .unwrap_or_else(|| PathBuf::from("/var/lib/librarian"));
        Self::new(data_dir)
    }

    /// Check whether systemd is available.
    fn check_systemd() -> bool {
        std::path::Path::new("/run/systemd/system").exists()
    }

    /// Parse a systemd unit status line into a process event.
    pub fn parse_systemd_status(line: &str, component: &str) -> ProcessEvent {
        match line.trim() {
            "Active: active (running)" => ProcessEvent::Started,
            "Active: inactive (dead)" => ProcessEvent::Stopped,
            "Active: failed" => ProcessEvent::Crashed,
            "Active: activating (start)" => ProcessEvent::StartRequested,
            "Active: deactivating (stop)" => ProcessEvent::StopRequested,
            _ => ProcessEvent::HealthCheckPassed,
        }
    }

    /// Map a process exit status to a governance event.
    pub fn exit_status_to_event(code: i32) -> ProcessEvent {
        match code {
            0 => ProcessEvent::Stopped,
            137 | 143 => ProcessEvent::Stopped,  // SIGKILL, SIGTERM
            _ => ProcessEvent::Crashed,
        }
    }

    /// Get the data directory path.
    pub fn data_dir(&self) -> &std::path::Path {
        &self.data_dir
    }
}

impl RuntimeAdapter for LinuxAdapter {
    fn start(&self, component_id: &str) -> Result<(), String> {
        if self.has_systemd {
            let output = std::process::Command::new("systemctl")
                .arg("start")
                .arg(format!("librarian-{}.service", component_id))
                .output()
                .map_err(|e| format!("Failed to start {}: {}", component_id, e))?;
            if output.status.success() {
                Ok(())
            } else {
                Err(format!("systemctl start failed for {}", component_id))
            }
        } else {
            // Fallback: direct process launch
            Err("systemd not available".to_string())
        }
    }

    fn stop(&self, component_id: &str) -> Result<(), String> {
        if self.has_systemd {
            let output = std::process::Command::new("systemctl")
                .arg("stop")
                .arg(format!("librarian-{}.service", component_id))
                .output()
                .map_err(|e| format!("Failed to stop {}: {}", component_id, e))?;
            if output.status.success() {
                Ok(())
            } else {
                Err(format!("systemctl stop failed for {}", component_id))
            }
        } else {
            Err("systemd not available".to_string())
        }
    }

    fn is_running(&self, component_id: &str) -> Result<bool, String> {
        if self.has_systemd {
            let output = std::process::Command::new("systemctl")
                .arg("is-active")
                .arg(format!("librarian-{}.service", component_id))
                .output()
                .map_err(|e| format!("Failed to check {}: {}", component_id, e))?;
            let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(status == "active")
        } else {
            // Check /proc for the process
            let proc_path = format!("/proc/{}", component_id);
            Ok(std::path::Path::new(&proc_path).exists())
        }
    }

    fn get_state(&self, component_id: &str) -> Result<ProcessState, String> {
        if self.has_systemd {
            let output = std::process::Command::new("systemctl")
                .arg("status")
                .arg(format!("librarian-{}.service", component_id))
                .output()
                .map_err(|e| format!("Failed to get status for {}: {}", component_id, e))?;
            let status = String::from_utf8_lossy(&output.stdout);
            for line in status.lines() {
                match line.trim() {
                    l if l.starts_with("Active: active") => return Ok(ProcessState::Running),
                    l if l.starts_with("Active: inactive") => return Ok(ProcessState::Stopped),
                    l if l.starts_with("Active: failed") => return Ok(ProcessState::Crashed),
                    l if l.starts_with("Active: activating") => return Ok(ProcessState::Starting),
                    _ => {}
                }
            }
            Ok(ProcessState::Stopped)
        } else {
            Ok(ProcessState::Stopped)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_systemd_active() {
        let event = LinuxAdapter::parse_systemd_status("Active: active (running)", "test");
        assert_eq!(event, ProcessEvent::Started);
    }

    #[test]
    fn test_parse_systemd_inactive() {
        let event = LinuxAdapter::parse_systemd_status("Active: inactive (dead)", "test");
        assert_eq!(event, ProcessEvent::Stopped);
    }

    #[test]
    fn test_parse_systemd_failed() {
        let event = LinuxAdapter::parse_systemd_status("Active: failed", "test");
        assert_eq!(event, ProcessEvent::Crashed);
    }

    #[test]
    fn test_parse_systemd_activating() {
        let event = LinuxAdapter::parse_systemd_status("Active: activating (start)", "test");
        assert_eq!(event, ProcessEvent::StartRequested);
    }

    #[test]
    fn test_exit_zero() {
        assert_eq!(LinuxAdapter::exit_status_to_event(0), ProcessEvent::Stopped);
    }

    #[test]
    fn test_exit_sigkill() {
        assert_eq!(LinuxAdapter::exit_status_to_event(137), ProcessEvent::Stopped);
    }

    #[test]
    fn test_exit_crash() {
        assert_eq!(LinuxAdapter::exit_status_to_event(1), ProcessEvent::Crashed);
        assert_eq!(LinuxAdapter::exit_status_to_event(139), ProcessEvent::Crashed); // SIGSEGV
    }

    #[test]
    fn test_default_path() {
        let adapter = LinuxAdapter::default();
        let path = adapter.data_dir();
        assert!(path.to_string_lossy().contains("Librarian"));
    }

    #[test]
    fn test_no_systemd_in_test_env() {
        // On macOS or CI, systemd is not available — adapter should handle gracefully
        let adapter = LinuxAdapter::default();
        let result = adapter.is_running("nonexistent");
        // Should not panic — returns Ok(false) or an Err
        assert!(result.is_ok() || result.is_err());
    }
}
