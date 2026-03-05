#!/usr/bin/env bash
# Apophy Sovereign — Quick Demo
# Run: curl -sSL https://raw.githubusercontent.com/MgeTa3DOM/Agenticstack/main/scripts/demo.sh | bash
set -e

echo ""
echo "======================================"
echo "  APOPHY SOVEREIGN — DIVINE SYNARCHY"
echo "  3,000 AI Agents | 9 Domains | 1 Binary"
echo "======================================"
echo ""

# Check Rust
if ! command -v cargo &> /dev/null; then
    echo "[!] Rust not found. Install: https://rustup.rs"
    echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Build
echo "[1/5] Building Apophy Sovereign..."
cargo build --release -p apophy-sovereign 2>&1 | tail -1

BIN="./target/release/apophy-sovereign"

echo ""
echo "[2/5] Hardware Detection"
$BIN hardware
echo ""

echo "[3/5] Sovereignty Audit"
$BIN audit 2>/dev/null | head -20
echo ""

echo "[4/5] Divine Synarchy Fleet"
$BIN fleet
echo ""

echo "[5/5] Security Fortress"
$BIN fortress 2>/dev/null | head -10
echo ""

echo "======================================"
echo "  SOVEREIGN. AUTONOMOUS. YOURS."
echo ""
echo "  Start server:  $BIN start"
echo "  Full catalog:  $BIN fleet-catalog"
echo "  MCP config:    $BIN mcp"
echo "  Spawn agents:  curl -X POST localhost:8080/api/v1/fleet/spawn -d '{\"tier\":\"all\"}'"
echo "======================================"
