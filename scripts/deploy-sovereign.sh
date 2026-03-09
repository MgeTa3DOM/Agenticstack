#!/bin/bash
# Apophy Sovereign - Universal 1-click deployment
# Works on any hardware: NVIDIA/AMD/Intel/Apple/CPU

set -euo pipefail

BOLD='\033[1m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BOLD}Apophy Sovereign - Universal Deployment${NC}"
echo "=========================================="

# Detect OS
case "$OSTYPE" in
    linux-gnu*)  OS="linux" ;;
    darwin*)     OS="macos" ;;
    msys*|cygwin*) OS="windows" ;;
    *)
        echo -e "${RED}Unsupported OS: $OSTYPE${NC}"
        exit 1
        ;;
esac
echo -e "${GREEN}OS: $OS${NC}"

# Detect hardware
echo "Detecting hardware..."
HARDWARE="cpu"
if command -v nvidia-smi &> /dev/null; then
    HARDWARE="nvidia"
    GPU_NAME=$(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null | head -1 || echo "NVIDIA GPU")
    echo -e "${GREEN}NVIDIA GPU detected: $GPU_NAME${NC}"
elif command -v rocm-smi &> /dev/null; then
    HARDWARE="amd"
    echo -e "${GREEN}AMD GPU detected${NC}"
elif command -v xpu-smi &> /dev/null; then
    HARDWARE="intel"
    echo -e "${GREEN}Intel GPU detected${NC}"
elif [[ "$OS" == "macos" ]] && sysctl -n machdep.cpu.brand_string 2>/dev/null | grep -q "Apple"; then
    HARDWARE="apple"
    echo -e "${GREEN}Apple Silicon detected${NC}"
else
    echo -e "${YELLOW}CPU fallback mode${NC}"
fi

# Check Rust toolchain
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}Installing Rust toolchain...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Build
echo ""
echo -e "${BOLD}Building Apophy Sovereign...${NC}"
cargo build --release 2>&1

# Create data directories
mkdir -p data models

# Generate default config if not exists
if [ ! -f config/sovereign.toml ]; then
    echo -e "${YELLOW}Creating default config...${NC}"
    mkdir -p config
    cat > config/sovereign.toml <<'CONF'
[server]
host = "0.0.0.0"
port = 8080

[hardware]
auto_detect = true
max_memory_percent = 90

[chat]
enable_e2e = true
max_group_size = 10000
auto_destroy_days = 30
max_message_size_kb = 1024

[ai]
model_path = "models/default.gguf"
model_name = "qwen-3.5-coder"
max_tokens = 4096
temperature = 0.7
context_length = 4096

[security]
encryption = "chacha20poly1305"
key_rotation_days = 90
audit_log = true

[database]
path = "data/sovereign.db"
CONF
fi

echo ""
echo -e "${GREEN}${BOLD}Build complete!${NC}"
echo ""
echo "Usage:"
echo "  ./target/release/apophy-sovereign start          # Start server"
echo "  ./target/release/apophy-sovereign hardware       # Show hardware"
echo "  ./target/release/apophy-sovereign keygen          # Generate keys"
echo "  ./target/release/apophy-sovereign health          # Check health"
echo ""
echo -e "Hardware: ${BOLD}$HARDWARE${NC}"
echo -e "Config:   ${BOLD}config/sovereign.toml${NC}"
echo ""
