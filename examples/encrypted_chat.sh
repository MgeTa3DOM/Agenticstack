#!/usr/bin/env bash
# Example: Send an E2E encrypted message via Signal Protocol (Double Ratchet)
# Usage: ./examples/encrypted_chat.sh
set -e

BASE="http://localhost:8080/api/v1"

echo "=== Peer Identity ==="
curl -s "$BASE/info" | python3 -m json.tool

echo ""
echo "=== Send E2E Encrypted Message ==="
curl -s -X POST "$BASE/chat/send" \
  -H "Content-Type: application/json" \
  -d '{
    "to": "self",
    "message": "Hello from the sovereign side. This message is encrypted with ChaCha20-Poly1305 + Double Ratchet.",
    "encrypt": true
  }' | python3 -m json.tool
