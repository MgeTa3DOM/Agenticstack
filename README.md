# Apophy Sovereign

**Zero-dependency sovereign AI infrastructure.**

100% on-prem | 100% encrypted | Universal hardware | $0 cloud costs

## What it solves

- **No vendor lock-in** - runs entirely on your hardware, no cloud APIs
- **Universal hardware** - NVIDIA, AMD, Intel, Apple Silicon, or pure CPU
- **E2E encrypted chat** (Fuel Protocol) - replaces WhatsApp/Slack/Teams
- **Single binary deployment** - one command, works everywhere
- **Sovereign data** - nothing leaves your network, ever

## Quick start

```bash
# Build and run
./scripts/deploy-sovereign.sh

# Or with Docker
docker compose up -d

# Or manually
cargo build --release
./target/release/apophy-sovereign start
```

## Architecture

```
apophy-sovereign (orchestrator)
├── apophy-crypto    - ChaCha20-Poly1305, X25519, Double Ratchet
├── apophy-fuel      - E2E encrypted P2P messaging
└── apophy-universal - Hardware auto-detection and optimization
```

## Commands

```bash
apophy-sovereign start      # Start the server
apophy-sovereign hardware   # Show detected hardware
apophy-sovereign keygen     # Generate encryption keys
apophy-sovereign health     # Check server health
```

## Configuration

Edit `config/sovereign.toml`. All settings have sane defaults.

## License

MIT