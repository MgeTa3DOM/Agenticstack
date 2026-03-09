# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Reporting a Vulnerability

**Do NOT open a public issue for security vulnerabilities.**

Instead, please report security issues by emailing: **security@iagenticflow.org**

Or use [GitHub Security Advisories](https://github.com/MgeTa3DOM/Agenticstack/security/advisories/new) to privately report a vulnerability.

### What to include

- Description of the vulnerability
- Steps to reproduce
- Affected component (crate name)
- Potential impact
- Suggested fix (if any)

### Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 7 days
- **Fix or mitigation**: Within 30 days for critical issues

## Security Architecture

Apophy Sovereign is designed with security as a core principle:

- **ChaCha20-Poly1305** for symmetric encryption (no OpenSSL dependency)
- **X25519** for key exchange (Diffie-Hellman)
- **Ed25519** for digital signatures
- **Double Ratchet Protocol** for forward secrecy in P2P chat
- **Zeroize** for automatic secret memory clearing
- **No cloud dependencies** — your data never leaves your hardware
- **SQLite WAL mode** — embedded database, no network exposure
- **Fortress Scanner** — built-in security audit tool

## Dependency Auditing

We use `cargo audit` in CI to check for known vulnerabilities in dependencies. The security audit runs on every push and pull request.

```bash
# Run locally
cargo install cargo-audit
cargo audit
```
