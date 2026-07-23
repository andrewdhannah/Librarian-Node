<#
.SYNOPSIS
    MCP Connection Healthcheck for The Librarian (Windows / PowerShell)

.DESCRIPTION
    Checks whether the Librarian server and MCP endpoint are reachable,
    whether JSON-RPC initialization succeeds, and whether expected MCP
    tools are available. Generates SessionStartup/MCP-STATUS.md for agent
    visibility.

    Windows-native equivalent of the macOS check-mcp-health.sh.
    Does NOT use curl, python3, or bash. Pure PowerShell 5.1+.

.PARAMETER ApiBase
    The Librarian API base URL. Defaults to http://127.0.0.1:3456.

.EXAMPLE
    .\scripts\check-mcp-health.ps1

.NOTES
    Exit codes:
      0 -- All OK
      1 -- Server unreachable
      2 -- MCP endpoint unreachable
      3 -- Tools missing
    Platform: Windows (PowerShell 5.1+)
    See also: scripts/mcp-bridge.ps1 for the stdio bridge.
#>

param(
    [string]$ApiBase = "http://127.0.0.1:3456"
)

$MCPEndpoint = "$ApiBase/mcp"
$HealthEndpoint = "$ApiBase/api/health"

# Expected MCP tool inventory (alphabetical)
$ExpectedTools = @(
    "librarian_checkin"
    "librarian_checkout"
    "librarian_checkpoint_work_order"
    "librarian_close_work_order"
    "librarian_diverge"
    "librarian_generate_doc"
    "librarian_get_item"
    "librarian_heartbeat"
    "librarian_plan_work"
    "librarian_resume_work_order"
    "librarian_search"
    "librarian_start_work_order"
)

# Status indicators
$script:apiOk = $false
$script:mcpOk = $false
$script:rpcOk = $false
$script:toolsOk = $false
$script:FoundTools = @()
$script:MissingTools = @()

function Log($msg) {
    Write-Host "[check-mcp] $msg"
}

function Write-Result($label, $ok) {
    if ($ok) { Write-Host "  OK: $label" -ForegroundColor Green }
    else     { Write-Host "  FAIL: $label" -ForegroundColor Red }
}

# ---------------------------------------------------------------------------
# Step 1: Check server health
# ---------------------------------------------------------------------------
Log "Checking server health at $HealthEndpoint..."
try {
    $health = Invoke-RestMethod -Uri $HealthEndpoint -Method Get -TimeoutSec 3 -ErrorAction Stop
    if ($health.status -eq "ok") {
        $script:apiOk = $true
        Log "Server health: OK"
    }
    else {
        Log "Server health: UNEXPECTED RESPONSE -- status=$($health.status)"
    }
}
catch {
    Log "Server health: UNREACHABLE -- $_"
}

# ---------------------------------------------------------------------------
# Step 2: Check MCP endpoint (send tools/list)
# ---------------------------------------------------------------------------
Log "Checking MCP endpoint at $MCPEndpoint..."

$toolsListPayload = '{"jsonrpc":"2.0","id":"healthcheck-1","method":"tools/list"}'

try {
    $mcpResponse = Invoke-RestMethod -Uri $MCPEndpoint -Method Post `
        -ContentType "application/json" -Body $toolsListPayload -TimeoutSec 5 -ErrorAction Stop
    $script:mcpOk = $true
    Log "MCP endpoint: reachable"

    # -----------------------------------------------------------------------
    # Step 3: Check JSON-RPC initialize
    # -----------------------------------------------------------------------
    $initPayload = @{
        jsonrpc = "2.0"
        id = "healthcheck-2"
        method = "initialize"
        params = @{
            protocolVersion = "2024-11-05"
            capabilities = @{}
            clientInfo = @{
                name = "mcp-healthcheck"
                version = "1.0.0"
            }
        }
    } | ConvertTo-Json

    try {
        $initResponse = Invoke-RestMethod -Uri $MCPEndpoint -Method Post `
            -ContentType "application/json" -Body $initPayload -TimeoutSec 5 -ErrorAction Stop
        if ($initResponse.result) {
            $script:rpcOk = $true
            Log "JSON-RPC initialize: success"
        }
        else {
            Log "JSON-RPC initialize: unexpected response"
        }
    }
    catch {
        Log "JSON-RPC initialize: not supported or error -- $_"
    }

    # -----------------------------------------------------------------------
    # Step 4: Parse tool list and compare against expected inventory
    # -----------------------------------------------------------------------
    Log "Checking tool inventory..."

    # Extract tool names from MCP response
    $toolNames = @()
    if ($mcpResponse.result.tools) {
        $toolNames = $mcpResponse.result.tools | ForEach-Object { $_.name }
    }
    elseif ($mcpResponse.result.content) {
        # Handle content-based response format
        $contentText = $mcpResponse.result.content[0].text
        try {
            $parsed = $contentText | ConvertFrom-Json
            if ($parsed.tools) {
                $toolNames = $parsed.tools | ForEach-Object { $_.name }
            }
        }
        catch {
            Log "Could not parse tool list from content text"
        }
    }

    if ($toolNames.Count -eq 0) {
        $script:toolsOk = $false
        Log "Could not parse tool list from response"
    }
    else {
        $script:FoundTools = $toolNames | Sort-Object
        $script:MissingTools = $ExpectedTools | Where-Object { $_ -notin $script:FoundTools }

        if ($script:MissingTools.Count -eq 0) {
            $script:toolsOk = $true
            Log "All $($ExpectedTools.Count) expected tools available"
        }
        else {
            $script:toolsOk = $false
            Log "Missing tools: $($script:MissingTools -join ', ')"
        }
    }
}
catch {
    Log "MCP endpoint: UNREACHABLE -- $_"
}

# ---------------------------------------------------------------------------
# Step 5: Validate MCP Permission Matrix
# ---------------------------------------------------------------------------
$permsOk = $null  # $null = not checked
$permsFile = Join-Path -Path $PSScriptRoot -ChildPath "..\config\mcp-permissions.json"
$permsFile = Resolve-Path $permsFile -ErrorAction SilentlyContinue

if ($permsFile) {
    try {
        $perms = Get-Content -Path $permsFile -Raw | ConvertFrom-Json
        $errors = @()

        # Check rules
        if ($perms.rules.agents_can_mark_verified -eq $true) {
            $errors += "agents_can_mark_verified must be false"
        }
        if ($perms.rules.human_verification_is_final -ne $true) {
            $errors += "human_verification_is_final must be true"
        }

        # Check no tool can verify
        foreach ($toolName in $perms.tools.PSObject.Properties.Name) {
            if ($perms.tools.$toolName.can_verify -eq $true) {
                $errors += "$toolName has can_verify=true"
            }
        }

        # Check all expected tools have entries
        foreach ($exp in $ExpectedTools) {
            if (-not ($perms.tools.PSObject.Properties.Name -contains $exp)) {
                $errors += "missing permission entry for $exp"
            }
        }

        if ($errors.Count -eq 0) {
            $permsOk = $true
            Log "Permission matrix: valid, all tools have entries, no tool can verify"
        }
        else {
            $permsOk = $false
            Log "Permission matrix: INVALID -- $($errors -join '; ')"
        }
    }
    catch {
        Log "Permission matrix: parse error -- $_"
    }
}
else {
    Log "Permission matrix: not found at $permsFile"
}

# ---------------------------------------------------------------------------
# Step 6: Generate status file
# ---------------------------------------------------------------------------
$sessionStartupDir = Join-Path -Path $PSScriptRoot -ChildPath "..\..\SessionStartup"
if (-not (Test-Path $sessionStartupDir)) {
    New-Item -ItemType Directory -Path $sessionStartupDir -Force | Out-Null
}

$statusFile = Join-Path -Path $sessionStartupDir -ChildPath "MCP-STATUS.md"
$generatedAt = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

$toolStatusLines = @()
foreach ($expected in $ExpectedTools) {
    $status = if ($expected -in $Script:FoundTools) { "available" } else { "MISSING" }
    $toolStatusLines += "| $expected | $status |"
}

$permsStatus = if ($permsOk -eq $true) { "valid" } elseif ($permsOk -eq $false) { "INVALID" } else { "not checked" }

$statusContent = @"
# MCP Status

Generated: $generatedAt

## Server

Librarian API: $(if ($script:apiOk) { "reachable" } else { "UNREACHABLE" })
Health endpoint: $HealthEndpoint
MCP endpoint: $MCPEndpoint

## MCP

JSON-RPC initialize: $(if ($script:rpcOk) { "success" } else { "FAIL" })
Tool list: $(if ($script:toolsOk) { "complete" } else { "INCOMPLETE" })

## Tools

| Tool | Status |
|---|---|
$($toolStatusLines -join "`n")

## Agent Instruction

If MCP is unavailable, do not claim Librarian custody is active.
If checkout/checkin tools are unavailable, do not generate canonical project documents.
If work-order tools are unavailable, use manual handoff and mark work as outside managed session.

## Summary

API health: $(if ($script:apiOk) { "OK" } else { "UNREACHABLE" })
MCP reachable: $(if ($script:mcpOk) { "OK" } else { "UNREACHABLE" })
RPC initialize: $(if ($script:rpcOk) { "OK" } else { "FAIL" })
Tools complete: $(if ($script:toolsOk) { "OK" } else { "INCOMPLETE" })
Tools found: $($script:FoundTools.Count)/$($ExpectedTools.Count)
Tools missing: $(if ($script:MissingTools.Count -eq 0) { "(none)" } else { $script:MissingTools -join ', ' })
Permission matrix: $permsStatus
"@

Set-Content -Path $statusFile -Value $statusContent
Log "Status written to $statusFile"

Write-Host "`n$('=' * 60)"
Get-Content -Path $statusFile
Write-Host "$('=' * 60)`n"

# ---------------------------------------------------------------------------
# Summary and exit
# ---------------------------------------------------------------------------
Log "Summary: API=$($script:apiOk) MCP=$($script:mcpOk) RPC=$($script:rpcOk) Tools=$($script:toolsOk) Perms=$permsStatus"

if (-not $script:apiOk) {
    Log "FATAL: Server unreachable. Start the Librarian server first."
    exit 1
}

if (-not $script:mcpOk) {
    Log "FATAL: MCP endpoint unreachable."
    exit 2
}

if (-not $script:toolsOk) {
    Log "WARNING: Some expected MCP tools are missing."
    exit 3
}

Log "All checks passed."
exit 0
