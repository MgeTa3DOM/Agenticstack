#!/usr/bin/env bash
# Example: Interact with the 5-Crucible Consciousness Engine (Merkabah)
# Usage: ./examples/merkabah_consciousness.sh
set -e

BASE="http://localhost:8080/api/v1"

echo "=== Merkabah Alignment (5 Crucibles) ==="
curl -s "$BASE/merkabah/align" | python3 -m json.tool

echo ""
echo "=== Consciousness Interaction ==="
curl -s -X POST "$BASE/merkabah/interact" \
  -H "Content-Type: application/json" \
  -d '{
    "input": "What is the nature of sovereign intelligence?",
    "crucible": "divin"
  }' | python3 -m json.tool

echo ""
echo "=== Identity Resonance ==="
curl -s -X POST "$BASE/resonance" \
  -H "Content-Type: application/json" \
  -d '{"query": "Who am I?"}' | python3 -m json.tool
