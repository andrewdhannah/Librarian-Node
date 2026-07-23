//! Backend process manager — launches, monitors, and stops llama.cpp backends.
//!
//! Each profile gets its own `BackendProcess` that manages the child
//! `llama-server.exe` process lifecycle. The router holds a map of these,
//! keyed by profile alias.
//!
use crate::config::{Profile, RouterConfig};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{error, info, warn};

/// Windows: CREATE_NO_WINDOW = 0x08000000 to avoid console window for backend.
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(not(target_os = "windows"))]
const CREATE_NO_WINDOW: u32 = 0;

/// Timeout for backend chat requests.
const BACKEND_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendState {
    Stopped,
    Starting,
    Healthy,
    Degraded,
    Failed,
}

impl BackendState {
    pub fn is_healthy(&self) -> bool {
        matches!(self, BackendState::Healthy)
    }

    pub fn is_available_for_chat(&self) -> bool {
        matches!(self, BackendState::Healthy)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            BackendState::Stopped => "stopped",
            BackendState::Starting => "starting",
            BackendState::Healthy => "healthy",
            BackendState::Degraded => "degraded",
            BackendState::Failed => "failed",
        }
    }
}

/// Runtime status of a single backend process.
#[derive(Debug, Serialize)]
pub struct BackendStatus {
    pub alias: String,
    pub state: String,
    pub pid: Option<u32>,
    pub port: u16,
    pub uptime_seconds: Option<u64>,
    pub health_fail_count: u32,
    pub error: Option<String>,
}

/// Manages a single `llama-server.exe` child process.
pub struct BackendProcess {
    pub alias: String,
    pub profile: Profile,
    config: RouterConfig,
    child: Arc<Mutex<Option<Child>>>,
    pub state: Arc<Mutex<BackendState>>,
    pub start_time: Arc<Mutex<Option<Instant>>>,
    pub health_fail_count: Arc<Mutex<u32>>,
    pub error_message: Arc<Mutex<Option<String>>>,
}

impl BackendProcess {
    pub fn new(profile: Profile, config: RouterConfig) -> Self {
        BackendProcess {
            alias: profile.alias.clone(),
            profile,
            config,
            child: Arc::new(Mutex::new(None)),
            state: Arc::new(Mutex::new(BackendState::Stopped)),
            start_time: Arc::new(Mutex::new(None)),
            health_fail_count: Arc::new(Mutex::new(0)),
            error_message: Arc::new(Mutex::new(None)),
        }
    }

    /// State accessors for lock-free reads in response handlers.
    pub async fn get_state(&self) -> BackendState {
        *self.state.lock().await
    }

    pub async fn get_status(&self) -> BackendStatus {
        let state = *self.state.lock().await;
        let pid = {
            let guard = self.child.lock().await;
            guard.as_ref().and_then(|c| c.id())
        };
        let start_time = *self.start_time.lock().await;
        let uptime = start_time.map(|t| t.elapsed().as_secs());
        let health_fail_count = *self.health_fail_count.lock().await;
        let error = self.error_message.lock().await.clone();

        BackendStatus {
            alias: self.alias.clone(),
            state: state.as_str().to_string(),
            pid,
            port: self.profile.port,
            uptime_seconds: uptime,
            health_fail_count,
            error,
        }
    }

    /// Start the backend process and wait for it to become healthy.
    pub async fn start(&self) -> Result<(), String> {
        let mut state = self.state.lock().await;
        let mut child_guard = self.child.lock().await;
        let mut start_time_guard = self.start_time.lock().await;

        // Check if already running
        if let Some(child) = child_guard.as_mut() {
            match child.try_wait() {
                Ok(Some(_)) => { /* exited, will restart */ }
                Ok(None) => {
                    info!("[{}] already running", self.alias);
                    return Ok(());
                }
                Err(_) => {}
            }
        }

        *state = BackendState::Starting;
        *self.error_message.lock().await = None;
        *self.health_fail_count.lock().await = 0;

        let binary_default = PathBuf::from(r"G:\openwork\librarian-runtime-node\runtime\llama.cpp\llama-server.exe");
        let binary = self.config.backend_binary_path.as_ref()
            .unwrap_or(&binary_default);
        if !binary.exists() {
            *state = BackendState::Failed;
            let msg = format!("Binary not found: {}", binary.display());
            *self.error_message.lock().await = Some(msg.clone());
            error!("[{}] {}", self.alias, msg);
            return Err(msg);
        }

        let cmd_str = format!(
            "{} -m \"{}\" -p {} -c {} -ngl {} -n 512 --alias \"{}\"",
            binary.display(),
            self.profile.model_path,
            self.profile.port,
            self.profile.context,
            self.profile.ngl,
            self.alias,
        );
        info!("[{}] launching: {}", self.alias, cmd_str);

        // Use logs/ subdirectory (matching Python router's log location).
        // Create the directory if it doesn't exist.
        let logs_dir = PathBuf::from("logs");
        std::fs::create_dir_all(&logs_dir).ok();
        let log_path = logs_dir.join(format!("backend_{}.log", self.alias));
        let log_file = std::fs::File::create(&log_path)
            .map_err(|e| format!("Failed to create log file '{}': {}", log_path.display(), e))?;

        let child = Command::new(binary)
            .arg("-m")
            .arg(&self.profile.model_path)
            .arg("-p")
            .arg(self.profile.port.to_string())
            .arg("-c")
            .arg(self.profile.context.to_string())
            .arg("-ngl")
            .arg(self.profile.ngl.to_string())
            .arg("-n")
            .arg("512")
            .arg("--alias")
            .arg(&self.alias)
            .stdout(log_file.try_clone().map_err(|e| format!("Failed to clone log file: {}", e))?)
            .stderr(log_file)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to spawn backend: {}", e))?;

        let pid = child.id().unwrap_or(0);
        info!("[{}] started (PID {}, port {})", self.alias, pid, self.profile.port);
        *child_guard = Some(child);
        *start_time_guard = Some(Instant::now());

        // Drop locks before the wait loop
        drop(child_guard);
        drop(start_time_guard);
        drop(state);

        // Wait for health
        let deadline = Instant::now() + Duration::from_secs(self.config.health_timeout_secs);
        let mut last_log = Instant::now();

        while Instant::now() < deadline {
            // Check if process exited
            {
                let mut guard = self.child.lock().await;
                if let Some(child) = guard.as_mut() {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            *self.state.lock().await = BackendState::Failed;
                            let msg = format!("Process exited (code {})", status);
                            *self.error_message.lock().await = Some(msg.clone());
                            error!("[{}] {}", self.alias, msg);
                            return Err(msg);
                        }
                        _ => {}
                    }
                }
            }

            if self.check_health().await {
                *self.state.lock().await = BackendState::Healthy;
                let elapsed = self.start_time.lock().await
                    .map(|t| t.elapsed().as_secs_f64())
                    .unwrap_or(0.0);
                info!("[{}] healthy after {:.1}s", self.alias, elapsed);
                return Ok(());
            }

            if last_log.elapsed() > Duration::from_secs(10) {
                info!("[{}] waiting for health...", self.alias);
                last_log = Instant::now();
            }

            sleep(Duration::from_secs(2)).await;
        }

        *self.state.lock().await = BackendState::Failed;
        let msg = format!("Timed out after {}s waiting for health", self.config.health_timeout_secs);
        *self.error_message.lock().await = Some(msg.clone());
        error!("[{}] {}", self.alias, msg);
        Err(msg)
    }

    /// Stop the backend process with graceful termination sequence.
    ///
    /// Sequence:
    ///   1. Start kill (SIGTERM/TerminateProcess)
    ///   2. Wait up to 5 seconds for the process to exit
    ///   3. If still alive, force kill
    ///   4. Set state to Stopped, release the child handle
    ///
    /// This matches the Python router's `terminate() -> wait(5s) -> kill()` pattern.
    pub async fn stop(&self) {
        let mut child_guard = self.child.lock().await;
        let mut state = self.state.lock().await;

        if let Some(child) = child_guard.as_mut() {
            let pid = child.id().unwrap_or(0);
            info!("[{}] stopping (PID {})", self.alias, pid);

            // Step 1: Start graceful termination
            match child.start_kill() {
                Ok(()) => {
                    info!("[{}] terminate signal sent to PID {}", self.alias, pid);

                    // Step 2: Wait up to 5 seconds for graceful exit
                    let deadline = Instant::now() + Duration::from_secs(5);
                    let mut exited_cleanly = false;

                    while Instant::now() < deadline {
                        match child.try_wait() {
                            Ok(Some(_)) => {
                                exited_cleanly = true;
                                break;
                            }
                            Ok(None) => {}
                            Err(e) => {
                                warn!("[{}] wait error: {}", self.alias, e);
                                break;
                            }
                        }
                        sleep(Duration::from_millis(200)).await;
                    }

                    if exited_cleanly {
                        info!("[{}] process exited cleanly", self.alias);
                    } else {
                        // Step 3: Force kill
                        warn!("[{}] graceful shutdown timed out (5s), forcing kill", self.alias);
                        match child.kill().await {
                            Ok(()) => info!("[{}] force kill succeeded", self.alias),
                            Err(e) => warn!("[{}] force kill failed: {}", self.alias, e),
                        }
                    }
                }
                Err(e) => {
                    warn!("[{}] terminate failed, forcing kill: {}", self.alias, e);
                    if let Err(kill_err) = child.kill().await {
                        warn!("[{}] force kill also failed: {}", self.alias, kill_err);
                    }
                }
            }
        } else {
            info!("[{}] no running process to stop", self.alias);
        }

        *state = BackendState::Stopped;
        *child_guard = None;
        info!("[{}] stopped", self.alias);
    }

    /// Restart the backend process: stop -> start -> wait healthy.
    /// Returns Ok(()) if restart succeeds and backend becomes healthy.
    /// Returns Err(String) if restart fails, leaving no orphan process.
    pub async fn restart(&self) -> Result<(), String> {
        info!("[{}] restarting...", self.alias);

        // Stop first (this cleans up any existing process)
        self.stop().await;

        // Small delay to ensure port is released
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Start again
        self.start().await
    }

    /// Poll the backend health endpoint. Returns true if healthy.
    ///
    /// Uses a tight timeout (health_check_timeout_secs, default 5s) so the
    /// health poller does not block for 3 minutes if a backend hangs.
    /// Startup health wait uses the separate health_timeout_secs (default 180s).
    pub async fn check_health(&self) -> bool {
        let url = format!("http://127.0.0.1:{}/health", self.profile.port);

        match reqwest::Client::builder()
            .timeout(Duration::from_secs(self.config.health_check_timeout_secs))
            .build()
            .ok()
        {
            Some(client) => {
                match client.get(&url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        let body: serde_json::Value = resp.json().await.unwrap_or_default();
                        let status_str = body.get("status").and_then(|v| v.as_str()).unwrap_or("");
                        if status_str == "ok" {
                            *self.health_fail_count.lock().await = 0;
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            None => {}
        }

        // Increment fail count
        let mut count = self.health_fail_count.lock().await;
        *count += 1;
        if *count >= 3 {
            let mut state = self.state.lock().await;
            match *state {
                BackendState::Healthy => {
                    *state = BackendState::Degraded;
                    *self.error_message.lock().await = Some("Consecutive health check failures".to_string());
                    warn!("[{}] degraded after {} health failures", self.alias, *count);
                }
                BackendState::Degraded => {
                    *state = BackendState::Failed;
                    *self.error_message.lock().await = Some("Consecutive health check failures — state set to failed".to_string());
                    warn!("[{}] failed after {} health failures while degraded", self.alias, *count);
                }
                _ => {}
            }
        }
        false
    }

    /// Verify model identity against /health and /v1/models.
    ///
    /// Checks that the backend's health endpoint reports the correct model alias
    /// AND the /v1/models endpoint returns the matching model id.
    /// Matches Python router's `ProcessManager.verify_identity()`.
    ///
    /// Returns (true, "") if identity matches, or (false, "detail message") on mismatch.
    pub async fn verify_identity(&self) -> (bool, String) {
        let health_timeout = Duration::from_secs(self.config.health_check_timeout_secs);

        let client = match reqwest::Client::builder()
            .timeout(health_timeout)
            .build()
        {
            Ok(c) => c,
            Err(e) => return (false, format!("Failed to create HTTP client: {}", e)),
        };

        // 1. Check /health reports correct model alias
        let health_url = format!("http://127.0.0.1:{}/health", self.profile.port);
        match client.get(&health_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                let reported_alias = body.get("model").and_then(|v| v.as_str()).unwrap_or("");
                if !reported_alias.is_empty() && reported_alias != self.alias {
                    let detail = format!(
                        "/health reports '{}', expected '{}'",
                        reported_alias, self.alias
                    );
                    return (false, detail);
                }
            }
            Ok(resp) => {
                return (false, format!("/health returned HTTP {}", resp.status()));
            }
            Err(e) => {
                return (false, format!("/health request failed: {}", e));
            }
        }

        // 2. Check /v1/models returns matching model id
        let models_url = format!("http://127.0.0.1:{}/v1/models", self.profile.port);
        match client.get(&models_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                let models_id = body
                    .get("data")
                    .and_then(|d| d.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|m| m.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !models_id.is_empty() && models_id != self.alias {
                    let detail = format!(
                        "/v1/models reports '{}', expected '{}'",
                        models_id, self.alias
                    );
                    return (false, detail);
                }
            }
            Ok(resp) => {
                return (false, format!("/v1/models returned HTTP {}", resp.status()));
            }
            Err(e) => {
                return (false, format!("/v1/models request failed: {}", e));
            }
        }

        (true, String::new())
    }

    /// Proxy a chat request to the backend's OpenAI-compatible endpoint.
    pub async fn proxy_chat(
        &self,
        messages: &[serde_json::Value],
        max_tokens: u32,
        temperature: f64,
    ) -> Result<serde_json::Value, String> {
        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.profile.port);

        let body = serde_json::json!({
            "messages": messages,
            "max_tokens": max_tokens,
            "temperature": temperature,
            "stream": false,
        });

        let client = reqwest::Client::builder()
            .timeout(BACKEND_REQUEST_TIMEOUT)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Backend request failed: {}", e))?;

        let status = resp.status();
        let response_body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse backend response: {}", e))?;

        if !status.is_success() {
            return Err(format!("Backend returned {}: {}", status, response_body));
        }

        Ok(response_body)
    }
}
