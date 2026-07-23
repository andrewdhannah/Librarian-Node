#!/usr/bin/env python3
"""
llama-router — Windows Runtime Router Implementation (WIN-ROUTER-IMPL-1)

Governed multi-model router for the Big Pickle (RX 570 4 GB Polaris) runtime node.
Implements WIN-ROUTER-CONTRACT-1 contract endpoints and refusal conditions.

Usage:
    python router.py [--port 8080] [--profiles <path>]

Authority: advisory_only
"""

import argparse
import json
import logging
import os
import signal
import subprocess
import sys
import threading
import time
import urllib.request
import urllib.error
from datetime import datetime, timezone
from http.server import HTTPServer, ThreadingHTTPServer, BaseHTTPRequestHandler
from pathlib import Path

# ============================================================================
# Configuration
# ============================================================================

DEFAULT_PORT = 8080
RUNTIME_NODE = Path(r"G:\openwork\librarian-runtime-node")
LIBRARIAN = Path(r"G:\openwork\thelibrarian")
BINARY = RUNTIME_NODE / r"runtime\llama.cpp\llama-server.exe"
MODELS_DIR = Path(r"G:\llama.cpp\models")
FIXTURES_DIR = LIBRARIAN / r"fixtures\windows-runtime-node\router-impl"
EVIDENCE_DIR = FIXTURES_DIR

# Profile sources (try runtime copy first, fall back to canonical)
PROFILE_SOURCES = [
    RUNTIME_NODE / r"config\model-profiles.json",
    LIBRARIAN / r"fixtures\windows-runtime-node\router\model-profiles.json",
]

HEALTH_POLL_SECONDS = 5
BACKEND_START_TIMEOUT_SECONDS = 180
BACKEND_HEALTH_TIMEOUT_SECONDS = 5
BACKEND_REQUEST_TIMEOUT_SECONDS = 120
MAX_BODY_BYTES = 64 * 1024  # 64 KB maximum request body size


class RequestTooLarge(Exception):
    """Raised when request body exceeds MAX_BODY_BYTES."""
    pass


logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    datefmt="%H:%M:%S",
)
log = logging.getLogger("router")

# ============================================================================
# Refusal Conditions
# ============================================================================

REFUSAL_REASONS = {
    "unknown_profile": "No profile registered with alias '{alias}'",
    "unverified_profile": "Profile '{alias}' has verified_status='{status}'. Must pass WIN-MODEL-FIT matrix first.",
    "identity_mismatch": "Running model on port {port} reports '{reported}' but profile alias is '{alias}'",
    "context_exceeds_verified": "Requested context {requested} exceeds verified max {verified} for profile '{alias}'",
    "authority_required": "This request implies authority beyond advisory. Model output is advisory only.",
    "file_mutation_forbidden": "File mutation or promotion to Librarian directory is forbidden.",
    "runtime_unhealthy": "Runtime for profile '{alias}' on port {port} is not healthy.",
    "autonomous_action_forbidden": "Autonomous action is forbidden. The router is a dispatcher, not a decision-maker.",
}

# Authority-bearing keywords that trigger refusal
AUTHORITY_KEYWORDS = [
    "approve", "promote", "commit", "escalate", "authorize",
    "mark valid", "override policy", "ignore policy",
    "autonomous", "self-directed", "automatic decision",
    "edit source", "modify file", "write to librarian",
]

# ============================================================================
# Profile Manager
# ============================================================================

class ProfileManager:
    """Loads and serves model profiles."""

    def __init__(self):
        self.profiles = {}  # alias -> profile dict
        self._load_profiles()

    def _load_profiles(self):
        loaded = False
        for path in PROFILE_SOURCES:
            if path.exists():
                try:
                    data = json.loads(path.read_text(encoding="utf-8"))
                    raw_profiles = data.get("profiles", data if isinstance(data, list) else [])
                    for p in raw_profiles:
                        alias = p.get("alias", p.get("name"))
                        if alias:
                            self.profiles[alias] = p
                    log.info("Loaded %d profiles from %s", len(self.profiles), path)
                    loaded = True
                    break
                except (json.JSONDecodeError, KeyError) as e:
                    log.warning("Failed to parse %s: %s", path, e)

        if not loaded:
            log.error("No profile source found. Tried: %s", PROFILE_SOURCES)

    def get(self, alias):
        return self.profiles.get(alias)

    def list_all(self):
        return [
            {
                "alias": p["alias"],
                "task_classes": p.get("task_classes", []),
                "verified": p.get("verified_status") == "verified",
                "port": p.get("port", 0),
                "model_file": p.get("model_file", ""),
            }
            for p in self.profiles.values()
        ]

    def to_json_dict(self):
        return {alias: {
            k: v for k, v in p.items()
            if k in ("alias", "model_file", "model_path", "backend", "ngl",
                     "context", "port", "launch_command", "task_classes",
                     "verified_status", "evidence_path", "authority_status",
                     "limitations", "known_behavior")
        } for alias, p in self.profiles.items()}


# ============================================================================
# Process Manager
# ============================================================================

class ProcessManager:
    """Manages a single llama.cpp process for one profile."""

    def __init__(self, profile, evidence_callback=None):
        self.profile = profile
        self.alias = profile["alias"]
        self.port = profile["port"]
        self.process = None
        self.pid = None
        self.state = "stopped"  # stopped, starting, healthy, degraded, failed
        self.start_time = None
        self.last_health_time = None
        self.health_fail_count = 0
        self.error_message = None
        self._lock = threading.Lock()
        self._evidence_callback = evidence_callback
        # Backend log management
        self._log_path = RUNTIME_NODE / "logs" / f"backend_{self.alias}.log"
        self._log_handle = None

    def _open_log(self):
        """Open or reopen the backend log file. Returns the file handle."""
        logs_dir = RUNTIME_NODE / "logs"
        logs_dir.mkdir(parents=True, exist_ok=True)
        handle = open(str(self._log_path), "w", encoding="utf-8")
        self._log_handle = handle
        return handle

    def _close_log(self):
        """Close the backend log handle if open."""
        if self._log_handle is not None:
            try:
                self._log_handle.close()
            except Exception:
                pass
            self._log_handle = None

    def start(self, timeout=BACKEND_START_TIMEOUT_SECONDS):
        """Start the llama-server process for this profile.
        
        Lock is held only for state transitions, NOT across the health
        wait loop, so the background HealthPoller can still acquire the
        lock via poll_health() during startup.
        """
        # Phase 1: Setup under lock
        with self._lock:
            if self.process and self.process.poll() is None:
                log.info("[%s] already running (PID %d)", self.alias, self.pid)
                return True

            self.state = "starting"
            self.error_message = None
            self.health_fail_count = 0

            binary = str(BINARY)
            if not os.path.exists(binary):
                self.state = "failed"
                self.error_message = f"Binary not found: {binary}"
                log.error("[%s] %s", self.alias, self.error_message)
                return False

            cmd = [
                binary,
                "-m", self.profile["model_path"],
                "-p", str(self.port),
                "-c", str(self.profile.get("context", 1024)),
                "-ngl", str(self.profile.get("ngl", 99)),
                "-n", "512",
                "--alias", self.alias,
            ]

            try:
                # Close any previous log handle before opening new one
                self._close_log()
                log_handle = self._open_log()
                self.process = subprocess.Popen(
                    cmd,
                    stdout=log_handle,
                    stderr=subprocess.STDOUT,
                    creationflags=subprocess.CREATE_NO_WINDOW,
                )
                self.pid = self.process.pid
                self.start_time = time.time()
                log.info("[%s] started (PID %d, port %d)", self.alias, self.pid, self.port)
            except FileNotFoundError as e:
                self._close_log()
                self.state = "failed"
                self.error_message = str(e)
                log.error("[%s] Failed to start: %s", self.alias, e)
                return False

        # Phase 2: Health wait loop (lock NOT held)
        deadline = time.time() + timeout
        while time.time() < deadline:
            with self._lock:
                # Re-check process status each iteration (under lock)
                if self.process.poll() is not None:
                    self._close_log()
                    self.state = "failed"
                    self.error_message = f"Process exited (code {self.process.returncode})"
                    log.error("[%s] %s", self.alias, self.error_message)
                    return False

            # Check health outside the lock so HealthPoller can run
            if self._check_health():
                with self._lock:
                    self.state = "healthy"
                    self.last_health_time = time.time()
                log.info("[%s] healthy after %.1fs", self.alias, time.time() - self.start_time)
                return True

            time.sleep(2)

        # Phase 3: Timed out — update state under lock
        with self._lock:
            self._close_log()
            self.state = "failed"
            self.error_message = f"Timed out after {timeout}s waiting for health"
            log.error("[%s] %s", self.alias, self.error_message)
        return False

    def stop(self):
        """Stop the llama-server process."""
        with self._lock:
            if self.process and self.process.poll() is None:
                log.info("[%s] stopping (PID %d)", self.alias, self.pid)
                self.process.terminate()
                try:
                    self.process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    log.warning("[%s] force kill (PID %d)", self.alias, self.pid)
                    self.process.kill()
                    self.process.wait()
                self.state = "stopped"
                self.pid = None
                log.info("[%s] stopped", self.alias)
            else:
                self.state = "stopped"
                self.pid = None
        # Close log handle outside the lock (no state dependency)
        self._close_log()

    def restart(self):
        """Restart the backend process."""
        log.info("[%s] restarting", self.alias)
        self.stop()
        return self.start()

    def _check_health(self):
        """Check if the backend health endpoint returns OK.
        NOTE: Caller must hold self._lock if state needs protection.
        """
        try:
            url = f"http://127.0.0.1:{self.port}/health"
            req = urllib.request.Request(url, method="GET")
            with urllib.request.urlopen(req, timeout=3) as resp:
                if resp.status == 200:
                    data = json.loads(resp.read().decode())
                    if data.get("status") == "ok":
                        return data
        except Exception:
            pass
        return None

    def poll_health(self):
        """Health check - updates state. Returns True if healthy."""
        # Skip if process is not running
        with self._lock:
            if self.state in ("stopped", "failed"):
                return False
            if self.process and self.process.poll() is not None:
                self.state = "failed"
                self.error_message = f"Process exited (code {self.process.returncode})"
                return False

        result = self._check_health()
        with self._lock:
            if result:
                reported_alias = result.get("model", "")
                if reported_alias and reported_alias != self.alias:
                    self.state = "degraded"
                    self.error_message = f"Identity mismatch: reports '{reported_alias}', expected '{self.alias}'"
                    self.health_fail_count += 1
                    log.warning("[%s] %s", self.alias, self.error_message)
                    return False
                self.health_fail_count = 0
                self.last_health_time = time.time()
                if self.state in ("starting", "degraded"):
                    self.state = "healthy"
                    self.error_message = None
                return True
            else:
                self.health_fail_count += 1
                if self.state == "healthy":
                    self.state = "degraded"
                    self.error_message = f"Health check failed ({self.health_fail_count})"
                elif self.state == "degraded" and self.health_fail_count >= 3:
                    self.state = "failed"
                    self.error_message = "Consecutive health check failures"
                return False

    def verify_identity(self):
        """Verify model identity against /health and /v1/models."""
        try:
            # Check /health
            health_url = f"http://127.0.0.1:{self.port}/health"
            req = urllib.request.Request(health_url, method="GET")
            with urllib.request.urlopen(req, timeout=BACKEND_HEALTH_TIMEOUT_SECONDS) as resp:
                health_data = json.loads(resp.read().decode())
                health_alias = health_data.get("model", "")
                if health_alias != self.alias:
                    return False, f"/health reports '{health_alias}', expected '{self.alias}'"

            # Check /v1/models
            models_url = f"http://127.0.0.1:{self.port}/v1/models"
            req = urllib.request.Request(models_url, method="GET")
            with urllib.request.urlopen(req, timeout=BACKEND_HEALTH_TIMEOUT_SECONDS) as resp:
                models_data = json.loads(resp.read().decode())
                models_id = models_data.get("data", [{}])[0].get("id", "")
                if models_id != self.alias:
                    return False, f"/v1/models reports '{models_id}', expected '{self.alias}'"

            return True, "identity verified"
        except Exception as e:
            return False, str(e)

    def get_status(self):
        """Return current status dict."""
        with self._lock:
            uptime = None
            if self.start_time and self.state == "healthy":
                uptime = int(time.time() - self.start_time)
            return {
                "alias": self.alias,
                "state": self.state,
                "pid": self.pid,
                "port": self.port,
                "uptime_seconds": uptime,
                "health_fail_count": self.health_fail_count,
                "error": self.error_message,
            }

    def proxy_chat(self, messages, max_tokens=256, temperature=0.7):
        """Proxy a chat request to the backend."""
        body = {
            "messages": messages,
            "max_tokens": max_tokens,
            "temperature": temperature,
            "stream": False,
        }
        url = f"http://127.0.0.1:{self.port}/v1/chat/completions"
        req = urllib.request.Request(
            url,
            data=json.dumps(body).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=BACKEND_REQUEST_TIMEOUT_SECONDS) as resp:
                response_data = json.loads(resp.read().decode())
                return response_data
        except urllib.error.HTTPError as e:
            error_body = e.read().decode() if e.fp else ""
            return {"error": f"Backend returned {e.code}: {error_body}"}
        except Exception as e:
            return {"error": f"Backend request failed: {e}"}


# ============================================================================
# Refusal Engine
# ============================================================================

class RefusalEngine:
    """Checks requests against contract refusal conditions."""

    def __init__(self, profile_manager, process_managers):
        self.profile_manager = profile_manager
        self.process_managers = process_managers  # alias -> ProcessManager

    def check_select(self, alias, task_class=None):
        """Check if a profile can be selected. Returns (ok, reason_dict)."""
        profile = self.profile_manager.get(alias)
        if not profile:
            return False, {
                "status": "refused",
                "reason": "unknown_profile",
                "detail": REFUSAL_REASONS["unknown_profile"].format(alias=alias),
                "authority": "advisory_only",
                "timestamp": _now_iso(),
            }

        if profile.get("verified_status") != "verified":
            return False, {
                "status": "refused",
                "reason": "unverified_profile",
                "detail": REFUSAL_REASONS["unverified_profile"].format(
                    alias=alias, status=profile.get("verified_status", "unknown")
                ),
                "authority": "advisory_only",
                "timestamp": _now_iso(),
            }

        if task_class:
            task_classes = profile.get("task_classes", [])
            if task_class not in task_classes:
                return False, {
                    "status": "refused",
                    "reason": "unknown_profile",
                    "detail": f"Task class '{task_class}' not declared for profile '{alias}'. Declared: {task_classes}",
                    "authority": "advisory_only",
                    "timestamp": _now_iso(),
                }

        pm = self.process_managers.get(alias)
        if pm and pm.state == "failed":
            return False, {
                "status": "refused",
                "reason": "runtime_unhealthy",
                "detail": REFUSAL_REASONS["runtime_unhealthy"].format(alias=alias, port=profile.get("port", "?")),
                "authority": "advisory_only",
                "timestamp": _now_iso(),
            }

        return True, None

    def check_chat(self, alias, messages, context=None):
        """Check if a chat request can be routed. Returns (ok, reason_dict)."""
        profile = self.profile_manager.get(alias)
        if not profile:
            return False, {
                "status": "refused",
                "reason": "unknown_profile",
                "detail": REFUSAL_REASONS["unknown_profile"].format(alias=alias),
                "authority": "advisory_only",
                "timestamp": _now_iso(),
            }

        # Context check
        verified_context = profile.get("context", 1024)
        if context and context > verified_context:
            return False, {
                "status": "refused",
                "reason": "context_exceeds_verified",
                "detail": REFUSAL_REASONS["context_exceeds_verified"].format(
                    requested=context, verified=verified_context, alias=alias
                ),
                "authority": "advisory_only",
                "timestamp": _now_iso(),
            }

        # Check runtime health
        pm = self.process_managers.get(alias)
        if not pm or pm.state in ("stopped", "failed", "starting"):
            port = profile.get("port", "?")
            return False, {
                "status": "refused",
                "reason": "runtime_unhealthy",
                "detail": REFUSAL_REASONS["runtime_unhealthy"].format(alias=alias, port=port),
                "authority": "advisory_only",
                "timestamp": _now_iso(),
            }

        # Check identity
        if pm.state == "healthy":
            ok, detail = pm.verify_identity()
            if not ok:
                return False, {
                    "status": "refused",
                    "reason": "identity_mismatch",
                    "detail": REFUSAL_REASONS["identity_mismatch"].format(
                        alias=alias, port=profile.get("port", "?"), reported="unknown"
                    ) + f": {detail}",
                    "authority": "advisory_only",
                    "timestamp": _now_iso(),
                }

        # Check for authority-bearing content
        user_text = " ".join(
            m.get("content", "") for m in (messages or [])
            if m.get("role") == "user"
        ).lower()

        for keyword in AUTHORITY_KEYWORDS:
            if keyword in user_text:
                return False, {
                    "status": "refused",
                    "reason": "authority_required",
                    "detail": f"Request contains authority-bearing content ('{keyword}'). {REFUSAL_REASONS['authority_required']}",
                    "authority": "advisory_only",
                    "timestamp": _now_iso(),
                }

        # Check for autonomous action
        if any(w in user_text for w in ["autonomous", "self-directed", "automatic decision"]):
            return False, {
                "status": "refused",
                "reason": "autonomous_action_forbidden",
                "detail": REFUSAL_REASONS["autonomous_action_forbidden"],
                "authority": "advisory_only",
                "timestamp": _now_iso(),
            }

        # Check for file mutation
        if any(w in user_text for w in ["edit source", "modify file", "write to librarian", "promote this file"]):
            return False, {
                "status": "refused",
                "reason": "file_mutation_forbidden",
                "detail": REFUSAL_REASONS["file_mutation_forbidden"],
                "authority": "advisory_only",
                "timestamp": _now_iso(),
            }

        return True, None


# ============================================================================
# Evidence Writer
# ============================================================================

class EvidenceWriter:
    """Writes evidence to fixtures/windows-runtime-node/router-impl/."""

    def __init__(self, directory=EVIDENCE_DIR):
        self.directory = Path(directory)
        self.directory.mkdir(parents=True, exist_ok=True)

    def write(self, filename, data):
        path = self.directory / filename
        path.write_text(
            json.dumps(data, indent=2, default=str),
            encoding="utf-8",
        )
        log.info("Evidence written: %s", path)
        return str(path)


def _now_iso():
    return datetime.now(timezone.utc).isoformat()


# ============================================================================
# HTTP Request Handler
# ============================================================================

class RouterHandler(BaseHTTPRequestHandler):
    """HTTP handler for the 6 contract endpoints."""

    # Shared state set from main
    profile_manager = None
    process_managers = {}
    refusal_engine = None
    evidence_writer = None
    start_time = time.time()

    def log_message(self, format, *args):
        log.info("HTTP %s %s", self.command, self.path)

    def _send_json(self, data, status=200):
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(json.dumps(data, indent=2, default=str).encode("utf-8"))

    def _send_error_json(self, status, detail):
        self._send_json({"error": detail}, status)

    def _read_body(self):
        """Read and parse request body. Raises RequestTooLarge if oversized."""
        raw_length = self.headers.get("Content-Length", "0")
        try:
            length = int(raw_length)
        except (ValueError, TypeError):
            length = 0

        if length <= 0:
            return {}

        if length > MAX_BODY_BYTES:
            raise RequestTooLarge(
                f"Request body length {length} exceeds maximum {MAX_BODY_BYTES} bytes"
            )

        # Read in chunks to avoid loading oversized data into memory
        chunks = []
        remaining = min(length, MAX_BODY_BYTES)
        while remaining > 0:
            chunk_size = min(remaining, 16384)
            chunk = self.rfile.read(chunk_size)
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)

        body_bytes = b"".join(chunks)
        if len(body_bytes) > MAX_BODY_BYTES:
            raise RequestTooLarge(
                f"Request body {len(body_bytes)} exceeds maximum {MAX_BODY_BYTES} bytes"
            )

        return json.loads(body_bytes.decode("utf-8"))

    # ---- Routes ----

    def _route_get(self):
        path = self.path.rstrip("/")
        if path == "/backend/status" or path == "/backend/status/":
            self._handle_status()
        elif path == "/backend/profiles" or path == "/backend/profiles/":
            self._handle_profiles()
        elif path == "/backend/health" or path == "/backend/health/":
            self._handle_health()
        elif path == "/health" or path == "/health/":
            self._handle_health_legacy()
        else:
            self._send_error_json(404, f"Not found: {self.path}")

    def _route_post(self):
        path = self.path.rstrip("/")
        try:
            body = self._read_body()
        except RequestTooLarge as e:
            self._send_error_json(413, str(e))
            return
        except (json.JSONDecodeError, UnicodeDecodeError) as e:
            self._send_error_json(400, f"Invalid JSON body: {e}")
            return

        if path == "/backend/select" or path == "/backend/select/":
            self._handle_select(body)
        elif path == "/backend/restart" or path == "/backend/restart/":
            self._handle_restart(body)
        elif path == "/backend/chat" or path == "/backend/chat/":
            self._handle_chat(body)
        else:
            self._send_error_json(404, f"Not found: {self.path}")

    def do_GET(self):
        self._route_get()

    def do_POST(self):
        self._route_post()

    def do_OPTIONS(self):
        self.send_response(204)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.end_headers()

    # ---- Endpoint Handlers ----

    def _handle_status(self):
        """GET /backend/status - Router and runtime status."""
        profiles_status = {}
        all_alive = True
        for alias, pm in self.process_managers.items():
            s = pm.get_status()
            profiles_status[alias] = s
            if s["state"] not in ("healthy", "stopped"):
                all_alive = False

        active_profile = None
        for alias, pm in self.process_managers.items():
            if pm.state == "healthy":
                active_profile = alias
                break

        overall = "ok" if all_alive else "degraded"
        if not any(pm.state == "healthy" for pm in self.process_managers.values()):
            overall = "degraded"

        response = {
            "status": overall,
            "active_profile": active_profile,
            "profiles_registered": len(self.profile_manager.list_all()),
            "runtimes_alive": sum(1 for pm in self.process_managers.values() if pm.state == "healthy"),
            "uptime_seconds": int(time.time() - self.start_time),
            "authority": "advisory_only",
            "profiles": profiles_status,
        }
        self._send_json(response)
        self.evidence_writer.write("status.json", response)

    def _handle_profiles(self):
        """GET /backend/profiles - List registered profiles."""
        response = {
            "profiles": self.profile_manager.list_all(),
            "authority": "advisory_only",
        }
        self._send_json(response)
        self.evidence_writer.write("profiles.json", response)

    def _handle_health(self):
        """GET /backend/health - Health check for all profiles."""
        profiles_health = {}
        all_healthy = True
        for alias, pm in self.process_managers.items():
            h = pm.poll_health()
            profiles_health[alias] = {
                "status": "ok" if h else "degraded",
                "state": pm.state,
                "identity_verified": pm.state == "healthy",
                "port": pm.port,
            }
            if not h:
                all_healthy = False

        response = {
            "status": "ok" if all_healthy else "degraded",
            "active_profile": next(
                (alias for alias, pm in self.process_managers.items() if pm.state == "healthy"),
                None
            ),
            "profiles": profiles_health,
            "authority": "advisory_only",
        }
        self._send_json(response)
        self.evidence_writer.write("health.json", response)

    def _handle_health_legacy(self):
        """GET /health - Legacy health endpoint for compatibility."""
        active_alias = next(
            (alias for alias, pm in self.process_managers.items() if pm.state == "healthy"),
            None
        )
        response = {
            "status": "ok" if active_alias else "degraded",
            "router": "ok",
            "active_profile": active_alias,
            "authority": "advisory_only",
        }
        self._send_json(response)

    def _handle_select(self, body):
        """POST /backend/select - Select a model profile."""
        alias = body.get("profile", "")
        task_class = body.get("task_class")
        context = body.get("context")

        if not alias:
            self._send_error_json(400, "Missing 'profile' field")
            return

        # Check refusal
        ok, refusal = self.refusal_engine.check_select(alias, task_class)
        if not ok:
            self._send_json(refusal, 403)
            self.evidence_writer.write("select-invalid.json", refusal)
            return

        profile = self.profile_manager.get(alias)
        pm = self.process_managers.get(alias)

        # Start the profile if not running
        if pm and pm.state in ("stopped", "failed"):
            log.info("[%s] starting profile on demand", alias)
            pm.start()

        # Wait briefly for health
        if pm:
            for _ in range(10):
                if pm.poll_health():
                    break
                time.sleep(1)

        response = {
            "status": "selected",
            "profile": alias,
            "port": profile["port"],
            "authority": "advisory_only",
            "task_class": task_class,
        }
        self._send_json(response)
        self.evidence_writer.write("select-valid.json", response)

    def _handle_restart(self, body):
        """POST /backend/restart - Restart a profile's backend."""
        alias = body.get("profile", "")
        if not alias:
            self._send_error_json(400, "Missing 'profile' field")
            return

        profile = self.profile_manager.get(alias)
        if not profile:
            self._send_json({
                "status": "refused",
                "reason": "unknown_profile",
                "detail": REFUSAL_REASONS["unknown_profile"].format(alias=alias),
                "authority": "advisory_only",
                "timestamp": _now_iso(),
            }, 403)
            return

        pm = self.process_managers.get(alias)
        if not pm:
            self._send_error_json(500, f"No process manager for '{alias}'")
            return

        old_pid = pm.pid
        success = pm.restart()
        new_pid = pm.pid

        response = {
            "status": "restarting" if success else "failed",
            "profile": alias,
            "old_pid": old_pid,
            "new_pid": new_pid,
            "estimated_wait_seconds": 10,
            "authority": "advisory_only",
        }
        status_code = 200 if success else 500
        self._send_json(response, status_code)
        self.evidence_writer.write("restart-result.json", response)

        # Write process before/after
        self.evidence_writer.write("process-before-after.txt",
            f"Before: PID={old_pid}\nAfter: PID={new_pid}\nProfile: {alias}\nTimestamp: {_now_iso()}\n")

    def _handle_chat(self, body):
        """POST /backend/chat - Send advisory chat to selected profile."""
        alias = body.get("profile", "")
        messages = body.get("messages", [])
        max_tokens = body.get("max_tokens", 256)
        temperature = body.get("temperature", 0.7)
        context = body.get("context")

        if not alias:
            self._send_error_json(400, "Missing 'profile' field")
            return

        if not messages:
            self._send_error_json(400, "Missing 'messages' field")
            return

        # Check refusal
        ok, refusal = self.refusal_engine.check_chat(alias, messages, context)
        if not ok:
            self._send_json(refusal, 403)
            self.evidence_writer.write("chat-refusal-authority.json", refusal)
            return

        pm = self.process_managers.get(alias)
        if not pm or pm.state != "healthy":
            self._send_json({
                "status": "refused",
                "reason": "runtime_unhealthy",
                "detail": REFUSAL_REASONS["runtime_unhealthy"].format(
                    alias=alias, port=pm.port if pm else "?"
                ),
                "authority": "advisory_only",
                "timestamp": _now_iso(),
            }, 503)
            return

        # Proxy to backend
        result = pm.proxy_chat(messages, max_tokens, temperature)

        if "error" in result:
            self._send_json({
                "status": "error",
                "error": result["error"],
                "profile": alias,
                "authority": "advisory_only",
            }, 502)
            return

        # Extract content
        choices = result.get("choices", [])
        content = ""
        finish_reason = "stop"
        if choices:
            content = choices[0].get("message", {}).get("content", "")
            finish_reason = choices[0].get("finish_reason", "stop")

        response = {
            "status": "ok",
            "content": content,
            "finish_reason": finish_reason,
            "profile": alias,
            "authority": "advisory_only",
        }
        self._send_json(response)
        self.evidence_writer.write("chat-valid.json", response)


# ============================================================================
# Health Poller (background thread)
# ============================================================================

class HealthPoller(threading.Thread):
    """Background thread that periodically polls backend health."""

    def __init__(self, process_managers, interval=HEALTH_POLL_SECONDS):
        super().__init__(daemon=True)
        self.process_managers = process_managers
        self.interval = interval
        self.running = True

    def run(self):
        while self.running:
            for alias, pm in self.process_managers.items():
                if pm.state not in ("stopped", "failed"):
                    pm.poll_health()
            time.sleep(self.interval)

    def stop(self):
        self.running = False


# ============================================================================
# Main
# ============================================================================

def ensure_runtime_config():
    """Copy model-profiles.json to runtime config directory if missing."""
    config_dir = RUNTIME_NODE / "config"
    config_dir.mkdir(parents=True, exist_ok=True)
    dest = config_dir / "model-profiles.json"
    if not dest.exists():
        for src in PROFILE_SOURCES:
            p = Path(src)
            if p.exists():
                import shutil
                shutil.copy2(str(p), str(dest))
                log.info("Copied profiles to %s", dest)
                return
        log.warning("No profile source to copy")


def main():
    parser = argparse.ArgumentParser(description="llama-router — Windows Runtime Router")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT, help="Router HTTP port")
    parser.add_argument("--profiles", type=str, help="Path to model-profiles.json")
    args = parser.parse_args()

    # Ensure runtime config
    ensure_runtime_config()

    # Load profiles
    if args.profiles:
        global PROFILE_SOURCES
        PROFILE_SOURCES = [Path(args.profiles)] + list(PROFILE_SOURCES)

    profile_manager = ProfileManager()
    if not profile_manager.profiles:
        log.error("No profiles loaded. Cannot start router.")
        sys.exit(1)

    log.info("Loaded %d profiles: %s", len(profile_manager.profiles),
             ", ".join(profile_manager.profiles.keys()))

    # Create process managers
    process_managers = {}
    for alias, profile in profile_manager.profiles.items():
        pm = ProcessManager(profile)
        process_managers[alias] = pm
        log.info("Registered profile '%s' on port %d", alias, profile.get("port", 0))

    # Create refusal engine
    refusal_engine = RefusalEngine(profile_manager, process_managers)

    # Create evidence writer
    evidence_writer = EvidenceWriter()
    log.info("Evidence directory: %s", evidence_writer.directory)

    # Set up handler
    RouterHandler.profile_manager = profile_manager
    RouterHandler.process_managers = process_managers
    RouterHandler.refusal_engine = refusal_engine
    RouterHandler.evidence_writer = evidence_writer
    RouterHandler.start_time = time.time()

    # Start health poller
    poller = HealthPoller(process_managers)
    poller.start()

    # Start HTTP server (threaded for concurrent requests)
    server = ThreadingHTTPServer(("0.0.0.0", args.port), RouterHandler)
    log.info("=" * 60)
    log.info("llama-router v1 (WIN-ROUTER-IMPL-1)")
    log.info("Listening on http://0.0.0.0:%d", args.port)
    log.info("Profiles: %s", ", ".join(profile_manager.profiles.keys()))
    log.info("Authority: advisory_only")
    log.info("=" * 60)

    # Write startup evidence
    evidence_writer.write("router-startup.json", {
        "status": "started",
        "port": args.port,
        "profiles_loaded": len(profile_manager.profiles),
        "profiles": list(profile_manager.profiles.keys()),
        "authority": "advisory_only",
        "timestamp": _now_iso(),
    })

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        log.info("Shutting down...")
    finally:
        log.info("Stopping processes...")
        for alias, pm in process_managers.items():
            pm.stop()
        poller.stop()
        server.server_close()
        log.info("Shutdown complete.")


if __name__ == "__main__":
    main()
