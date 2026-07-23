# Sprint: WIN-BACKEND-SERVICE-PROOF-1
**Status:** COMPLETED
**Date:** 2026-06-19

## Objective
Prove that when `LibrarianRunTimeNode` is started as a Windows service, a backend-starting request causes `llama-server.exe` to launch under the governed runtime path, and stopping the service terminates the backend with no orphan process.

## Final Result
- **Service Lifecycle:** VERIFIED.
- **Backend Launch:** VERIFIED. `POST /backend/select` successfully triggered `llama-server.exe`.
- **Parentage Proof:** VERIFIED. Backend PID 17632 was launched by Router PID 13812.
- **Orphan Process Check:** PASS. `Stop-Service` successfully terminates the launcher, router, and backend processes.
- **Port Cleanup:** PASS. Ports 9130 and 9120 were cleared.

## Technical Details
- **Service Name:** `LibrarianRunTimeNode`
- **Router Port:** 9130
- **Backend Launch Endpoint:** `POST /backend/select`
- **Target Backend Binary:** `G:\openwork\librarian-runtime-node\runtime\llama.cpp\llama-server.exe`
- **Test Profile:** `phi-4` (Port 9120)

## Verification Evidence
- **Starting HEAD:** `7bfb880`
- **Router PID:** 13812
- **Backend PID:** 17632 (Parent: 13812)
- **Backend Port:** 9120
- **Request Used:** `POST /backend/select {"profile": "phi-4"}`
- **Stop Result:** Service `Stopped`, no `python.exe` or `llama-server.exe` orphans remaining.

- **Stop Result:** Service `Stopped`, no `python.exe` or `llama-server.exe` orphans remaining.
