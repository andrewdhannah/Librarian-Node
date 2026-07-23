//! # Linux Service Management
//!
//! Linux-specific service unit management for systemd.
//! Generates systemd unit files and manages service registration.
//! All governance events are emitted through the RuntimeAdapter boundary.

use std::path::PathBuf;

/// A systemd service unit for a Librarian component.
#[derive(Debug, Clone)]
pub struct SystemdUnit {
    /// Component identifier.
    pub component_id: String,
    /// Path to the executable binary.
    pub binary_path: PathBuf,
    /// Arguments to pass to the binary.
    pub args: Vec<String>,
    /// Working directory.
    pub working_dir: PathBuf,
    /// User to run as.
    pub run_as_user: String,
    /// Description for the unit file.
    pub description: String,
}

impl SystemdUnit {
    /// Generate the systemd unit file content.
    pub fn generate_unit_file(&self) -> String {
        let args_str = self.args.join(" ");
        format!(
            "[Unit]\n\
             Description={desc}\n\
             After=network.target\n\
             \n\
             [Service]\n\
             Type=simple\n\
             User={user}\n\
             WorkingDirectory={workdir}\n\
             ExecStart={binary} {args}\n\
             Restart=on-failure\n\
             RestartSec=5\n\
             StandardOutput=journal\n\
             StandardError=journal\n\
             \n\
             [Install]\n\
             WantedBy=multi-user.target\n",
            desc = self.description,
            user = self.run_as_user,
            workdir = self.working_dir.display(),
            binary = self.binary_path.display(),
            args = args_str,
        )
    }

    /// Get the expected unit file path.
    pub fn unit_path(&self) -> PathBuf {
        PathBuf::from(format!(
            "/etc/systemd/system/librarian-{}.service",
            self.component_id
        ))
    }

    /// Get the service name (for systemctl commands).
    pub fn service_name(&self) -> String {
        format!("librarian-{}.service", self.component_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_unit() -> SystemdUnit {
        SystemdUnit {
            component_id: "node".into(),
            binary_path: PathBuf::from("/usr/bin/librarian-node"),
            args: vec!["--port".into(), "3456".into()],
            working_dir: PathBuf::from("/var/lib/librarian"),
            run_as_user: "librarian".into(),
            description: "Librarian Runtime Node".into(),
        }
    }

    #[test]
    fn test_unit_file_generation() {
        let unit = sample_unit();
        let content = unit.generate_unit_file();
        assert!(content.contains("Description=Librarian Runtime Node"));
        assert!(content.contains("ExecStart=/usr/bin/librarian-node --port 3456"));
        assert!(content.contains("Restart=on-failure"));
        assert!(content.contains("WantedBy=multi-user.target"));
        assert!(content.contains("User=librarian"));
    }

    #[test]
    fn test_unit_path() {
        let unit = sample_unit();
        let path = unit.unit_path();
        assert_eq!(path, PathBuf::from("/etc/systemd/system/librarian-node.service"));
    }

    #[test]
    fn test_service_name() {
        let unit = sample_unit();
        assert_eq!(unit.service_name(), "librarian-node.service");
    }

    #[test]
    fn test_unit_has_restart_policy() {
        let unit = sample_unit();
        let content = unit.generate_unit_file();
        assert!(content.contains("Restart=on-failure"));
        assert!(content.contains("RestartSec=5"));
    }
}
