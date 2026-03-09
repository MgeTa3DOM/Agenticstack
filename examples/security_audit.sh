#!/usr/bin/env bash
# Example: Run a full sovereignty + security audit
# Usage: ./examples/security_audit.sh
set -e

BASE="http://localhost:8080/api/v1"

echo "=== Freedom Index ==="
curl -s "$BASE/freedom" | python3 -m json.tool

echo ""
echo "=== Fortress Security Scan ==="
curl -s "$BASE/fortress" | python3 -m json.tool

echo ""
echo "=== Governance Maturity Audit ==="
curl -s "$BASE/governance" | python3 -m json.tool

echo ""
echo "=== Attack Surface Analysis ==="
curl -s "$BASE/governance/surface" | python3 -m json.tool
