# Windows Librarian Host Options

**Status:** Draft
**Date:** 2026-06-29
**Plan ref:** `docs/planning/WIN-AGENT-HARNESS-PLAN.md`
**Baseline ref:** `docs/planning/WIN-AGENT-HARNESS-ENV-BASELINE-1-BASELINE.md`

---

## 1. Purpose

Evaluate host technology options for a future Windows version of The Librarian — a governed Owner-facing app that preserves custody, receipts, validation, approval, and non-canonical model output boundaries.

This document does not decide the architecture. It surveys the available options against the constraints revealed by the baseline inventory and the custody sandbox model.

---

## 2. Constraints from Baseline

| Constraint | Source | Impact |
|------------|--------|--------|
| No Swift toolchain | Baseline F-002 (inferred) | Eliminates SwiftUI, native Swift on Windows |
| .NET SDK not installed | Baseline F-002 | Eliminates WPF/WinUI unless SDK is added |
| Node.js v24.14.0 is available | Baseline §12 | Enables Electron, Tauri (Node side) |
| Rust 1.96.0 is available | Baseline §13 | Enables Tauri (Rust side), native Rust |
| Python 3.14.3 is available | Baseline §11 | Enables local web server, Flask/FastAPI UI |
| GPU: RX 570 4 GB (Vulkan) | Baseline §6 | Native graphics options feasible |
| RAM: 24.3 GB | Baseline §5 | Electron/Tauri memory overhead acceptable |
| C: drive critically low (10 GB) | Baseline F-001 | Large SDK installs may fail |
| Non-admin shell | Baseline elevation check | Service/install actions elevated only |
| Vulkan SDK installed | Baseline §15 | Native Vulkan rendering possible |

---

## 3. Options Survey

### Option A: Rust Native (Tauri)

| Dimension | Assessment |
|-----------|------------|
| **UI** | Web-based frontend (HTML/JS/React) with Rust backend |
| **Custody model** | Rust backend can enforce custody natively |
| **Receipt integration** | Can read/write JSON receipts directly |
| **Service integration** | Can manage Windows service via `windows-service` crate |
| **Existing code reuse** | Rust router exists at `rust-router/` — could share types |
| **Build toolchain** | ✅ Rust 1.96.0 installed. MSVC toolchain available via VS BuildTools |
| **SDK install** | No additional SDK needed |
| **Disk impact** | ~500 MB for Rust toolchain (already installed) |
| **Elevation** | Non-admin for development; admin only for service install |
| **Maturity** | Tauri 2.x stable, active community |

**Verdict:** ✅ Strong candidate. Rust toolchain is already installed and proven. Tauri provides a modern UI with a custody-capable Rust backend.

### Option B: Electron

| Dimension | Assessment |
|-----------|------------|
| **UI** | Web-based frontend (HTML/JS/React) with Node.js backend |
| **Custody model** | Node.js backend with sandboxing (limited compared to Rust) |
| **Receipt integration** | Can read/write JSON receipts |
| **Service integration** | Via child_process (limited) or native addon |
| **Existing code reuse** | Node.js v24.14.0 available |
| **Build toolchain** | ✅ Node.js installed |
| **SDK install** | `npm install electron` — minimal |
| **Disk impact** | ~200 MB for Electron + app deps |
| **Elevation** | Non-admin for development |
| **Maturity** | Electron 30+, very mature |

**Verdict:** ✅ Feasible. Node.js is available. Electron is well-understood but heavier than Tauri and offers weaker custody guarantees at the OS boundary.

### Option C: .NET / WPF / WinUI

| Dimension | Assessment |
|-----------|------------|
| **UI** | Native Windows (WPF, WinUI 3) |
| **Custody model** | .NET has strong sandboxing via AppDomains (limited in .NET Core) |
| **Receipt integration** | Can read/write JSON via System.Text.Json |
| **Service integration** | Native via System.ServiceProcess |
| **Existing code reuse** | None — no existing .NET code in repo |
| **Build toolchain** | ❌ .NET SDK not installed |
| **SDK install** | Requires ~600 MB SDK download |
| **Disk impact** | C: drive has only 10 GB free — SDK install may fail |
| **Elevation** | Non-admin for development |
| **Maturity** | WinUI 3 mature but Windows-only |

**Verdict:** ⚠️ Blocked by C: drive capacity. `dotnet --version` already failed (no SDK). Installing the .NET SDK on a drive with 10 GB free is risky. Revisit after disk-space triage.

### Option D: Local Web App (Flask/FastAPI)

| Dimension | Assessment |
|-----------|------------|
| **UI** | Browser-based (localhost) |
| **Custody model** | Python backend with explicit auth/advisory layer |
| **Receipt integration** | Python can read/write JSON natively |
| **Service integration** | Via subprocess (similar to current router) |
| **Existing code reuse** | Python router exists at `router/router.py` — could extend |
| **Build toolchain** | ✅ Python 3.14.3 installed |
| **SDK install** | No additional SDK needed |
| **Disk impact** | Minimal (pip install) |
| **Elevation** | Non-admin for development |
| **Maturity** | Flask/FastAPI production-ready |

**Verdict:** ✅ Fastest path to a working UI. Suitable for prototyping. The custody model would need a dedicated backend layer — the current Python router is reference-only.

### Option E: Windows App SDK / WinUI 3 (C++/WinRT)

| Dimension | Assessment |
|-----------|------------|
| **UI** | Native Windows (WinUI 3) |
| **Custody model** | C++ backend via WinRT |
| **Receipt integration** | Via C++ JSON libraries |
| **Service integration** | Win32 API access |
| **Existing code reuse** | None |
| **Build toolchain** | ⚠️ MSVC BuildTools installed but cl.exe not in PATH |
| **SDK install** | Windows App SDK requires additional install |
| **Elevation** | Admin for full Win32 access |
| **Maturity** | Mature but complex |

**Verdict:** ❌ High complexity, no existing toolchain in PATH, and significant disk requirements. Not recommended as a starting point.

---

## 4. Comparative Matrix

| Criterion | Rust/Tauri | Electron | .NET/WPF | Python Web | WinUI/C++ |
|-----------|-----------|----------|----------|------------|-----------|
| Toolchain ready | ✅ | ✅ | ❌ | ✅ | ❌ |
| Custody model strength | Strong | Medium | Medium | Weak | Strong |
| Disk space required | Low | Low | High | Minimal | High |
| Service integration | Native | Wrapper | Native | Subprocess | Native |
| Existing code reuse | Medium | Low | None | High | None |
| Cross-platform potential | High | High | Low | Medium | None |
| Development speed | Medium | Fast | Medium | Fast | Slow |
| UI quality | Modern | Modern | Native | Browser | Native |

---

## 5. Recommendation

### Phase 0 (Immediate): Python Web Prototype
The fastest path to a working Windows Librarian UI prototype is a local web app served by Python (Flask or FastAPI). The Python router already exists at `router/router.py` and can serve as a foundation. A web UI removes all build-toolchain dependencies and works within the current disk constraints.

**Use case:** Prove the Windows Librarian UI model, custody display, and receipt visualization before committing to a native app framework.

### Phase 1 (Near-term): Rust/Tauri
Once the custody model is validated through the Python prototype, migrate to Rust/Tauri for:
- Stronger custody guarantees at the OS boundary
- Native Windows service integration
- Shared types with the existing `rust-router` crate
- Smaller binary and better performance

### Phase 2 (Future): Cross-platform Consideration
If the Windows Librarian needs Mac parity, Tauri's cross-platform support makes it the strongest candidate. A Tauri app can share the same Rust custody backend across Windows and macOS, with platform-specific shells.

---

## 6. Disk-Space Gate

**None of these options can proceed if C: drive free space drops below 5 GB.** The first Phase 0 prototype (Python web) is the least disk-intensive and can proceed at current free space (10.2 GB). Any option requiring a new SDK install (.NET, Windows App SDK) must wait for disk-space triage.
