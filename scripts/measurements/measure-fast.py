#!/usr/bin/env python3
"""
MAC/WIN-ROUTER-CONTEXT-MEASURE-1 — Fast Context Movement Cost Measurement

Optimized for speed. Measures core dimensions with reduced iteration counts.
"""

import json
import os
import sys
import time
import statistics
import subprocess
import zlib
from pathlib import Path
from datetime import datetime, timezone

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
FIXTURES_DIR = REPO_ROOT / "fixtures" / "context-route"
CONFIG_DIR = REPO_ROOT / "config"
REPORTS_DIR = REPO_ROOT / "reports"
TEMP_DIR = REPO_ROOT / "temp"
TEMP_DIR.mkdir(parents=True, exist_ok=True)

BYTES_PER_TOKEN = 4

def gen_payload(tokens):
    base = ("the quick brown fox jumps over lazy dog context route recall packet "
            "evidence receipt sprint workload optimizer cache memory storage "
            "runtime node health status profile model gateway protocol network ")
    need = tokens * BYTES_PER_TOKEN
    return (base * ((need // len(base)) + 1))[:need]

def time_it(func, n=20):
    times = []
    errs = 0
    for _ in range(n):
        try:
            t0 = time.perf_counter_ns()
            func()
            t1 = time.perf_counter_ns()
            times.append((t1 - t0) / 1e6)
        except:
            errs += 1
            # For timeout/error cases, still record the time
            t1 = time.perf_counter_ns()
            times.append((t1 - t0) / 1e6)
    if not times:
        return {"min_ms": None, "max_ms": None, "mean_ms": None, "median_ms": None,
                "p95_ms": None, "std_ms": None, "n": n, "ok": 0, "err": errs}
    s = sorted(times)
    return {
        "min_ms": round(s[0], 4), "max_ms": round(s[-1], 4),
        "mean_ms": round(statistics.mean(times), 4),
        "median_ms": round(statistics.median(times), 4),
        "p95_ms": round(s[int(len(s)*0.95)], 4),
        "std_ms": round(statistics.stdev(times), 4) if len(times) > 1 else 0,
        "n": n, "ok": len(times), "err": errs
    }

print("=" * 60)
print("MAC/WIN-ROUTER-CONTEXT-MEASURE-1 — Fast Measurement")
print("=" * 60)
print(f"Start: {datetime.now(timezone.utc).isoformat()}")

results = {}

# --- 1. File I/O ---
print("\n[1/7] File I/O...")
file_io = {}
for label, tok in [("small_429", 429), ("medium_8k", 8000), ("large_32k", 32700)]:
    payload = gen_payload(tok)
    fp = TEMP_DIR / f"m-{label}.txt"
    fp.write_text(payload, encoding="utf-8")
    sz = fp.stat().st_size

    w = time_it(lambda p=payload, f=fp: f.write_text(p, encoding="utf-8"), n=30)
    w["op"] = "write"; w["label"] = label; w["bytes"] = sz; w["tokens"] = tok

    r = time_it(lambda f=fp: f.read_text(encoding="utf-8"), n=30)
    r["op"] = "read_cold"; r["label"] = label; r["bytes"] = sz; r["tokens"] = tok

    rw = time_it(lambda f=fp: f.read_text(encoding="utf-8"), n=30)
    rw["op"] = "read_warm"; rw["label"] = label; rw["bytes"] = sz; rw["tokens"] = tok

    file_io[label] = {"write": w, "read_cold": r, "read_warm": rw}
    fp.unlink(missing_ok=True)
    print(f"  {label}: write={w['median_ms']:.2f}ms read_cold={r['median_ms']:.2f}ms read_warm={rw['median_ms']:.2f}ms")

results["file_io"] = file_io

# --- 2. JSON Processing ---
print("\n[2/7] JSON Processing...")
json_proc = {}
fixtures = list(FIXTURES_DIR.glob("*.json"))
fixture_results = []
for fx in fixtures[:5]:  # Limit to 5 fixtures
    raw = fx.read_bytes()
    sz = len(raw)

    rd = time_it(lambda f=fx: f.read_bytes(), n=30)
    rd["op"] = "read"; rd["label"] = fx.stem; rd["bytes"] = sz

    pr = time_it(lambda d=raw: json.loads(d), n=30)
    pr["op"] = "parse"; pr["label"] = fx.stem; pr["bytes"] = sz

    obj = json.loads(raw)
    se = time_it(lambda o=obj: json.dumps(o, separators=(",", ":")), n=30)
    se["op"] = "serialize"; se["label"] = fx.stem; se["bytes"] = sz

    fixture_results.append({"name": fx.stem, "read": rd, "parse": pr, "serialize": se})

# Synthetic
synth_results = {}
for label, tok in [("small_429", 429), ("medium_8k", 8000), ("large_32k", 32700)]:
    obj = {"context": gen_payload(tok), "tokens": tok}
    raw = json.dumps(obj)
    sz = len(raw.encode("utf-8"))

    pr = time_it(lambda r=raw: json.loads(r), n=20)
    pr["op"] = "parse_synthetic"; pr["label"] = label; pr["bytes"] = sz

    se = time_it(lambda o=obj: json.dumps(o, separators=(",", ":")), n=20)
    se["op"] = "serialize_synthetic"; se["label"] = label; se["bytes"] = sz

    synth_results[label] = {"parse": pr, "serialize": se}
    print(f"  {label}: parse={pr['median_ms']:.2f}ms serialize={se['median_ms']:.2f}ms")

results["json_processing"] = {"fixtures": fixture_results, "synthetic": synth_results}

# --- 3. Recall Packet ---
print("\n[3/7] Recall Packet...")
recall = {}
for label, tok in [("compact_5k", 5000), ("medium_32k", 32700), ("large_64k", 64000)]:
    packet = {
        "recall_id": f"recall-{label}",
        "context_summary": gen_payload(tok // 2),
        "decisions_log": [{"turn": i, "action": f"act_{i}"} for i in range(min(tok // 500, 40))],
        "state": {"files": ["a.py", "b.py"], "governance": "verified"},
    }
    raw = json.dumps(packet)
    sz = len(raw.encode("utf-8"))

    se = time_it(lambda p=packet: json.dumps(p), n=15)
    de = time_it(lambda r=raw: json.loads(r), n=15)
    comp = time_it(lambda r=raw: zlib.compress(r.encode("utf-8")), n=15)
    compressed = zlib.compress(raw.encode("utf-8"))
    decomp = time_it(lambda c=compressed: zlib.decompress(c), n=15)

    recall[label] = {
        "serialize": {**se, "op": "serialize", "bytes": sz},
        "deserialize": {**de, "op": "deserialize", "bytes": sz},
        "compress": {**comp, "op": "compress", "bytes": sz, "compressed_bytes": len(compressed)},
        "decompress": {**decomp, "op": "decompress", "bytes": sz, "compressed_bytes": len(compressed)},
    }
    ratio = len(compressed) / sz * 100 if sz > 0 else 0
    print(f"  {label}: ser={se['median_ms']:.2f}ms deser={de['median_ms']:.2f}ms "
          f"comp={comp['median_ms']:.2f}ms decomp={decomp['median_ms']:.2f}ms ratio={ratio:.1f}%")

results["recall_packet"] = recall

# --- 4. Canonical Evidence ---
print("\n[4/7] Canonical Evidence...")
evidence = {}

r = time_it(lambda: subprocess.run(["git", "status", "--short"], cwd=str(REPO_ROOT),
             capture_output=True, text=True, timeout=10), n=20)
r["op"] = "git_status"; r["method"] = "subprocess"
evidence["git_status"] = r
print(f"  git status: {r['median_ms']:.2f}ms")

r = time_it(lambda: subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=str(REPO_ROOT),
             capture_output=True, text=True, timeout=10), n=20)
r["op"] = "git_revparse"; r["method"] = "subprocess"
evidence["git_revparse"] = r
print(f"  git rev-parse: {r['median_ms']:.2f}ms")

for name, path in [("model_profiles", CONFIG_DIR / "model-profiles.json"),
                    ("sprint_doc", REPO_ROOT / "docs" / "sprints" / "WIN-RUNTIME-QUALIFICATION-1.md"),
                    ("contract_fixture", FIXTURES_DIR / "sprint-planning.json")]:
    if path.exists():
        sz = path.stat().st_size
        r = time_it(lambda p=path: p.read_text(encoding="utf-8"), n=20)
        r["op"] = "file_read"; r["label"] = name; r["bytes"] = sz
        evidence[name] = r
        print(f"  {name}: {r['median_ms']:.2f}ms ({sz} bytes)")

results["canonical_evidence"] = evidence

# --- 5. Runtime Health ---
print("\n[5/7] Runtime Health (expected failures)...")
import urllib.request, urllib.error
health = {}
for label, url in [("health_8080", "http://localhost:8080/health"),
                    ("status_8080", "http://localhost:8080/backend/status"),
                    ("health_9120", "http://localhost:9120/health")]:
    def check(u=url):
        try:
            urllib.request.urlopen(urllib.request.Request(u), timeout=2)
        except:
            raise ConnectionError("expected")
    r = time_it(check, n=3)
    r["op"] = "health_check"; r["label"] = label; r["url"] = url
    r["notes"] = "Expected failure - node stopped"
    health[label] = r
    print(f"  {label}: {r['median_ms']:.2f}ms (expected fail)")

results["runtime_health"] = health

# --- 6. LAN Round-trip ---
print("\n[6/7] LAN Round-trip...")
lan = {}
def unreachable():
    try:
        urllib.request.urlopen(urllib.request.Request("http://localhost:19999/"), timeout=2)
    except:
        raise ConnectionError("expected")

r = time_it(unreachable, n=3)
r["op"] = "unreachable"; r["label"] = "port_19999"
lan["unreachable"] = r
print(f"  unreachable: {r['median_ms']:.2f}ms")

def refused():
    try:
        urllib.request.urlopen(urllib.request.Request("http://localhost:8080/health"), timeout=2)
    except:
        raise ConnectionError("expected")

r = time_it(refused, n=3)
r["op"] = "refused"; r["label"] = "port_8080_refused"
lan["refused"] = r
print(f"  refused: {r['median_ms']:.2f}ms")

results["lan_roundtrip"] = lan

# --- 7. Small Append + Large Context ---
print("\n[7/7] Append & Large Context...")
append_results = {}
for tok in [200, 429, 600]:
    ctx = gen_payload(4096)
    app = gen_payload(tok)
    combined = {"context": ctx, "append": app}
    raw = json.dumps(combined)
    sz = len(raw.encode("utf-8"))

    se = time_it(lambda c=combined: json.dumps(c), n=20)
    fp = TEMP_DIR / f"app-{tok}.json"
    wr = time_it(lambda c=combined, f=fp: f.write_text(json.dumps(c), encoding="utf-8"), n=20)
    rd = time_it(lambda f=fp: f.read_text(encoding="utf-8"), n=20)
    fp.unlink(missing_ok=True)

    append_results[f"append_{tok}"] = {
        "serialize": {**se, "bytes": sz, "tokens": 4096 + tok},
        "write": {**wr, "bytes": sz},
        "read": {**rd, "bytes": sz},
    }
    print(f"  append_{tok}: ser={se['median_ms']:.2f}ms wr={wr['median_ms']:.2f}ms rd={rd['median_ms']:.2f}ms")

results["small_append"] = append_results

# Large context
large_ctx = {}
for label, tok in [("large_32k", 32700), ("large_64k", 64000)]:
    obj = {"reused_context": gen_payload(tok)}
    raw = json.dumps(obj)
    sz = len(raw.encode("utf-8"))

    se = time_it(lambda o=obj: json.dumps(o), n=10)
    fp = TEMP_DIR / f"lc-{label}.json"
    wr = time_it(lambda o=obj, f=fp: f.write_text(json.dumps(o), encoding="utf-8"), n=10)
    rd = time_it(lambda f=fp: f.read_text(encoding="utf-8"), n=10)
    raw2 = fp.read_text(encoding="utf-8")
    de = time_it(lambda r=raw2: json.loads(r), n=10)
    fp.unlink(missing_ok=True)

    large_ctx[label] = {
        "serialize": {**se, "bytes": sz, "tokens": tok},
        "write": {**wr, "bytes": sz},
        "read": {**rd, "bytes": sz},
        "deserialize": {**de, "bytes": sz},
    }
    print(f"  {label}: ser={se['median_ms']:.2f}ms wr={wr['median_ms']:.2f}ms "
          f"rd={rd['median_ms']:.2f}ms deser={de['median_ms']:.2f}ms")

results["large_context"] = large_ctx

# --- System Info ---
sys_info = {"platform": "Windows", "measurement_time": datetime.now(timezone.utc).isoformat()}
try:
    import platform
    sys_info["os"] = platform.platform()
    sys_info["python"] = platform.python_version()
    sys_info["machine"] = platform.machine()
except:
    pass

# --- Save ---
output = {
    "metadata": {
        "sprint_id": "MAC/WIN-ROUTER-CONTEXT-MEASURE-1",
        "version": "1.0.0",
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "repo_root": str(REPO_ROOT),
        "bytes_per_token": BYTES_PER_TOKEN,
    },
    "system_info": sys_info,
    "results": results,
}

REPORTS_DIR.mkdir(parents=True, exist_ok=True)
results_path = REPORTS_DIR / "router-context-measure-results.json"
with open(results_path, "w", encoding="utf-8") as f:
    json.dump(output, f, indent=2, default=str)

# Build profiles
profiles = {
    "_comment": "Calibrated profiles from MAC/WIN-ROUTER-CONTEXT-MEASURE-1 measurements",
    "windows_runtime_node": {
        "status": "measured",
        "file_read_warm_ms": {
            "429tok": results["file_io"]["small_429"]["read_warm"]["median_ms"],
            "8ktok": results["file_io"]["medium_8k"]["read_warm"]["median_ms"],
            "32ktok": results["file_io"]["large_32k"]["read_warm"]["median_ms"],
        },
        "json_parse_warm_ms": results["json_processing"]["synthetic"]["medium_8k"]["parse"]["median_ms"],
        "json_serialize_ms": results["json_processing"]["synthetic"]["medium_8k"]["serialize"]["median_ms"],
        "git_status_ms": results["canonical_evidence"]["git_status"]["median_ms"],
        "git_revparse_ms": results["canonical_evidence"]["git_revparse"]["median_ms"],
    },
    "mac_coordinator": {"status": "not_measured_in_this_sprint"},
    "weak_lan_runtime_node": {
        "status": "derived_from_measurements",
        "unreachable_timeout_ms": results["lan_roundtrip"]["unreachable"]["median_ms"],
        "connection_refused_ms": results["lan_roundtrip"]["refused"]["median_ms"],
    },
}

profiles_path = CONFIG_DIR / "measured_hardware_profiles.json"
with open(profiles_path, "w", encoding="utf-8") as f:
    json.dump(profiles, f, indent=2, default=str)

print(f"\n{'=' * 60}")
print(f"Results: {results_path}")
print(f"Profiles: {profiles_path}")
print(f"End: {datetime.now(timezone.utc).isoformat()}")
