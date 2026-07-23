#!/usr/bin/env python3
"""
MAC/WIN-ROUTER-CONTEXT-MEASURE-1 — Context Movement Cost Measurement Harness

Measures real costs of context movement, evidence reads, recall-packet handling,
JSON processing, and remote runtime checks on the actual Librarian hardware stack.

DO NOT modify production router behavior.
DO NOT implement live context routing.
DO NOT modify model execution.
DO NOT add a cache engine.
DO NOT add GPU/RDMA/KV-cache behavior or claims.

Sprint: MAC/WIN-ROUTER-CONTEXT-MEASURE-1
Status: Measurement / calibration only.
"""

import json
import os
import sys
import time
import statistics
import hashlib
import subprocess
import tempfile
import zlib
import string
import random
from pathlib import Path
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional, Tuple

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
FIXTURES_DIR = REPO_ROOT / "fixtures" / "context-route"
CONFIG_DIR = REPO_ROOT / "config"
REPORTS_DIR = REPO_ROOT / "reports"
TEMP_DIR = REPO_ROOT / "temp"

# Token approximation: 1 token ~= 4 bytes (English text avg)
BYTES_PER_TOKEN = 4

# Payload sizes in tokens
PAYLOAD_SIZES = {
    "small_append": 429,
    "medium_context": 8000,
    "large_reused_context": 32700,
    "very_large_context": 64000,  # Reduced from 128K for measurement feasibility
}

# Iteration counts (tuned for Windows measurement without timeout)
ITERATIONS = {
    "small_ops": 50,
    "medium_ops": 20,
    "large_ops": 10,
    "network_ops": 20,
    "timeout_ops": 3,
}

# Router ports to test (from model-profiles.json)
ROUTER_PORTS = [8080, 9120, 9121, 9122, 9123, 9124]


# ---------------------------------------------------------------------------
# Utility functions
# ---------------------------------------------------------------------------

def generate_payload(token_count: int) -> str:
    """Generate a synthetic text payload of approximately token_count tokens.
    Optimized for speed: uses pre-built repeating pattern."""
    # Use ~4 bytes per token approximation
    byte_count = token_count * BYTES_PER_TOKEN
    # Use a pre-built realistic pattern repeated as needed
    base = ("the quick brown fox jumps over lazy dog context route recall packet "
            "evidence receipt sprint workload optimizer cache memory storage "
            "runtime node health status profile model gateway protocol network "
            "latency throughput governance provenance freshness verified current ")
    repetitions = (byte_count // len(base)) + 1
    return (base * repetitions)[:byte_count]


def generate_json_payload(token_count: int) -> dict:
    """Generate a JSON-like payload with approximately token_count tokens of content."""
    text = generate_payload(token_count)
    return {
        "context_data": text,
        "token_count": token_count,
        "metadata": {
            "source": "measurement_harness",
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "purpose": "context_movement_cost_measurement",
        },
    }


def time_operation(func, *args, iterations: int = 1, **kwargs) -> Dict[str, Any]:
    """Time a function call over multiple iterations and return statistics."""
    times = []
    errors = 0
    last_error = None

    for _ in range(iterations):
        try:
            start = time.perf_counter_ns()
            func(*args, **kwargs)
            end = time.perf_counter_ns()
            times.append((end - start) / 1_000_000)  # Convert to ms
        except Exception as e:
            errors += 1
            last_error = str(e)

    if not times:
        return {
            "min_latency_ms": None,
            "max_latency_ms": None,
            "mean_latency_ms": None,
            "median_latency_ms": None,
            "p95_latency_ms": None,
            "std_dev_ms": None,
            "iteration_count": iterations,
            "success_count": 0,
            "failure_count": errors,
            "last_error": last_error,
        }

    times_sorted = sorted(times)
    p95_idx = int(len(times_sorted) * 0.95)

    return {
        "min_latency_ms": round(times_sorted[0], 4),
        "max_latency_ms": round(times_sorted[-1], 4),
        "mean_latency_ms": round(statistics.mean(times), 4),
        "median_latency_ms": round(statistics.median(times), 4),
        "p95_latency_ms": round(times_sorted[min(p95_idx, len(times_sorted) - 1)], 4),
        "std_dev_ms": round(statistics.stdev(times), 4) if len(times) > 1 else 0.0,
        "iteration_count": iterations,
        "success_count": len(times),
        "failure_count": errors,
    }


# ---------------------------------------------------------------------------
# Measurement dimensions
# ---------------------------------------------------------------------------

def measure_file_io() -> List[Dict[str, Any]]:
    """Dimension 1: Local file read/write for context-sized payloads."""
    results = []
    test_dir = TEMP_DIR / "measure-file-io"
    test_dir.mkdir(parents=True, exist_ok=True)

    for label, token_count in PAYLOAD_SIZES.items():
        payload = generate_payload(token_count)
        payload_bytes = len(payload.encode("utf-8"))
        filepath = test_dir / f"payload-{label}.txt"

        # Cold write
        write_stats = time_operation(
            lambda p=payload, f=filepath: f.write_text(p, encoding="utf-8"),
            iterations=ITERATIONS["small_ops"] if token_count < 1000 else ITERATIONS["medium_ops"],
        )
        write_stats["operation_type"] = "file_write_cold"
        write_stats["payload_size_bytes"] = payload_bytes
        write_stats["approx_tokens"] = token_count
        write_stats["cold_warm"] = "cold"
        write_stats["label"] = label
        results.append(write_stats)

        # Cold read (file exists, first read after write)
        read_stats = time_operation(
            lambda f=filepath: f.read_text(encoding="utf-8"),
            iterations=ITERATIONS["small_ops"] if token_count < 1000 else ITERATIONS["medium_ops"],
        )
        read_stats["operation_type"] = "file_read_cold"
        read_stats["payload_size_bytes"] = payload_bytes
        read_stats["approx_tokens"] = token_count
        read_stats["cold_warm"] = "cold"
        read_stats["label"] = label
        results.append(read_stats)

        # Warm read (repeated reads, OS cache should help)
        read_warm_stats = time_operation(
            lambda f=filepath: f.read_text(encoding="utf-8"),
            iterations=ITERATIONS["small_ops"] if token_count < 1000 else ITERATIONS["medium_ops"],
        )
        read_warm_stats["operation_type"] = "file_read_warm"
        read_warm_stats["payload_size_bytes"] = payload_bytes
        read_warm_stats["approx_tokens"] = token_count
        read_warm_stats["cold_warm"] = "warm"
        read_warm_stats["label"] = label
        results.append(read_warm_stats)

        # Binary read (bytes mode)
        bin_read_stats = time_operation(
            lambda f=filepath: f.read_bytes(),
            iterations=ITERATIONS["small_ops"] if token_count < 1000 else ITERATIONS["medium_ops"],
        )
        bin_read_stats["operation_type"] = "file_read_binary"
        bin_read_stats["payload_size_bytes"] = payload_bytes
        bin_read_stats["approx_tokens"] = token_count
        bin_read_stats["cold_warm"] = "warm"
        bin_read_stats["label"] = label
        results.append(bin_read_stats)

        # Cleanup
        filepath.unlink(missing_ok=True)

    return results


def measure_json_processing() -> List[Dict[str, Any]]:
    """Dimension 2: JSON load / parse / serialize."""
    results = []

    # Measure with existing fixture files
    fixture_files = list(FIXTURES_DIR.glob("*.json"))
    for fixture_file in fixture_files:
        raw = fixture_file.read_bytes()
        raw_len = len(raw)

        # Read raw bytes
        read_stats = time_operation(
            lambda f=fixture_file: f.read_bytes(),
            iterations=ITERATIONS["small_ops"],
        )
        read_stats["operation_type"] = "json_file_read"
        read_stats["payload_size_bytes"] = raw_len
        read_stats["label"] = f"fixture:{fixture_file.stem}"
        results.append(read_stats)

        # Parse (loads)
        parse_stats = time_operation(
            lambda d=raw: json.loads(d),
            iterations=ITERATIONS["small_ops"],
        )
        parse_stats["operation_type"] = "json_parse"
        parse_stats["payload_size_bytes"] = raw_len
        parse_stats["label"] = f"fixture:{fixture_file.stem}"
        results.append(parse_stats)

        # Serialize (dumps)
        obj = json.loads(raw)
        serialize_stats = time_operation(
            lambda o=obj: json.dumps(o, separators=(",", ":")),
            iterations=ITERATIONS["small_ops"],
        )
        serialize_stats["operation_type"] = "json_serialize"
        serialize_stats["payload_size_bytes"] = raw_len
        serialize_stats["label"] = f"fixture:{fixture_file.stem}"
        results.append(serialize_stats)

        # Write
        out_path = TEMP_DIR / f"json-write-test-{fixture_file.stem}.json"
        write_stats = time_operation(
            lambda o=obj, p=out_path: p.write_text(json.dumps(o), encoding="utf-8"),
            iterations=ITERATIONS["small_ops"],
        )
        write_stats["operation_type"] = "json_write"
        write_stats["payload_size_bytes"] = raw_len
        write_stats["label"] = f"fixture:{fixture_file.stem}"
        results.append(write_stats)
        out_path.unlink(missing_ok=True)

        # Warm parse (repeated)
        warm_parse_stats = time_operation(
            lambda d=raw: json.loads(d),
            iterations=ITERATIONS["small_ops"],
        )
        warm_parse_stats["operation_type"] = "json_parse_warm"
        warm_parse_stats["payload_size_bytes"] = raw_len
        warm_parse_stats["label"] = f"fixture:{fixture_file.stem}"
        results.append(warm_parse_stats)

    # Synthetic payloads at different sizes
    for label, token_count in PAYLOAD_SIZES.items():
        synth = generate_json_payload(token_count)
        synth_bytes = len(json.dumps(synth).encode("utf-8"))

        parse_stats = time_operation(
            lambda s=json.dumps(synth): json.loads(s),
            iterations=ITERATIONS["small_ops"] if token_count < 1000 else ITERATIONS["medium_ops"],
        )
        parse_stats["operation_type"] = "json_parse_synthetic"
        parse_stats["payload_size_bytes"] = synth_bytes
        parse_stats["approx_tokens"] = token_count
        parse_stats["label"] = f"synthetic:{label}"
        results.append(parse_stats)

        serialize_stats = time_operation(
            lambda o=synth: json.dumps(o, separators=(",", ":")),
            iterations=ITERATIONS["small_ops"] if token_count < 1000 else ITERATIONS["medium_ops"],
        )
        serialize_stats["operation_type"] = "json_serialize_synthetic"
        serialize_stats["payload_size_bytes"] = synth_bytes
        serialize_stats["approx_tokens"] = token_count
        serialize_stats["label"] = f"synthetic:{label}"
        results.append(serialize_stats)

    return results


def measure_recall_packet() -> List[Dict[str, Any]]:
    """Dimension 3: Recall packet serialize / deserialize."""
    results = []

    # Build recall packets of varying sizes
    recall_sizes = {
        "compact_recall": 5000,
        "medium_recall": 32700,
        "large_recall": 64000,  # Reduced from 128K for measurement feasibility
    }

    for label, token_count in recall_sizes.items():
        # Build a realistic recall packet structure
        recall_packet = {
            "recall_id": f"recall-{label}-{int(time.time())}",
            "contract_version": "0.1",
            "workload_type": "agent_handoff",
            "context_summary": generate_payload(token_count // 2),
            "decisions_log": [
                {"turn": i, "action": f"decision_{i}", "evidence": generate_payload(50)}
                for i in range(min(token_count // 200, 50))
            ],
            "state_snapshot": {
                "active_files": ["file1.py", "file2.py", "file3.py"],
                "pending_actions": ["action1", "action2"],
                "governance_state": "verified_current",
            },
            "metadata": {
                "created_at": datetime.now(timezone.utc).isoformat(),
                "token_estimate": token_count,
                "compression_target": 0.85,
            },
        }

        packet_json = json.dumps(recall_packet)
        packet_bytes = len(packet_json.encode("utf-8"))

        # Serialize
        serialize_stats = time_operation(
            lambda p=recall_packet: json.dumps(p),
            iterations=ITERATIONS["medium_ops"],
        )
        serialize_stats["operation_type"] = "recall_serialize"
        serialize_stats["payload_size_bytes"] = packet_bytes
        serialize_stats["approx_tokens"] = token_count
        serialize_stats["label"] = label
        serialize_stats["cold_warm"] = "warm"
        results.append(serialize_stats)

        # Deserialize
        deserialize_stats = time_operation(
            lambda j=packet_json: json.loads(j),
            iterations=ITERATIONS["medium_ops"],
        )
        deserialize_stats["operation_type"] = "recall_deserialize"
        deserialize_stats["payload_size_bytes"] = packet_bytes
        deserialize_stats["approx_tokens"] = token_count
        deserialize_stats["label"] = label
        deserialize_stats["cold_warm"] = "warm"
        results.append(deserialize_stats)

        # Compress (zlib)
        compress_stats = time_operation(
            lambda j=packet_json: zlib.compress(j.encode("utf-8")),
            iterations=ITERATIONS["medium_ops"],
        )
        compressed = zlib.compress(packet_json.encode("utf-8"))
        compress_stats["operation_type"] = "recall_compress"
        compress_stats["payload_size_bytes"] = packet_bytes
        compress_stats["compressed_size_bytes"] = len(compressed)
        compress_stats["approx_tokens"] = token_count
        compress_stats["label"] = label
        compress_stats["cold_warm"] = "warm"
        results.append(compress_stats)

        # Decompress
        decompress_stats = time_operation(
            lambda c=compressed: zlib.decompress(c),
            iterations=ITERATIONS["medium_ops"],
        )
        decompress_stats["operation_type"] = "recall_decompress"
        decompress_stats["payload_size_bytes"] = packet_bytes
        decompress_stats["compressed_size_bytes"] = len(compressed)
        decompress_stats["approx_tokens"] = token_count
        decompress_stats["label"] = label
        decompress_stats["cold_warm"] = "warm"
        results.append(decompress_stats)

    return results


def measure_canonical_evidence() -> List[Dict[str, Any]]:
    """Dimension 4: Canonical evidence read operations."""
    results = []

    # git status --short
    git_status_stats = time_operation(
        lambda: subprocess.run(
            ["git", "status", "--short"],
            cwd=str(REPO_ROOT),
            capture_output=True,
            text=True,
            timeout=10,
        ),
        iterations=ITERATIONS["network_ops"],
    )
    git_status_stats["operation_type"] = "canonical_evidence_read"
    git_status_stats["label"] = "git_status_short"
    git_status_stats["method"] = "subprocess:git status --short"
    git_status_stats["cold_warm"] = "warm"
    results.append(git_status_stats)

    # git rev-parse --short HEAD
    git_revparse_stats = time_operation(
        lambda: subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=str(REPO_ROOT),
            capture_output=True,
            text=True,
            timeout=10,
        ),
        iterations=ITERATIONS["network_ops"],
    )
    git_revparse_stats["operation_type"] = "canonical_evidence_read"
    git_revparse_stats["label"] = "git_revparse_short_head"
    git_revparse_stats["method"] = "subprocess:git rev-parse --short HEAD"
    git_revparse_stats["cold_warm"] = "warm"
    results.append(git_revparse_stats)

    # Small file read (model-profiles.json)
    small_file = CONFIG_DIR / "model-profiles.json"
    if small_file.exists():
        small_file_stats = time_operation(
            lambda f=small_file: f.read_text(encoding="utf-8"),
            iterations=ITERATIONS["small_ops"],
        )
        small_file_stats["operation_type"] = "canonical_evidence_read"
        small_file_stats["label"] = "small_file_read"
        small_file_stats["payload_size_bytes"] = small_file.stat().st_size
        small_file_stats["method"] = "pathlib:read_text"
        small_file_stats["cold_warm"] = "warm"
        results.append(small_file_stats)

    # Sprint doc read
    sprint_doc = REPO_ROOT / "docs" / "sprints" / "WIN-RUNTIME-QUALIFICATION-1.md"
    if sprint_doc.exists():
        sprint_doc_stats = time_operation(
            lambda f=sprint_doc: f.read_text(encoding="utf-8"),
            iterations=ITERATIONS["medium_ops"],
        )
        sprint_doc_stats["operation_type"] = "canonical_evidence_read"
        sprint_doc_stats["label"] = "sprint_doc_read"
        sprint_doc_stats["payload_size_bytes"] = sprint_doc.stat().st_size
        sprint_doc_stats["method"] = "pathlib:read_text"
        sprint_doc_stats["cold_warm"] = "warm"
        results.append(sprint_doc_stats)

    # Contract fixture read
    contract_fixture = FIXTURES_DIR / "sprint-planning.json"
    if contract_fixture.exists():
        contract_stats = time_operation(
            lambda f=contract_fixture: f.read_text(encoding="utf-8"),
            iterations=ITERATIONS["small_ops"],
        )
        contract_stats["operation_type"] = "canonical_evidence_read"
        contract_stats["label"] = "contract_fixture_read"
        contract_stats["payload_size_bytes"] = contract_fixture.stat().st_size
        contract_stats["method"] = "pathlib:read_text"
        contract_stats["cold_warm"] = "warm"
        results.append(contract_stats)

    # Test result file read (if exists)
    test_result = REPO_ROOT / "reports" / "router-workload-optimizer-results.json"
    if test_result.exists():
        test_result_stats = time_operation(
            lambda f=test_result: f.read_text(encoding="utf-8"),
            iterations=ITERATIONS["medium_ops"],
        )
        test_result_stats["operation_type"] = "canonical_evidence_read"
        test_result_stats["label"] = "test_result_file_read"
        test_result_stats["payload_size_bytes"] = test_result.stat().st_size
        test_result_stats["method"] = "pathlib:read_text"
        test_result_stats["cold_warm"] = "warm"
        results.append(test_result_stats)

    return results


def measure_runtime_node_health() -> List[Dict[str, Any]]:
    """Dimension 5: Runtime-node health latency."""
    results = []

    # Test each port with a quick HTTP connection attempt
    import urllib.request
    import urllib.error

    endpoints = {
        "router_health_8080": "http://localhost:8080/health",
        "router_health_9120": "http://localhost:9120/health",
        "backend_status_8080": "http://localhost:8080/backend/status",
        "backend_profiles_8080": "http://localhost:8080/backend/profiles",
        "backend_health_8080": "http://localhost:8080/backend/health",
    }

    for label, url in endpoints.items():
        # Try connecting - expect timeout/failure since nodes are stopped
        def try_connect(u=url):
            try:
                req = urllib.request.Request(u, method="GET")
                with urllib.request.urlopen(req, timeout=2) as resp:
                    return resp.read()
            except urllib.error.URLError:
                raise ConnectionError(f"Connection refused: {u}")
            except Exception as e:
                raise ConnectionError(str(e))

        stats = time_operation(
            try_connect,
            iterations=ITERATIONS["timeout_ops"],
        )
        stats["operation_type"] = "runtime_node_health"
        stats["label"] = label
        stats["endpoint"] = url
        stats["cold_warm"] = "cold"
        stats["notes"] = "Expected failure - nodes are stopped per qualification state"
        results.append(stats)

    return results


def measure_lan_roundtrip() -> List[Dict[str, Any]]:
    """Dimension 6: LAN round-trip to router."""
    results = []

    import urllib.request
    import urllib.error

    # Localhost router call
    localhost_endpoints = [
        ("localhost_8080_health", "http://localhost:8080/health"),
        ("localhost_8080_status", "http://localhost:8080/backend/status"),
    ]

    for label, url in localhost_endpoints:
        def try_connect(u=url):
            try:
                req = urllib.request.Request(u, method="GET")
                with urllib.request.urlopen(req, timeout=3) as resp:
                    return resp.read()
            except Exception as e:
                raise ConnectionError(str(e))

        stats = time_operation(
            try_connect,
            iterations=ITERATIONS["network_ops"],
        )
        stats["operation_type"] = "lan_roundtrip"
        stats["label"] = label
        stats["endpoint"] = url
        stats["cold_warm"] = "cold"
        stats["notes"] = "localhost call - measures TCP stack overhead"
        results.append(stats)

    # Unreachable port test
    def try_unreachable():
        try:
            req = urllib.request.Request("http://localhost:19999/health", method="GET")
            with urllib.request.urlopen(req, timeout=2) as resp:
                return resp.read()
        except Exception as e:
            raise ConnectionError(str(e))

    unreachable_stats = time_operation(
        try_unreachable,
        iterations=ITERATIONS["timeout_ops"],
    )
    unreachable_stats["operation_type"] = "lan_roundtrip_unreachable"
    unreachable_stats["label"] = "unreachable_port"
    unreachable_stats["endpoint"] = "http://localhost:19999/health"
    unreachable_stats["cold_warm"] = "cold"
    unreachable_stats["notes"] = "Measures timeout cost for unreachable node"
    results.append(unreachable_stats)

    # Wrong port (router not running)
    def try_wrong_port():
        try:
            req = urllib.request.Request("http://localhost:8080/health", method="GET")
            with urllib.request.urlopen(req, timeout=2) as resp:
                return resp.read()
        except Exception as e:
            raise ConnectionError(str(e))

    wrong_port_stats = time_operation(
        try_wrong_port,
        iterations=ITERATIONS["timeout_ops"],
    )
    wrong_port_stats["operation_type"] = "lan_roundtrip_refused"
    wrong_port_stats["label"] = "router_port_refused"
    wrong_port_stats["endpoint"] = "http://localhost:8080/health"
    wrong_port_stats["cold_warm"] = "cold"
    wrong_port_stats["notes"] = "Router stopped - measures connection refused cost"
    results.append(wrong_port_stats)

    return results


def measure_small_append() -> List[Dict[str, Any]]:
    """Dimension 7: Small append payload latency (model agentic turn pattern)."""
    results = []

    # Simulate the small append pattern: serialize a small context + append
    for token_count in [200, 429, 600]:
        context = generate_payload(4096)  # 4K context
        append = generate_payload(token_count)

        # Serialize context + append
        combined = {
            "context": context,
            "append": append,
            "turn_id": int(time.time()),
        }

        serialize_stats = time_operation(
            lambda c=combined: json.dumps(c),
            iterations=ITERATIONS["small_ops"],
        )
        payload_bytes = len(json.dumps(combined).encode("utf-8"))
        serialize_stats["operation_type"] = "small_append_serialize"
        serialize_stats["payload_size_bytes"] = payload_bytes
        serialize_stats["approx_tokens"] = 4096 + token_count
        serialize_stats["label"] = f"append_{token_count}_tokens"
        serialize_stats["cold_warm"] = "warm"
        results.append(serialize_stats)

        # Write to file (simulate SSD persistence)
        out_path = TEMP_DIR / f"small-append-{token_count}.json"
        write_stats = time_operation(
            lambda c=combined, p=out_path: p.write_text(json.dumps(c), encoding="utf-8"),
            iterations=ITERATIONS["small_ops"],
        )
        write_stats["operation_type"] = "small_append_write"
        write_stats["payload_size_bytes"] = payload_bytes
        write_stats["approx_tokens"] = 4096 + token_count
        write_stats["label"] = f"append_{token_count}_tokens"
        write_stats["cold_warm"] = "warm"
        results.append(write_stats)

        # Read back
        read_stats = time_operation(
            lambda p=out_path: p.read_text(encoding="utf-8"),
            iterations=ITERATIONS["small_ops"],
        )
        read_stats["operation_type"] = "small_append_read"
        read_stats["payload_size_bytes"] = payload_bytes
        read_stats["approx_tokens"] = 4096 + token_count
        read_stats["label"] = f"append_{token_count}_tokens"
        read_stats["cold_warm"] = "warm"
        results.append(read_stats)

        out_path.unlink(missing_ok=True)

    return results


def measure_large_reused_context() -> List[Dict[str, Any]]:
    """Dimension 8: Large reused-context payload latency."""
    results = []

    for label, token_count in [("large_32k", 32700), ("very_large_64k", 64000)]:
        context = generate_payload(token_count)
        payload_bytes = len(context.encode("utf-8"))

        # Serialize to JSON
        obj = {"reused_context": context, "token_count": token_count}
        serialize_stats = time_operation(
            lambda o=obj: json.dumps(o),
            iterations=ITERATIONS["large_ops"],
        )
        serialize_stats["operation_type"] = "large_context_serialize"
        serialize_stats["payload_size_bytes"] = len(json.dumps(obj).encode("utf-8"))
        serialize_stats["approx_tokens"] = token_count
        serialize_stats["label"] = label
        serialize_stats["cold_warm"] = "warm"
        results.append(serialize_stats)

        # Write to file
        out_path = TEMP_DIR / f"large-context-{label}.json"
        write_stats = time_operation(
            lambda o=obj, p=out_path: p.write_text(json.dumps(o), encoding="utf-8"),
            iterations=ITERATIONS["large_ops"],
        )
        write_stats["operation_type"] = "large_context_write"
        write_stats["payload_size_bytes"] = len(json.dumps(obj).encode("utf-8"))
        write_stats["approx_tokens"] = token_count
        write_stats["label"] = label
        write_stats["cold_warm"] = "warm"
        results.append(write_stats)

        # Read back
        read_stats = time_operation(
            lambda p=out_path: p.read_text(encoding="utf-8"),
            iterations=ITERATIONS["large_ops"],
        )
        read_stats["operation_type"] = "large_context_read"
        read_stats["payload_size_bytes"] = len(json.dumps(obj).encode("utf-8"))
        read_stats["approx_tokens"] = token_count
        read_stats["label"] = label
        read_stats["cold_warm"] = "warm"
        results.append(read_stats)

        # Deserialize
        raw = out_path.read_text(encoding="utf-8")
        deserialize_stats = time_operation(
            lambda r=raw: json.loads(r),
            iterations=ITERATIONS["large_ops"],
        )
        deserialize_stats["operation_type"] = "large_context_deserialize"
        deserialize_stats["payload_size_bytes"] = len(raw.encode("utf-8"))
        deserialize_stats["approx_tokens"] = token_count
        deserialize_stats["label"] = label
        deserialize_stats["cold_warm"] = "warm"
        results.append(deserialize_stats)

        # Transfer cost simulation (serialize + write + read + deserialize)
        transfer_stats = time_operation(
            lambda o=obj, p=out_path: (
                p.write_text(json.dumps(o), encoding="utf-8"),
                p.read_text(encoding="utf-8"),
            ),
            iterations=ITERATIONS["large_ops"],
        )
        transfer_stats["operation_type"] = "large_context_transfer_roundtrip"
        transfer_stats["payload_size_bytes"] = len(json.dumps(obj).encode("utf-8"))
        transfer_stats["approx_tokens"] = token_count
        transfer_stats["label"] = label
        transfer_stats["cold_warm"] = "warm"
        results.append(transfer_stats)

        out_path.unlink(missing_ok=True)

    return results


def measure_degraded_node() -> List[Dict[str, Any]]:
    """Dimension 9: Weak/degraded node handling."""
    results = []

    import urllib.request
    import urllib.error

    # Runtime node stopped (should already be stopped)
    stopped_stats = time_operation(
        lambda: _try_health_check("http://localhost:8080/health", timeout=2),
        iterations=ITERATIONS["timeout_ops"],
    )
    stopped_stats["operation_type"] = "degraded_node_stopped"
    stopped_stats["label"] = "runtime_node_stopped"
    stopped_stats["notes"] = "LibrarianRunTimeNode is stopped per qualification state"
    results.append(stopped_stats)

    # Router port unavailable
    unavailable_stats = time_operation(
        lambda: _try_health_check("http://localhost:9999/health", timeout=2),
        iterations=ITERATIONS["timeout_ops"],
    )
    unavailable_stats["operation_type"] = "degraded_node_unavailable"
    unavailable_stats["label"] = "router_port_unavailable"
    unavailable_stats["notes"] = "Port 9999 not in use"
    results.append(unavailable_stats)

    # Timeout path (using very short timeout)
    timeout_stats = time_operation(
        lambda: _try_health_check("http://localhost:8080/health", timeout=1),
        iterations=ITERATIONS["timeout_ops"],
    )
    timeout_stats["operation_type"] = "degraded_node_timeout"
    timeout_stats["label"] = "health_check_timeout"
    timeout_stats["notes"] = "1 second timeout on stopped node"
    results.append(timeout_stats)

    # Wrong port (simulating degraded LAN)
    wrong_port_stats = time_operation(
        lambda: _try_health_check("http://localhost:8081/health", timeout=2),
        iterations=ITERATIONS["timeout_ops"],
    )
    wrong_port_stats["operation_type"] = "degraded_node_wrong_port"
    wrong_port_stats["label"] = "wrong_port_degraded_lan"
    wrong_port_stats["notes"] = "Simulates degraded LAN routing"
    results.append(wrong_port_stats)

    return results


def _try_health_check(url: str, timeout: int = 2) -> bool:
    """Try a health check, return True if successful."""
    import urllib.request
    try:
        req = urllib.request.Request(url, method="GET")
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return resp.status == 200
    except Exception:
        raise ConnectionError(f"Health check failed: {url}")


# ---------------------------------------------------------------------------
# Main measurement runner
# ---------------------------------------------------------------------------

def collect_system_info() -> Dict[str, Any]:
    """Collect system information for the measurement profile."""
    import platform
    import os

    info = {
        "platform": platform.platform(),
        "python_version": platform.python_version(),
        "processor": platform.processor(),
        "machine": platform.machine(),
        "node": platform.node(),
        "os_name": platform.system(),
        "os_release": platform.release(),
    }

    # Try to get disk info
    try:
        import shutil
        usage = shutil.disk_usage("G:\\")
        info["disk_total_gb"] = round(usage.total / (1024**3), 2)
        info["disk_free_gb"] = round(usage.free / (1024**3), 2)
    except Exception:
        pass

    # Try to get memory info
    try:
        import ctypes
        kernel32 = ctypes.windll.kernel32
        c_ulonglong = ctypes.c_ulonglong
        class MEMORYSTATUSEX(ctypes.Structure):
            _fields_ = [
                ("dwLength", ctypes.c_ulong),
                ("dwMemoryLoad", ctypes.c_ulong),
                ("ullTotalPhys", c_ulonglong),
                ("ullAvailPhys", c_ulonglong),
                ("ullTotalPageFile", c_ulonglong),
                ("ullAvailPageFile", c_ulonglong),
                ("ullTotalVirtual", c_ulonglong),
                ("ullAvailVirtual", c_ulonglong),
                ("ullAvailExtendedVirtual", c_ulonglong),
            ]
        mem = MEMORYSTATUSEX()
        mem.dwLength = ctypes.sizeof(MEMORYSTATUSEX)
        kernel32.GlobalMemoryStatusEx(ctypes.byref(mem))
        info["ram_total_gb"] = round(mem.ullTotalPhys / (1024**3), 2)
        info["ram_free_gb"] = round(mem.ullAvailPhys / (1024**3), 2)
    except Exception:
        pass

    return info


def run_all_measurements() -> Dict[str, Any]:
    """Run all measurement dimensions and compile results."""
    print("=" * 70)
    print("MAC/WIN-ROUTER-CONTEXT-MEASURE-1 — Context Movement Cost Measurement")
    print("=" * 70)
    print(f"Start time: {datetime.now(timezone.utc).isoformat()}")
    print(f"Repo root: {REPO_ROOT}")
    print()

    system_info = collect_system_info()
    print(f"System: {system_info.get('platform', 'unknown')}")
    print(f"RAM: {system_info.get('ram_total_gb', '?')} GB total, {system_info.get('ram_free_gb', '?')} GB free")
    print(f"Disk: {system_info.get('disk_total_gb', '?')} GB total, {system_info.get('disk_free_gb', '?')} GB free")
    print()

    all_results = {}

    # Dimension 1: File I/O
    print("[1/9] Measuring local file read/write for context-sized payloads...")
    all_results["file_io"] = measure_file_io()
    print(f"      -> {len(all_results['file_io'])} measurements recorded")
    print()

    # Dimension 2: JSON Processing
    print("[2/9] Measuring JSON load/parse/serialize...")
    all_results["json_processing"] = measure_json_processing()
    print(f"      -> {len(all_results['json_processing'])} measurements recorded")
    print()

    # Dimension 3: Recall Packet
    print("[3/9] Measuring recall packet serialize/deserialize...")
    all_results["recall_packet"] = measure_recall_packet()
    print(f"      -> {len(all_results['recall_packet'])} measurements recorded")
    print()

    # Dimension 4: Canonical Evidence Read
    print("[4/9] Measuring canonical evidence read operations...")
    all_results["canonical_evidence"] = measure_canonical_evidence()
    print(f"      -> {len(all_results['canonical_evidence'])} measurements recorded")
    print()

    # Dimension 5: Runtime Node Health
    print("[5/9] Measuring runtime-node health latency...")
    all_results["runtime_health"] = measure_runtime_node_health()
    print(f"      -> {len(all_results['runtime_health'])} measurements recorded")
    print()

    # Dimension 6: LAN Round-trip
    print("[6/9] Measuring LAN round-trip to router...")
    all_results["lan_roundtrip"] = measure_lan_roundtrip()
    print(f"      -> {len(all_results['lan_roundtrip'])} measurements recorded")
    print()

    # Dimension 7: Small Append
    print("[7/9] Measuring small append payload latency...")
    all_results["small_append"] = measure_small_append()
    print(f"      -> {len(all_results['small_append'])} measurements recorded")
    print()

    # Dimension 8: Large Reused Context
    print("[8/9] Measuring large reused-context payload latency...")
    all_results["large_reused_context"] = measure_large_reused_context()
    print(f"      -> {len(all_results['large_reused_context'])} measurements recorded")
    print()

    # Dimension 9: Degraded Node
    print("[9/9] Measuring weak/degraded node handling...")
    all_results["degraded_node"] = measure_degraded_node()
    print(f"      -> {len(all_results['degraded_node'])} measurements recorded")
    print()

    # Compile final output
    total_measurements = sum(len(v) for v in all_results.values())
    print("=" * 70)
    print(f"Total measurements: {total_measurements}")
    print(f"End time: {datetime.now(timezone.utc).isoformat()}")
    print("=" * 70)

    return {
        "metadata": {
            "sprint_id": "MAC/WIN-ROUTER-CONTEXT-MEASURE-1",
            "measurement_version": "1.0.0",
            "start_time": datetime.now(timezone.utc).isoformat(),
            "repo_root": str(REPO_ROOT),
            "total_measurements": total_measurements,
            "token_bytes_approximation": BYTES_PER_TOKEN,
            "iteration_config": ITERATIONS,
            "payload_sizes_tokens": PAYLOAD_SIZES,
        },
        "system_info": system_info,
        "results": all_results,
    }


def save_results(output: Dict[str, Any]) -> Tuple[str, str]:
    """Save measurement results and hardware profiles."""
    REPORTS_DIR.mkdir(parents=True, exist_ok=True)

    # Save raw results
    results_path = REPORTS_DIR / "router-context-measure-results.json"
    with open(results_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, default=str)
    print(f"\nResults saved to: {results_path}")

    # Build calibrated profiles from measurements
    profiles = build_hardware_profiles(output)
    profiles_path = CONFIG_DIR / "measured_hardware_profiles.json"
    with open(profiles_path, "w", encoding="utf-8") as f:
        json.dump(profiles, f, indent=2, default=str)
    print(f"Profiles saved to: {profiles_path}")

    return str(results_path), str(profiles_path)


def build_hardware_profiles(output: Dict[str, Any]) -> Dict[str, Any]:
    """Build calibrated hardware profiles from measurement results."""
    results = output["results"]

    # Extract key measurements for profile calibration
    def find_measurement(measurements, op_type, label=None):
        for m in measurements:
            if m.get("operation_type") == op_type:
                if label is None or m.get("label") == label:
                    return m
        return None

    def find_all_measurements(measurements, op_type):
        return [m for m in measurements if m.get("operation_type") == op_type]

    # File I/O measurements
    file_read_warm = find_all_measurements(results.get("file_io", []), "file_read_warm")
    file_read_cold = find_all_measurements(results.get("file_io", []), "file_read_cold")

    # JSON measurements
    json_parse_warm = find_all_measurements(results.get("json_processing", []), "json_parse_warm")
    json_serialize = find_all_measurements(results.get("json_processing", []), "json_serialize")

    # Recall measurements
    recall_serialize = find_all_measurements(results.get("recall_packet", []), "recall_serialize")
    recall_deserialize = find_all_measurements(results.get("recall_packet", []), "recall_deserialize")
    recall_compress = find_all_measurements(results.get("recall_packet", []), "recall_compress")
    recall_decompress = find_all_measurements(results.get("recall_packet", []), "recall_decompress")

    # Evidence measurements
    git_status = find_measurement(results.get("canonical_evidence", []), "canonical_evidence_read", "git_status_short")
    git_revparse = find_measurement(results.get("canonical_evidence", []), "canonical_evidence_read", "git_revparse_short_head")

    # Network measurements
    lan_unreachable = find_measurement(results.get("lan_roundtrip", []), "lan_roundtrip_unreachable")
    lan_refused = find_measurement(results.get("lan_roundtrip", []), "lan_roundtrip_refused")
    degraded_stopped = find_measurement(results.get("degraded_node", []), "degraded_node_stopped")

    # Build profiles
    profiles = {
        "_comment": "Calibrated hardware/runtime profiles from measured data. MAC/WIN-ROUTER-CONTEXT-MEASURE-1.",
        "_honesty": "Measured on Windows runtime node only. Mac measurements not_measured_in_this_sprint.",
        "measurement_metadata": {
            "sprint_id": "MAC/WIN-ROUTER-CONTEXT-MEASURE-1",
            "measurement_date": datetime.now(timezone.utc).isoformat(),
            "system_info": output["system_info"],
        },
        "mac_coordinator": {
            "status": "not_measured_in_this_sprint",
            "description": "Mac measurements not available - Owner on PC during measurement sprint.",
        },
        "windows_runtime_node": {
            "status": "measured",
            "description": "Windows desktop workstation with Big Pickle RX 570 4GB.",
            "measured_file_read_warm_ms": {
                "small_append_429tok": _avg([m["median_latency_ms"] for m in file_read_warm if m.get("label") == "small_append"]),
                "medium_context_8k": _avg([m["median_latency_ms"] for m in file_read_warm if m.get("label") == "medium_context"]),
                "large_reused_32k": _avg([m["median_latency_ms"] for m in file_read_warm if m.get("label") == "large_reused_context"]),
                "very_large_128k": _avg([m["median_latency_ms"] for m in file_read_warm if m.get("label") == "very_large_context"]),
            },
            "measured_file_read_cold_ms": {
                "small_append_429tok": _avg([m["median_latency_ms"] for m in file_read_cold if m.get("label") == "small_append"]),
                "medium_context_8k": _avg([m["median_latency_ms"] for m in file_read_cold if m.get("label") == "medium_context"]),
                "large_reused_32k": _avg([m["median_latency_ms"] for m in file_read_cold if m.get("label") == "large_reused_context"]),
                "very_large_128k": _avg([m["median_latency_ms"] for m in file_read_cold if m.get("label") == "very_large_context"]),
            },
            "measured_json_parse_warm_ms": _avg([m["median_latency_ms"] for m in json_parse_warm]),
            "measured_json_serialize_ms": _avg([m["median_latency_ms"] for m in json_serialize]),
            "measured_recall_serialize_ms": _avg([m["median_latency_ms"] for m in recall_serialize]) if recall_serialize else None,
            "measured_recall_deserialize_ms": _avg([m["median_latency_ms"] for m in recall_deserialize]) if recall_deserialize else None,
            "measured_recall_compress_ms": _avg([m["median_latency_ms"] for m in recall_compress]) if recall_compress else None,
            "measured_recall_decompress_ms": _avg([m["median_latency_ms"] for m in recall_decompress]) if recall_decompress else None,
            "measured_git_status_ms": git_status["median_latency_ms"] if git_status else None,
            "measured_git_revparse_ms": git_revparse["median_latency_ms"] if git_revparse else None,
            "node_stopped_connection_refused_ms": degraded_stopped["median_latency_ms"] if degraded_stopped else None,
        },
        "weak_lan_runtime_node": {
            "status": "synthetic_reference_only",
            "description": "Derived from measured timeout/unreachable costs. Not directly measured on weak LAN.",
            "unreachable_timeout_ms": lan_unreachable["median_latency_ms"] if lan_unreachable else None,
            "connection_refused_ms": lan_refused["median_latency_ms"] if lan_refused else None,
        },
        "local_file_io_profile": {
            "status": "measured",
            "description": "Local SSD/file system I/O profile from measurement data.",
            "cold_write_ms_per_429tok": _avg([m["median_latency_ms"] for m in results.get("file_io", []) if m.get("operation_type") == "file_write_cold" and m.get("label") == "small_append"]),
            "cold_read_ms_per_429tok": _avg([m["median_latency_ms"] for m in file_read_cold if m.get("label") == "small_append"]),
            "warm_read_ms_per_429tok": _avg([m["median_latency_ms"] for m in file_read_warm if m.get("label") == "small_append"]),
            "cold_write_ms_per_128k": _avg([m["median_latency_ms"] for m in results.get("file_io", []) if m.get("operation_type") == "file_write_cold" and m.get("label") == "very_large_context"]),
            "cold_read_ms_per_128k": _avg([m["median_latency_ms"] for m in file_read_cold if m.get("label") == "very_large_context"]),
            "warm_read_ms_per_128k": _avg([m["median_latency_ms"] for m in file_read_warm if m.get("label") == "very_large_context"]),
        },
    }

    return profiles


def _avg(values):
    """Calculate average of non-None values."""
    valid = [v for v in values if v is not None]
    return round(statistics.mean(valid), 4) if valid else None


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    output = run_all_measurements()
    results_path, profiles_path = save_results(output)
    print(f"\nDone. Results: {results_path}")
    print(f"Profiles: {profiles_path}")
