//! Config loader for `model-profiles.json`.
//!
//! Reads and validates the model profile configuration file.
//! The router reads this at startup only — changes require a restart.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

/// Router configuration with environment variable overrides.
#[derive(Debug, Clone)]
pub struct RouterConfig {
    pub router_host: String,
    pub router_port: u16,
    pub backend_port_base: u16,
    pub auth_token: Option<String>,
    pub require_auth: bool,
    pub max_body_bytes: usize,
    pub profile_config_path: Option<PathBuf>,
    pub backend_binary_path: Option<PathBuf>,
    pub evidence_path: Option<PathBuf>,
    pub log_path: Option<PathBuf>,
    pub health_timeout_secs: u64,
    /// Timeout for each individual health check HTTP request (default 5s).
    /// Separate from health_timeout_secs which gates backend startup.
    pub health_check_timeout_secs: u64,
    pub health_poll_interval_secs: u64,
}

impl RouterConfig {
    /// Load configuration from environment variables with defaults.
    pub fn from_env() -> Self {
        RouterConfig {
            router_host: std::env::var("ROUTER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            router_port: std::env::var("ROUTER_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(9130),
            backend_port_base: std::env::var("BACKEND_PORT_BASE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(9120),
            auth_token: std::env::var("ROUTER_AUTH_TOKEN").ok(),
            require_auth: std::env::var("ROUTER_REQUIRE_AUTH")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            max_body_bytes: std::env::var("ROUTER_MAX_BODY_BYTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10 * 1024 * 1024),
            profile_config_path: std::env::var("PROFILE_CONFIG_PATH").ok().map(PathBuf::from),
            backend_binary_path: std::env::var("BACKEND_BINARY_PATH").ok().map(PathBuf::from),
            evidence_path: std::env::var("EVIDENCE_PATH").ok().map(PathBuf::from),
            log_path: std::env::var("LOG_PATH").ok().map(PathBuf::from),
            health_timeout_secs: std::env::var("HEALTH_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(180),
            health_check_timeout_secs: std::env::var("HEALTH_CHECK_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            health_poll_interval_secs: std::env::var("HEALTH_POLL_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
        }
    }
}

/// A single model profile from the configuration file.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Profile {
    pub alias: String,
    pub model_file: String,
    pub model_path: String,
    #[serde(default)]
    pub gguf_size_gb: f64,
    #[serde(default = "default_backend")]
    pub backend: String,
    #[serde(default = "default_ngl")]
    pub ngl: u32,
    #[serde(default = "default_context")]
    pub context: u32,
    pub port: u16,
    pub launch_command: Option<String>,
    #[serde(default)]
    pub task_classes: Vec<String>,
    #[serde(default = "default_verified_status")]
    pub verified_status: String,
    pub evidence_path: Option<String>,
    #[serde(default = "default_authority_status")]
    pub authority_status: String,
    #[serde(default)]
    pub limitations: String,
    #[serde(default)]
    pub known_behavior: String,
    #[serde(default)]
    pub test_cells: Vec<String>,
}

fn default_backend() -> String { "vulkan".to_string() }
fn default_ngl() -> u32 { 99 }
fn default_context() -> u32 { 1024 }
fn default_verified_status() -> String { "unverified".to_string() }
fn default_authority_status() -> String { "advisory_only".to_string() }

/// Top-level config file structure.
#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub _meta: serde_json::Value,
    #[serde(default)]
    pub defaults: serde_json::Value,
    pub profiles: Vec<Profile>,
}

/// In-memory profile store, indexed by alias.
/// The router creates this once at startup.
#[derive(Debug, Clone)]
pub struct ProfileManager {
    profiles: HashMap<String, Profile>,
    profile_list: Vec<Profile>,
}

impl ProfileManager {
    /// Load profiles from `model-profiles.json` at the given path.
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config '{}': {}", path.display(), e))?;
        let config: RuntimeConfig = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config '{}': {}", path.display(), e))?;

        let mut profiles = HashMap::new();
        for p in &config.profiles {
            profiles.insert(p.alias.clone(), p.clone());
        }

        Ok(ProfileManager {
            profiles,
            profile_list: config.profiles,
        })
    }

    /// Try profile sources in order. First match wins.
    pub fn load_from_sources(sources: &[&Path]) -> Result<Self, String> {
        let mut last_err = String::new();
        for source in sources {
            if source.exists() {
                match Self::load(source) {
                    Ok(pm) => return Ok(pm),
                    Err(e) => last_err = format!("{}: {}", source.display(), e),
                }
            }
        }
        Err(format!(
            "No profile source found. Tried {} sources. Last error: {}",
            sources.len(),
            last_err
        ))
    }

    /// Load profiles using RouterConfig for path overrides.
    pub fn load_from_config(config: &RouterConfig) -> Result<Self, String> {
        let mut sources: Vec<PathBuf> = Vec::new();

        // Custom profile config path from env/config takes highest priority
        if let Some(ref custom) = config.profile_config_path {
            sources.push(custom.clone());
        }

        // Default sources matching Python router
        let runtime_config = PathBuf::from(r"G:\openwork\librarian-runtime-node\config\model-profiles.json");
        let librarian_config = PathBuf::from(
            r"G:\openwork\thelibrarian\fixtures\windows-runtime-node\router\model-profiles.json",
        );

        sources.push(runtime_config);
        sources.push(librarian_config);

        let source_refs: Vec<&Path> = sources.iter().map(|p| p.as_path()).collect();
        Self::load_from_sources(&source_refs)
    }

    pub fn get(&self, alias: &str) -> Option<&Profile> {
        self.profiles.get(alias)
    }

    pub fn list_all(&self) -> Vec<serde_json::Value> {
        self.profile_list
            .iter()
            .map(|p| {
                serde_json::json!({
                    "alias": p.alias,
                    "task_classes": p.task_classes,
                    "verified": p.verified_status == "verified",
                    "port": p.port,
                    "model_file": p.model_file,
                    "backend": p.backend,
                    "context": p.context,
                    "ngl": p.ngl,
                    "evidence_path": p.evidence_path,
                    "limitations": p.limitations,
                })
            })
            .collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Profile> {
        self.profile_list.iter()
    }

    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }

    pub fn aliases(&self) -> Vec<String> {
        self.profiles.keys().cloned().collect()
    }
}
