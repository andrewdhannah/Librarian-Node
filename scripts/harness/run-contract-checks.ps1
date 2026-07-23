<#
.SYNOPSIS
  Unified Windows Harness contract-test runner.

.DESCRIPTION
  Wraps existing repository test/check scripts into a single harness action
  with structured pass/fail output. Supports listing available checks,
  running selected checks by name, and running all safe read-only checks.

  Design principles:
  - READ-ONLY: Does NOT start or stop services. Does NOT run model workloads.
               Does NOT repair environment state.
  - DETERMINISTIC: Same environment state produces same output.
  - STRUCTURED: JSON output via -Json switch for automated consumption.
  - COMPREHENSIVE: Lists all known checks with classification so operators
                   can discover what exists and what requires service/model.

  Exit code 0 = ALL SELECTED CHECKS PASS
  Exit code 1 = ANY SELECTED CHECK FAILS, or unknown check name in -CheckName

.PARAMETER List
  List all registered checks with their classification, description, and
  skip reason (if applicable). Does NOT execute any checks. Exit 0.

.PARAMETER CheckName
  One or more check names (from the Name column of -List) to execute.
  Unknown names cause exit code 1.

.PARAMETER AllSafe
  Run all checks in the 'safe_readonly' category. These are read-only
  validations that do not require service, model, or admin.

.PARAMETER Json
  Emit structured JSON result object to stdout (deterministic format).
  When combined with -List, emits JSON array of check definitions instead
  of the human-readable table.

.PARAMETER RepoRoot
  Path to the librarian-runtime-node repo root.
  Auto-detected from the script location if omitted.

.PARAMETER Quiet
  Suppress human-readable output. When combined with -Json, only JSON
  is emitted.

.EXAMPLE
  .\scripts\harness\run-contract-checks.ps1 -List

.EXAMPLE
  .\scripts\harness\run-contract-checks.ps1 -AllSafe

.EXAMPLE
  .\scripts\harness\run-contract-checks.ps1 -CheckName pre-mutation-check,runtime-status

.EXAMPLE
  .\scripts\harness\run-contract-checks.ps1 -AllSafe -Json

.LINK
  scripts/harness/pre-mutation-check.ps1
  scripts/harness/postflight-check.ps1
  scripts/harness/new-sprint-receipt.ps1
  docs/planning/WIN-CUSTODY-SANDBOX-MODEL.md
#>

param(
  [switch]$List,
  [string[]]$CheckName = @(),
  [switch]$AllSafe,
  [switch]$Json,
  [string]$RepoRoot = "",
  [switch]$Quiet
)

# ============================================================================
# Constants
# ============================================================================

$Script:RunnerVersion = "1.0.0"
$Script:RunnerId = "WIN-HARNESS-CONTRACT-RUNNER-1"

# ============================================================================
# Helper functions
# ============================================================================

function Write-Message {
  param([string]$Text, [string]$Color = "White")
  if (-not $Quiet) {
    Write-Host $Text -ForegroundColor $Color
  }
}

function Write-ResultLine {
  param([string]$Mark, [string]$Name, [string]$Detail, [string]$Color)
  if ($Quiet) { return }
  $extra = if ($Detail) { " -- $Detail" } else { "" }
  Write-Host "  $mark $Name$extra" -ForegroundColor $Color
}

# ============================================================================
# Check registry
# ============================================================================
#
# Each entry:
#   Name         - Short unique identifier (used with -CheckName)
#   Display      - Human-readable heading
#   Description  - What the check validates
#   Command      - Command string to execute (relative or absolute)
#   Category     - Classification: safe_readonly | requires_service |
#                  requires_model | requires_admin | mutation_capable |
#                  requires_parameter | excluded
#   SkipReason   - Non-empty means the check is NOT auto-runnable;
#                  explains why.
#   WorkingDir   - Working directory for the command (defaults to RepoRoot)
#

function Get-CheckRegistry {
  param([string]$Root)
  $checks = @()

  # ---- safe_readonly: PowerShell harness/ops scripts ----

  $checks += @{
    Name = "pre-mutation-check"
    Display = "Pre-Mutation Custody Gate"
    Description = "Validates environment state: HEAD, working tree, branch, service, ports, orphans, disk space, origin sync, required files."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\harness\pre-mutation-check.ps1`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "runtime-status"
    Display = "Runtime Operator Status"
    Description = "Reports service state, port 9130 listener, process state, recent log files."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\operations\runtime-status.ps1`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "runtime-clean-check"
    Display = "Runtime Clean State Check"
    Description = "Verifies service Stopped/Manual, port 9130 free, no orphans, clean git trees."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\operations\runtime-clean-check.ps1`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "runtime-logs"
    Display = "Runtime Log File Inventory"
    Description = "Lists recent log files with sizes and timestamps. Read-only state snapshot."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\operations\runtime-logs.ps1`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "list-models"
    Display = "Model File Inventory"
    Description = "Scans models directory for .gguf files and returns JSON manifest."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\list-models.ps1`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "check-model-registry"
    Display = "Model Registry Validation"
    Description = "Validates model registry JSON: service entries, model card files, model files, SHA-256 hashes."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\check-model-registry.ps1`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "check-mcp-health"
    Display = "MCP Connection Health Check"
    Description = "Validates Librarian server health, MCP endpoint, JSON-RPC init, tool inventory, permission matrix."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\check-mcp-health.ps1`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "health-check"
    Display = "Backend Health Endpoint"
    Description = "Queries llama.cpp health endpoint and checks background job status."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\health-check.ps1`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  # ---- safe_readonly: runtime qualification dimension scripts ----

  $checks += @{
    Name = "test-runtime-artifact-identity"
    Display = "Qualification D1: Artifact Identity"
    Description = "Records binary path, SHA256, build timestamp, source HEADs, provenance match."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\test-runtime-artifact-identity.ps1`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "test-runtime-profiles"
    Display = "Qualification D5: Model Profile Envelope"
    Description = "Reads model-profiles.json, verifies evidence files, checks qualification status."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\test-runtime-profiles.ps1`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "test-runtime-cleanup"
    Display = "Qualification D7: Cleanup/Orphan Proof"
    Description = "Checks orphans and port state. Read-only pre/post snapshot."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\test-runtime-cleanup.ps1`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  # ---- safe_readonly: Python contract/validation test scripts ----

  $checks += @{
    Name = "test-operator-runbook"
    Display = "Operator Runbook Validation"
    Description = "Validates operator runbook for required sections, port map, binary refs, no auto-service instructions."
    Command = "python `"$Root\scripts\tests\test-win-runtime-operator-runbook.py`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "test-dry-run-readiness"
    Display = "Dry-Run Readiness Validation"
    Description = "Validates runbook-referenced files exist, no tracked gitignores, no auto-start/model instructions."
    Command = "python `"$Root\scripts\tests\test-win-runtime-dry-run-readiness.py`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "test-dry-run-gap-close"
    Display = "Dry-Run Gap Close Validation"
    Description = "Validates closure of GAP-001 (MCP permissions), GAP-002 (gitignore/runbook), GAP-003 (embedding port)."
    Command = "python `"$Root\scripts\tests\test-win-runtime-dry-run-gap-close.py`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "test-startup-custody-inventory"
    Display = "Startup Inventory Custody Validation"
    Description = "Validates startup custody inventory JSON: categories, risk classification, no production file changes."
    Command = "python `"$Root\scripts\tests\test-startup-files-custody-inventory.py`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "test-custody-normalization"
    Display = "Custody Normalization Regression"
    Description = "Validates no lowercase G:\openwork drift, no machine-local paths, no duplicate ports, path consistency."
    Command = "python `"$Root\scripts\tests\test-custody-normalization.py`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "test-context-route-contract"
    Display = "Context-Route Contract Validation"
    Description = "Validates context-route fixture JSON files against v0.1 contract: fields, enums, governance invariants."
    Command = "python `"$Root\scripts\tests\test-context-route-contract.py`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "test-advisory-stub"
    Display = "Advisory Stub Engine Tests"
    Description = "Tests offline advisory stub: all 9 workload types, invariants, forbidden actions, receipt rules."
    Command = "python `"$Root\scripts\tests\test-advisory-stub.py`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "test-router-context-design"
    Display = "Router Context Design Validation"
    Description = "Validates design artifact: 10 questions covered, fixture schemas, measured costs, advisory boundary."
    Command = "python `"$Root\scripts\tests\test-router-context-runtime-design.py`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "test-router-context-contract"
    Display = "Router Context Contract Tests"
    Description = "Contract validation: valid/invalid fixtures, enum validation, governance routes, receipt rules."
    Command = "python `"$Root\scripts\tests\test-router-context-runtime-contract.py`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "test-router-context-prototype"
    Display = "Router Context Prototype Tests"
    Description = "Tests prototype decision generator: 9 workload types, 13 invariants, 7 scenarios (A-G)."
    Command = "python `"$Root\scripts\tests\test-router-context-prototype.py`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  $checks += @{
    Name = "test-mcp-template-reconciliation"
    Display = "MCP Template Reconciliation"
    Description = "Validates MCP files exist, no macOS commands in Windows templates, platform separation, bridge structure."
    Command = "python `"$Root\scripts\tests\test-mcp-template-reconciliation.py`""
    Category = "safe_readonly"
    SkipReason = ""
  }

  # ---- requires_service: need router or backend running ----

  $checks += @{
    Name = "test-rust-router-endpoints"
    Display = "Rust Router Endpoint Tests"
    Description = "Tests all Rust router HTTP endpoints: /health, /profiles, /status, /v1/models, select, chat, stop."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\test-rust-router-endpoints.ps1`""
    Category = "requires_service"
    SkipReason = "requires service running (router/backend)"
  }

  $checks += @{
    Name = "test-runtime-contract"
    Display = "Runtime Contract Tests (Rust Router)"
    Description = "Verifies router contract: GET/POST status codes, JSON shapes, auth fields, error handling, secret leakage."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\test-runtime-contract.ps1`""
    Category = "requires_service"
    SkipReason = "requires service running (router/backend)"
  }

  $checks += @{
    Name = "test-runtime-lifecycle"
    Display = "Runtime Lifecycle Tests"
    Description = "Tests full lifecycle: start, profile select, chat, stop backend, stop router, port freed."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\test-runtime-lifecycle.ps1`""
    Category = "requires_service"
    SkipReason = "requires service running (router/backend)"
  }

  $checks += @{
    Name = "test-runtime-network-boundary"
    Display = "Network Boundary Tests"
    Description = "Tests default bind, LAN exposure, auth-required mode, secret logging with 3 router instances."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\test-runtime-network-boundary.ps1`""
    Category = "requires_service"
    SkipReason = "requires service running (router) and network test isolation"
  }

  $checks += @{
    Name = "test-runtime-limits"
    Display = "Runtime Limits Tests"
    Description = "Tests oversized body rejection (413), normal body acceptance, DefaultBodyLimit configuration."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\test-runtime-limits.ps1`""
    Category = "requires_service"
    SkipReason = "requires service running (router/backend)"
  }

  $checks += @{
    Name = "test-rust-router-parity"
    Display = "Rust/Python Router Parity Tests"
    Description = "Starts both routers on different ports, compares response status codes and JSON shapes."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\test-rust-router-parity.ps1`""
    Category = "requires_service"
    SkipReason = "requires both Rust and Python routers running"
  }

  $checks += @{
    Name = "test-network-boundary"
    Display = "Quick Network Boundary Test"
    Description = "Tests localhost binding, missing/valid token auth, oversized request rejection."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\test_network_boundary.ps1`""
    Category = "requires_service"
    SkipReason = "requires service running (router)"
  }

  $checks += @{
    Name = "run-router-contract-tests"
    Display = "Orchestrated Router Contract Suite"
    Description = "40+ step HTTP contract test: 7 endpoints, shape validation, refusal semantics, oversized body, auth phases."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\tests\run-router-contract-tests.ps1`""
    Category = "requires_service"
    SkipReason = "requires service running (router) for 40+ step contract suite"
  }

  # ---- requires_model: need inference-capable model backend ----

  $checks += @{
    Name = "test-model-fit"
    Display = "Single-Model Fit Test"
    Description = "Tests single model: start llama-server, wait for health, query /v1/models, run tiny chat, save evidence."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\test-model-fit.ps1`""
    Category = "requires_model"
    SkipReason = "requires model/runtime (llama-server + GGUF)"
  }

  $checks += @{
    Name = "test-reduced-offload-fit"
    Display = "Reduced Offload Fit Test"
    Description = "Tests OOM-prone profiles at descending ngl values to find stable GPU offload, restarts router per config."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\test-reduced-offload-fit.ps1`""
    Category = "requires_model"
    SkipReason = "requires model/runtime (inference workload)"
  }

  $checks += @{
    Name = "test-reconcile-fit"
    Display = "Model Fit Reconciliation"
    Description = "Reconciles phi-4 and qwen-coder fit using restart-per-config-change method with chat verification."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\test-reconcile-fit.ps1`""
    Category = "requires_model"
    SkipReason = "requires model/runtime (inference workload)"
  }

  $checks += @{
    Name = "run-model-fit-matrix"
    Display = "Model Fit Matrix Runner"
    Description = "Tests each model at each ngl level using llama-server. Records results to CSV and evidence JSON."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\run-model-fit-matrix.ps1`""
    Category = "requires_model"
    SkipReason = "requires model/runtime (full fit matrix over all profiles)"
  }

  # ---- requires_admin: need elevated privileges ----

  $checks += @{
    Name = "test-win-rust-service-swap"
    Display = "Rust Service Swap NSSM Test"
    Description = "Proves Rust router runs as NSSM service: start service, verify health, select backend, chat, stop."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\test-win-rust-service-swap.ps1`""
    Category = "requires_admin"
    SkipReason = "requires admin/operator (NSSM service control)"
  }

  $checks += @{
    Name = "run-win-rust-service-swap-proof"
    Display = "Rust Service Swap Proof Runner"
    Description = "Admin-elevated runner: verify prerequisites, ensure clean slate, run swap test, capture evidence."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\run-win-rust-service-swap-proof.ps1`""
    Category = "requires_admin"
    SkipReason = "requires admin/operator (NSSM service control)"
  }

  # ---- requires_parameter: need user-provided file paths ----

  $checks += @{
    Name = "verify-receipt"
    Display = "Integration Receipt Verifier"
    Description = "48-check receipt verifier. Requires receipt file path (-ReceiptPath). Not auto-runnable."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\verify-receipt.ps1`""
    Category = "requires_parameter"
    SkipReason = "requires manual -ReceiptPath parameter"
  }

  $checks += @{
    Name = "verify-runtime-qualification"
    Display = "Qualification Record Verifier"
    Description = "Validates qualification record JSON schema, required fields, hash formats. Needs file path."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\verify-runtime-qualification.ps1`""
    Category = "requires_parameter"
    SkipReason = "requires manual -QualificationPath parameter"
  }

  # ---- mutation_capable: excluded from contract runner ----

  $checks += @{
    Name = "run-runtime-qualification"
    Display = "Full Runtime Qualification"
    Description = "Rebuilds Rust router, captures artifact evidence, runs 7 dimension tests, emits qualification record."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\run-runtime-qualification.ps1`""
    Category = "mutation_capable"
    SkipReason = "mutation-capable (cargo rebuild) -- excluded from contract runner"
  }

  $checks += @{
    Name = "run-win-runtime-qualification"
    Display = "Windows Runtime Qualification Orchestrator"
    Description = "Orchestrates all 7 dimension tests, emits comprehensive qualification receipt."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\run-win-runtime-qualification.ps1`""
    Category = "mutation_capable"
    SkipReason = "mutation-capable (orchestrates build/test/emit) -- excluded from contract runner"
  }

  $checks += @{
    Name = "run-integration-proof-v2"
    Display = "Full Integration Proof v2"
    Description = "Full lifecycle proof: pre-checks, start router, test endpoints, chat, stop, cleanup, artifact collection."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\run-integration-proof-v2.ps1`""
    Category = "mutation_capable"
    SkipReason = "mutation-capable (starts services, modifies state) -- excluded from contract runner"
  }

  $checks += @{
    Name = "collect-inventory"
    Display = "Machine Inventory Collector"
    Description = "Collects hardware and network evidence: systeminfo, CPU, GPU, memory, ipconfig, listening ports."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\collect-inventory.ps1`""
    Category = "mutation_capable"
    SkipReason = "mutation-capable (writes snapshot files to fixtures/) -- excluded from contract runner"
  }

  $checks += @{
    Name = "measure-router-context"
    Display = "Router Context Measurement Harness"
    Description = "Collects Windows system info, runs Python measurements, runs I/O and JSON measurements, appends report."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\measurements\measure-router-context.ps1`""
    Category = "mutation_capable"
    SkipReason = "mutation-capable (appends results to JSON report) -- excluded from contract runner"
  }

  # ---- excluded: not validation scripts ----

  $checks += @{
    Name = "start-phi4-example"
    Display = "Phi-4 Example Startup (Template)"
    Description = "Example Phi-4 startup script. Must edit paths before use. Not a validation check."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\start-phi4.example.ps1`""
    Category = "excluded"
    SkipReason = "not a validation script (example template)"
  }

  $checks += @{
    Name = "mcp-bridge"
    Display = "MCP Stdio Bridge"
    Description = "MCP stdio bridge: reads JSON-RPC from stdin, POSTs to Librarian MCP. Proxy tool, not validation."
    Command = "powershell.exe -NoProfile -File `"$Root\scripts\mcp-bridge.ps1`""
    Category = "excluded"
    SkipReason = "not a validation script (MCP proxy bridge)"
  }

  return $checks
}

# ============================================================================
# Resolve repo root
# ============================================================================

if (-not $RepoRoot) {
  $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
  $RepoRoot = Split-Path -Parent (Split-Path -Parent $scriptDir)
}

$RepoRoot = (Resolve-Path $RepoRoot).Path

# ============================================================================
# Mode selection & validation
# ============================================================================

$modeCount = @($List -or $CheckName.Count -gt 0 -or $AllSafe)
if ($modeCount.Count -gt 1) {
  Write-Host "  FAIL Multiple modes specified. Use one of: -List, -CheckName, -AllSafe" -ForegroundColor "Red"
  exit 1
}

if (-not $List -and $CheckName.Count -eq 0 -and -not $AllSafe) {
  Write-Host "  FAIL No mode specified. Use -List to discover checks, -CheckName to run specific checks, or -AllSafe to run all safe checks." -ForegroundColor "Red"
  exit 1
}

# ============================================================================
# Load registry
# ============================================================================

$Registry = Get-CheckRegistry -Root $RepoRoot

# ============================================================================
# LIST mode
# ============================================================================

if ($List) {
  if ($Json) {
    $listOutput = $Registry | ForEach-Object {
      [PSCustomObject]@{
        name = $_.Name
        display = $_.Display
        description = $_.Description
        category = $_.Category
        skip_reason = if ($_.SkipReason) { $_.SkipReason } else { $null }
      }
    }
    $listJson = $listOutput | ConvertTo-Json
    Write-Host $listJson
    exit 0
  }

  Write-Message -Text "---" -Color "DarkGray"
  Write-Message -Text "Contract Check Registry -- $Script:RunnerId" -Color "Cyan"
  Write-Message -Text "Repo root: $RepoRoot" -Color "Gray"
  Write-Message -Text "---" -Color "DarkGray"

  $categories = @("safe_readonly", "requires_service", "requires_model", "requires_admin", "requires_parameter", "mutation_capable", "excluded")
  $catLabels = @{
    "safe_readonly" = "SAFE -- READ-ONLY"
    "requires_service" = "REQUIRES SERVICE"
    "requires_model" = "REQUIRES MODEL"
    "requires_admin" = "REQUIRES ADMIN"
    "requires_parameter" = "REQUIRES PARAMETER"
    "mutation_capable" = "MUTATION-CAPABLE"
    "excluded" = "EXCLUDED"
  }

  foreach ($cat in $categories) {
    $catChecks = $Registry | Where-Object { $_.Category -eq $cat }
    if ($catChecks.Count -eq 0) { continue }

    Write-Host "`n[$($catLabels[$cat])]" -ForegroundColor "Yellow"
    foreach ($chk in $catChecks | Sort-Object Name) {
      $skipInfo = if ($chk.SkipReason) { " [skip: $($chk.SkipReason)]" } else { "" }
      Write-Host "  $($chk.Name.PadRight(35)) $($chk.Display)$skipInfo" -ForegroundColor "Gray"
    }
  }

  Write-Message -Text "`n---" -Color "DarkGray"
  Write-Message -Text "Total: $($Registry.Count) checks registered" -Color "Gray"

  $safeCount = @($Registry | Where-Object { $_.Category -eq "safe_readonly" }).Count
  Write-Message -Text "Safe runnable: $safeCount  |  Skip-eligible: $($Registry.Count - $safeCount)" -Color "Gray"
  Write-Message -Text "Use -CheckName <name> to run specific checks, -AllSafe to run all safe checks." -Color "Gray"
  Write-Message -Text "---" -Color "DarkGray"

  exit 0
}

# ============================================================================
# Resolve which checks to run
# ============================================================================

$RunTargets = @()

if ($AllSafe) {
  $RunTargets = @($Registry | Where-Object { $_.Category -eq "safe_readonly" })
  Write-Message -Text "---" -Color "DarkGray"
  Write-Message -Text "Contract Runner: All Safe Checks -- $Script:RunnerId" -Color "Cyan"
  Write-Message -Text "Repo root: $RepoRoot" -Color "Gray"
  Write-Message -Text "---" -Color "DarkGray"
}

if ($CheckName.Count -gt 0) {
  Write-Message -Text "---" -Color "DarkGray"
  Write-Message -Text "Contract Runner: Named Checks -- $Script:RunnerId" -Color "Cyan"
  Write-Message -Text "Repo root: $RepoRoot" -Color "Gray"
  Write-Message -Text "---" -Color "DarkGray"

  $unknownNames = @()
  foreach ($name in $CheckName) {
    $match = $Registry | Where-Object { $_.Name -eq $name }
    if ($match) {
      $RunTargets += $match
    } else {
      $unknownNames += $name
    }
  }

  if ($unknownNames.Count -gt 0) {
    Write-Host "  FAIL Unknown check name(s): $($unknownNames -join ', ')" -ForegroundColor "Red"
    Write-Host "  Use -List to see all available check names." -ForegroundColor "Yellow"
    exit 1
  }
}

# ============================================================================
# Execute checks
# ============================================================================

$Script:CheckResults = @()
$Script:PassCount = 0
$Script:FailCount = 0
$Script:SkipCount = 0
$Script:ErrorCount = 0

$global:LASTEXITCODE = 0  # reset

foreach ($chk in $RunTargets) {
  # If check has a SkipReason and not AllSafe, skip it
  # (AllSafe only picks safe_readonly which have empty SkipReason, but be safe)
  if ($chk.SkipReason) {
    $Script:SkipCount++
    $result = @{
      "name" = $chk.Name
      "display" = $chk.Display
      "command" = $chk.Command
      "status" = "skip"
      "exit_code" = $null
      "duration_ms" = 0
      "category" = $chk.Category
      "skip_reason" = $chk.SkipReason
    }
    $Script:CheckResults += $result
    Write-ResultLine -Mark "SKIP" -Name $chk.Display -Detail $chk.SkipReason -Color "Yellow"
    continue
  }

  # Execute the check
  $startTime = Get-Date
  $stdoutLines = @()
  $stderrLines = @()
  $exitCode = -1

  try {
    # Split command into executable and arguments
    $cmdParts = $chk.Command -split ' ', 2
    $exe = $cmdParts[0]
    $args = if ($cmdParts.Count -gt 1) { $cmdParts[1] } else { "" }

    Write-ResultLine -Mark "RUN" -Name $chk.Display -Detail "" -Color "Cyan"

    # Run the command and capture output
    $output = & $exe $args 2>&1
    $exitCode = $global:LASTEXITCODE

    # Separate stdout from stderr
    foreach ($line in $output) {
      if ($line -is [System.Management.Automation.ErrorRecord]) {
        $stderrLines += $line.ToString()
      } else {
        $stdoutLines += $line.ToString()
      }
    }
  } catch {
    $exitCode = -1
    $stderrLines += $_.Exception.Message
  }

  $duration = (Get-Date) - $startTime
  $durationMs = [math]::Round($duration.TotalMilliseconds)

  if ($exitCode -eq -1) {
    # Execution error (command not found, etc.)
    $Script:ErrorCount++
    $status = "error"
    $detail = "Execution error: $($stderrLines -join '; ')"
    Write-ResultLine -Mark "ERROR" -Name $chk.Display -Detail $detail -Color "Red"
  } elseif ($exitCode -eq 0) {
    $Script:PassCount++
    $status = "pass"
    $detail = "$($stdoutLines.Count) output lines"
    Write-ResultLine -Mark "PASS" -Name $chk.Display -Detail "$($durationMs)ms" -Color "Green"
  } else {
    $Script:FailCount++
    $status = "fail"
    # Pick a meaningful error detail (last stderr line, or first stdout line with FAIL/Error)
    $failDetail = ""
    if ($stderrLines.Count -gt 0) {
      $failDetail = $stderrLines[-1]
    } elseif ($stdoutLines.Count -gt 0) {
      $failLines = $stdoutLines | Where-Object { $_ -match "FAIL|Error|exit code" }
      if ($failLines.Count -gt 0) { $failDetail = $failLines[-1] }
    }
    if (-not $failDetail) { $failDetail = "exit code $exitCode" }
    Write-ResultLine -Mark "FAIL" -Name $chk.Display -Detail "$($durationMs)ms -- $failDetail" -Color "Red"
  }

  $result = @{
    "name" = $chk.Name
    "display" = $chk.Display
    "command" = $chk.Command
    "status" = $status
    "exit_code" = $exitCode
    "duration_ms" = $durationMs
    "category" = $chk.Category
    "skip_reason" = $null
  }
  $Script:CheckResults += $result
}

# ============================================================================
# Summary
# ============================================================================

$totalRun = $Script:PassCount + $Script:FailCount + $Script:ErrorCount
$totalAll = $totalRun + $Script:SkipCount

Write-Message -Text "---" -Color "DarkGray"

$overallPass = ($Script:FailCount -eq 0 -and $Script:ErrorCount -eq 0)
$summaryText = "Run $totalRun | Passed $Script:PassCount | Failed $Script:FailCount | Errors $Script:ErrorCount | Skipped $Script:SkipCount"
Write-Message -Text "  OVERALL: $(if ($overallPass) { 'PASS' } else { 'FAIL' }) ($summaryText)" -Color $(if ($overallPass) { "Green" } else { "Red" })
Write-Message -Text "---" -Color "DarkGray"

# ============================================================================
# JSON output
# ============================================================================

if ($Json) {
  $jsonResult = @{
    "runner_id" = $Script:RunnerId
    "version" = $Script:RunnerVersion
    "timestamp" = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssK")
    "repo_root" = $RepoRoot
    "mode" = if ($AllSafe) { "all_safe" } elseif ($CheckName.Count -gt 0) { "named_checks" } else { "unknown" }
    "summary" = @{
      "total" = $totalAll
      "run" = $totalRun
      "passed" = $Script:PassCount
      "failed" = $Script:FailCount
      "errors" = $Script:ErrorCount
      "skipped" = $Script:SkipCount
      "overall" = if ($overallPass) { "PASS" } else { "FAIL" }
    }
    "checks" = $Script:CheckResults
  }

  $jsonText = $jsonResult | ConvertTo-Json -Depth 4
  if ($Quiet) {
    # Emit to stdout for programmatic capture
    $jsonText
  } else {
    # Styled display for interactive use
    Write-Host $jsonText -ForegroundColor "Gray"
  }
}

# ============================================================================
# Exit code
# ============================================================================

if ($overallPass) {
  exit 0
} else {
  exit 1
}
