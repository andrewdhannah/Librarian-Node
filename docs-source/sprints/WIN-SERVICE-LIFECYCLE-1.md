# Sprint: WIN-SERVICE-LIFECYCLE-1
**Status:** COMPLETED
**Date:** 2026-06-19

## Objective
Verify the full lifecycle (Start -> Stop -> Restart) of the `LibrarianRunTimeNode` as a registered Windows Service to ensure no orphan processes (router or llama-server) remain.

## Final Result
- **Manual Lifecycle:** VERIFIED.
- **Service Lifecycle:** VERIFIED.
- **Orphan Process Check:** PASS. `Stop-Service` successfully terminates the launcher and router processes.
- **Backend Proof:** No backend orphan was present after service stop; backend launch-under-service was not exercised unless a backend-starting request was performed separately.
- **Restart Cycle:** PASS. Verified PID rotation (17564 $\rightarrow$ 8768) and port 9130 availability.

## Technical Details
- **Service Name:** `LibrarianRunTimeNode`
- **Router Port:** 9130
- **Backend Ports:** 9120-9124
- **NSSM Path:** `G:\openwork\librarian-runtime-node\runtime\bin\nssm.exe`
- **Launcher Script:** `G:\openwork\librarian-runtime-node\scripts\start-librarian-runtime-node.ps1`
- **Working Directory:** `G:\openwork\librarian-runtime-node`

## Verification Evidence
- **Start:** Service `Running`, Router PID 17564, Port 9130 Listening.
- **Restart:** Service `Running`, Router PID 8768, Port 9130 Listening.
- **Stop:** Service `Stopped`, No router/launcher processes remaining.
