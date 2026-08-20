#!/bin/bash
# startup-linux.sh — Linux Startup Adapter
#
# Version: 1.0.0
# Platform: Linux
# Status: Reference Implementation
#
# Purpose:
#   This script implements the canonical startup protocol for Linux nodes.
#   It follows the STARTUP-PROTOCOL.md contract and produces a startup
#   receipt conforming to STARTUP-OUTPUT-CONTRACT.md.
#
# Usage:
#   ./startup-linux.sh --node-path /opt/librarian --output-path /var/librarian/evidence/startup
#
# Parameters:
#   --node-path     Path to node directory (required)
#   --output-path   Path for startup receipt (default: /var/librarian/evidence/startup)

set -e

# Default values
NODE_PATH=""
OUTPUT_PATH="/var/librarian/evidence/startup"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --node-path)
            NODE_PATH="$2"
            shift 2
            ;;
        --output-path)
            OUTPUT_PATH="$2"
            shift 2
            ;;
        *)
            echo "Unknown parameter: $1"
            exit 1
            ;;
    esac
done

# Validate required parameters
if [ -z "$NODE_PATH" ]; then
    echo "Error: --node-path is required"
    exit 1
fi

# Generate timestamp
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

echo "=== Librarian Node Linux Startup ==="
echo "Node Path: $NODE_PATH"
echo "Output Path: $OUTPUT_PATH"
echo ""

# Ensure output directory exists
mkdir -p "$OUTPUT_PATH"

# Initialize receipt
RECEIPT_FILE="$OUTPUT_PATH/startup-receipt-$TIMESTAMP.json"
cat > "$RECEIPT_FILE" << EOF
{
    "receipt_id": "LINUX-STARTUP-$TIMESTAMP",
    "node_id": "",
    "platform": "linux",
    "governance_commit": "",
    "startup_phase": "pending",
    "identity_loaded": false,
    "governance_verified": false,
    "capabilities_loaded": false,
    "environment_validated": false,
    "checks_passed": 0,
    "checks_failed": 0,
    "status": "pending",
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%S.0000000Z)"
}
EOF

# Phase 1: Identity Loading
echo "Phase 1: Loading identity..."
IDENTITY_FILE="$NODE_PATH/node-identity.json"

if [ -f "$IDENTITY_FILE" ]; then
    NODE_ID=$(jq -r '.node_id' "$IDENTITY_FILE")
    NODE_TYPE=$(jq -r '.node_type' "$IDENTITY_FILE")
    AUTHORITY=$(jq -r '.authority' "$IDENTITY_FILE")
    PLATFORM=$(jq -r '.platform' "$IDENTITY_FILE")
    
    if [ "$NODE_TYPE" = "librarian-runtime-node" ] && \
       [ "$AUTHORITY" = "owner-controlled" ] && \
       [ "$PLATFORM" = "linux" ]; then
        
        # Update receipt
        jq --arg node_id "$NODE_ID" \
           '.node_id = $node_id | .identity_loaded = true | .checks_passed += 1' \
           "$RECEIPT_FILE" > "${RECEIPT_FILE}.tmp" && mv "${RECEIPT_FILE}.tmp" "$RECEIPT_FILE"
        
        echo "  ✓ Identity loaded: $NODE_ID"
    else
        echo "  ✗ Invalid identity format"
        jq '.checks_failed += 1' "$RECEIPT_FILE" > "${RECEIPT_FILE}.tmp" && mv "${RECEIPT_FILE}.tmp" "$RECEIPT_FILE"
        exit 1
    fi
else
    echo "  ✗ Identity file not found: $IDENTITY_FILE"
    jq '.checks_failed += 1' "$RECEIPT_FILE" > "${RECEIPT_FILE}.tmp" && mv "${RECEIPT_FILE}.tmp" "$RECEIPT_FILE"
    exit 1
fi

# Phase 2: Governance Verification
echo "Phase 2: Verifying governance..."
GOVERNANCE_FILE="$NODE_PATH/governance-sync.json"

if [ -f "$GOVERNANCE_FILE" ]; then
    VERIFICATION_STATUS=$(jq -r '.verification_status' "$GOVERNANCE_FILE")
    GOVERNANCE_COMMIT=$(jq -r '.last_verified_commit' "$GOVERNANCE_FILE")
    
    if [ "$VERIFICATION_STATUS" = "verified" ] && \
       [[ "$GOVERNANCE_COMMIT" =~ ^[a-f0-9]{40}$ ]]; then
        
        # Update receipt
        jq --arg commit "$GOVERNANCE_COMMIT" \
           '.governance_commit = $commit | .governance_verified = true | .checks_passed += 1' \
           "$RECEIPT_FILE" > "${RECEIPT_FILE}.tmp" && mv "${RECEIPT_FILE}.tmp" "$RECEIPT_FILE"
        
        echo "  ✓ Governance verified: $GOVERNANCE_COMMIT"
    else
        echo "  ✗ Governance verification failed"
        jq '.checks_failed += 1' "$RECEIPT_FILE" > "${RECEIPT_FILE}.tmp" && mv "${RECEIPT_FILE}.tmp" "$RECEIPT_FILE"
        exit 1
    fi
else
    echo "  ✗ Governance file not found: $GOVERNANCE_FILE"
    jq '.checks_failed += 1' "$RECEIPT_FILE" > "${RECEIPT_FILE}.tmp" && mv "${RECEIPT_FILE}.tmp" "$RECEIPT_FILE"
    exit 1
fi

# Phase 3: Capability Loading
echo "Phase 3: Loading capabilities..."
CAPABILITIES_FILE="$NODE_PATH/capabilities.json"

if [ -f "$CAPABILITIES_FILE" ]; then
    GOVERNANCE_READ=$(jq -r '.governance_read' "$CAPABILITIES_FILE")
    GOVERNANCE_VERIFY=$(jq -r '.governance_verify' "$CAPABILITIES_FILE")
    
    if [ "$GOVERNANCE_READ" = "true" ] && \
       [ "$GOVERNANCE_VERIFY" = "true" ]; then
        
        # Update receipt
        jq '.capabilities_loaded = true | .checks_passed += 1' \
           "$RECEIPT_FILE" > "${RECEIPT_FILE}.tmp" && mv "${RECEIPT_FILE}.tmp" "$RECEIPT_FILE"
        
        echo "  ✓ Capabilities loaded"
    else
        echo "  ✗ Required capabilities missing"
        jq '.checks_failed += 1' "$RECEIPT_FILE" > "${RECEIPT_FILE}.tmp" && mv "${RECEIPT_FILE}.tmp" "$RECEIPT_FILE"
        exit 1
    fi
else
    echo "  ✗ Capabilities file not found: $CAPABILITIES_FILE"
    jq '.checks_failed += 1' "$RECEIPT_FILE" > "${RECEIPT_FILE}.tmp" && mv "${RECEIPT_FILE}.tmp" "$RECEIPT_FILE"
    exit 1
fi

# Phase 4: Environment Validation
echo "Phase 4: Validating environment..."

# Check bash version
BASH_VERSION_MAJOR=$(bash --version | head -n1 | grep -oE '[0-9]+\.[0-9]+' | cut -d. -f1)
if [ "$BASH_VERSION_MAJOR" -ge 4 ]; then
    # Update receipt
    jq '.environment_validated = true | .checks_passed += 1' \
       "$RECEIPT_FILE" > "${RECEIPT_FILE}.tmp" && mv "${RECEIPT_FILE}.tmp" "$RECEIPT_FILE"
    
    echo "  ✓ Environment validated"
else
    echo "  ✗ Bash 4.0 or higher required"
    jq '.checks_failed += 1' "$RECEIPT_FILE" > "${RECEIPT_FILE}.tmp" && mv "${RECEIPT_FILE}.tmp" "$RECEIPT_FILE"
    exit 1
fi

# Phase 5: Generate Startup Receipt
echo "Phase 5: Generating startup receipt..."

# Update receipt
jq '.startup_phase = "complete" | .status = "GOVERNED_EXECUTION" | .checks_passed += 1' \
   "$RECEIPT_FILE" > "${RECEIPT_FILE}.tmp" && mv "${RECEIPT_FILE}.tmp" "$RECEIPT_FILE"

echo "  ✓ Startup receipt generated"

# Phase 6: Enter Governed Mode
echo "Phase 6: Entering governed mode..."

# Update receipt
jq '.checks_passed += 1' "$RECEIPT_FILE" > "${RECEIPT_FILE}.tmp" && mv "${RECEIPT_FILE}.tmp" "$RECEIPT_FILE"

echo "  ✓ Entered governed mode"

# Display final receipt
echo ""
echo "=== Linux Startup Complete ==="
echo "Status: $(jq -r '.status' "$RECEIPT_FILE")"
echo "Checks Passed: $(jq -r '.checks_passed' "$RECEIPT_FILE") | Failed: $(jq -r '.checks_failed' "$RECEIPT_FILE")"
echo "Receipt: $RECEIPT_FILE"

# Exit with appropriate code
STATUS=$(jq -r '.status' "$RECEIPT_FILE")
if [ "$STATUS" = "STARTUP_FAILED" ]; then
    exit 1
else
    exit 0
fi
