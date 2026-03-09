#!/usr/bin/env bash
# Example: Spawn the entire Divine Synarchy fleet and query it
# Usage: ./examples/spawn_fleet.sh
set -e

BASE="http://localhost:8080/api/v1"

echo "=== Spawning 3,000 Agent Fleet ==="
curl -s -X POST "$BASE/fleet/spawn" \
  -H "Content-Type: application/json" \
  -d '{"tier": "all"}' | python3 -m json.tool

echo ""
echo "=== Fleet Summary ==="
curl -s "$BASE/fleet/summary" | python3 -m json.tool

echo ""
echo "=== Domain Breakdown ==="
curl -s "$BASE/fleet/domains" | python3 -m json.tool
