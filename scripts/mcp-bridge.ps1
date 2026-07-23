<#
.SYNOPSIS
    MCP stdio bridge for The Librarian (Windows / PowerShell)

.DESCRIPTION
    Reads JSON-RPC lines from stdin and POSTs them to the local Librarian
    MCP endpoint. This allows OpenWork to connect via stdio (type: local)
    instead of HTTP/SSE (type: remote).

    Windows-native equivalent of the macOS mcp-bridge.sh.

.PARAMETER McpUrl
    The Librarian MCP endpoint URL. Defaults to the LIBRARIAN_MCP_URL
    environment variable, or http://127.0.0.1:3456/mcp.

.EXAMPLE
    # Run standalone (pipe JSON-RPC lines to stdin):
    echo '{"jsonrpc":"2.0","id":"1","method":"tools/list"}' | .\scripts\mcp-bridge.ps1

    # Configure via env var:
    $env:LIBRARIAN_MCP_URL = "http://127.0.0.1:3456/mcp"
    .\scripts\mcp-bridge.ps1

.NOTES
    Platform: Windows (PowerShell 5.1+)
    See also: scripts/check-mcp-health.ps1 for health verification.
#>

$MCP_URL = if ($env:LIBRARIAN_MCP_URL) { $env:LIBRARIAN_MCP_URL } else { "http://127.0.0.1:3456/mcp" }

try {
    # Read JSON-RPC messages line by line from stdin
    while ($line = [Console]::In.ReadLine()) {
        if (-not [string]::IsNullOrWhiteSpace($line)) {
            try {
                $response = Invoke-RestMethod -Uri $MCP_URL -Method Post `
                    -ContentType "application/json" -Body $line -ErrorAction Stop
                if ($response) {
                    $response | ConvertTo-Json -Compress
                }
            }
            catch {
                Write-Warning "MCP bridge error: $_"
                Write-Output "{`"jsonrpc`":`"2.0`",`"id`":null,`"error`":{`"code`":-32000,`"message`":`"Bridge error: $($_.Exception.Message)`"}}"
            }
        }
    }
}
catch {
    # stdin closed or pipe ended — normal exit
    exit 0
}
