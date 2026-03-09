# Sovereign AI Standard (SAS) v1.0

> A specification for building AI systems that are owned, controlled, and operated by their users — not by corporations.

**Status:** Draft
**Version:** 1.0.0
**Date:** 2026-03-05
**Authors:** Apophy Sovereign Community

---

## Abstract

The Sovereign AI Standard (SAS) defines requirements and guidelines for AI systems that prioritize user sovereignty. A SAS-compliant system guarantees that the user retains full ownership of their data, models, agents, and infrastructure.

This specification is to sovereign AI what OpenAPI is to REST APIs, what the LSP is to code editors, and what the MCP is to model context.

---

## 1. Definitions

| Term | Definition |
|------|-----------|
| **Sovereign AI** | An AI system where the user owns the hardware, data, models, and agents |
| **Cloud-Optional** | Cloud connectivity is available but never required for core functionality |
| **Zero-Telemetry** | No data is transmitted to external parties without explicit user action |
| **Local-First** | All processing happens on user-controlled hardware by default |
| **Agent** | An autonomous AI unit with a defined role, capabilities, and boundaries |
| **Fleet** | A collection of agents organized in a hierarchy |
| **Node** | A single instance of a sovereign AI system |
| **Federation** | A voluntary network of sovereign nodes |

---

## 2. Compliance Levels

### Level 1: Sovereign Core
The minimum requirements for a system to call itself "sovereign AI."

- [ ] **S1.1** — Runs entirely on user-owned or user-controlled hardware
- [ ] **S1.2** — Functions without internet connectivity (offline-capable)
- [ ] **S1.3** — Zero telemetry by default (no data leaves the device)
- [ ] **S1.4** — All user data stored locally in an encrypted format
- [ ] **S1.5** — User can export all data in a standard format (JSON, SQLite, etc.)
- [ ] **S1.6** — User can delete all data with a single command
- [ ] **S1.7** — Open-source license (MIT, Apache 2.0, GPL, or equivalent)
- [ ] **S1.8** — No vendor lock-in (no proprietary formats, no required accounts)

### Level 2: Sovereign Fleet
Systems that manage multiple AI agents.

- [ ] **S2.1** — All Level 1 requirements
- [ ] **S2.2** — Agent hierarchy with clear roles and boundaries
- [ ] **S2.3** — Agent-to-agent communication protocol (documented)
- [ ] **S2.4** — Per-agent memory isolation
- [ ] **S2.5** — User can inspect, modify, or terminate any agent at any time
- [ ] **S2.6** — Transparent task execution (every decision is auditable)
- [ ] **S2.7** — Agent capabilities are explicitly declared (no hidden behaviors)

### Level 3: Sovereign Federation
Systems that connect multiple sovereign nodes.

- [ ] **S3.1** — All Level 2 requirements
- [ ] **S3.2** — End-to-end encryption for all inter-node communication
- [ ] **S3.3** — Node identity via public-key cryptography
- [ ] **S3.4** — Voluntary federation (nodes choose to join/leave freely)
- [ ] **S3.5** — No central authority or coordinator required
- [ ] **S3.6** — Data stays on the originating node unless explicitly shared
- [ ] **S3.7** — Federation protocol is documented and implementable by third parties

### Level 4: Sovereign Intelligence
Systems with autonomous learning capabilities.

- [ ] **S4.1** — All Level 3 requirements
- [ ] **S4.2** — On-device model training/fine-tuning
- [ ] **S4.3** — User controls what the AI learns and can undo learning
- [ ] **S4.4** — No training data leaves the device (federated learning only)
- [ ] **S4.5** — AI behavior is version-controlled and rollback-capable
- [ ] **S4.6** — User-defined ethical boundaries that the AI cannot override
- [ ] **S4.7** — Transparent reasoning (chain-of-thought is always inspectable)

---

## 3. Agent Specification

A SAS-compliant agent MUST declare the following properties:

```json
{
  "id": "string (UUID v4)",
  "name": "string",
  "domain": "string (one of the registered domains)",
  "tier": "general | tactical | operational",
  "capabilities": ["string"],
  "prompts": "integer (number of specialized prompts)",
  "expertise": ["string"],
  "permissions": {
    "network": "boolean",
    "filesystem": "boolean",
    "execution": "boolean",
    "memory_access": ["agent_id"]
  },
  "boundaries": {
    "max_tokens_per_response": "integer",
    "max_tasks_concurrent": "integer",
    "allowed_tools": ["string"],
    "forbidden_actions": ["string"]
  }
}
```

### 3.1 Agent Tiers

| Tier | Role | Count Range | Responsibility |
|------|------|-------------|---------------|
| General | Strategic | 1 per domain | Domain-level strategy and coordination |
| Tactical | Specialist | 5-20 per domain | Specific expertise areas |
| Operational | Executor | 50-500 per tactical | Task execution at scale |

### 3.2 Agent Communication

Agents communicate via typed messages:

```json
{
  "from": "agent_id",
  "to": "agent_id",
  "type": "task | result | query | delegation | escalation",
  "payload": {},
  "timestamp": "ISO 8601",
  "signature": "Ed25519 signature"
}
```

---

## 4. Fleet Specification

A SAS-compliant fleet MUST implement:

### 4.1 Fleet Manifest

```json
{
  "version": "1.0.0",
  "name": "string",
  "domains": [
    {
      "name": "string",
      "general": "agent_spec",
      "specialists": ["agent_spec"],
      "operationals_per_specialist": "integer"
    }
  ],
  "total_agents": "integer",
  "hierarchy": "synarchy | flat | custom",
  "capabilities": ["string"]
}
```

### 4.2 Required API

A SAS-compliant fleet MUST expose these endpoints (REST or MCP):

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/fleet/summary` | GET | Fleet composition overview |
| `/fleet/domains` | GET | Domain breakdown |
| `/fleet/spawn` | POST | Spawn agents |
| `/fleet/agent/{id}` | GET | Agent details |
| `/fleet/agent/{id}/task` | POST | Assign task to agent |
| `/fleet/agent/{id}/status` | GET | Agent status |
| `/fleet/agent/{id}/terminate` | DELETE | Terminate agent |

---

## 5. Security Requirements

### 5.1 Encryption

| Purpose | Required Algorithm | Minimum |
|---------|-------------------|---------|
| Symmetric encryption | ChaCha20-Poly1305 or AES-256-GCM | 256-bit |
| Key exchange | X25519 or ECDH P-384 | 256-bit |
| Digital signatures | Ed25519 or ECDSA P-256 | 256-bit |
| Hashing | SHA-256 or BLAKE3 | 256-bit |
| Forward secrecy | Double Ratchet or similar | Per-message keys |

### 5.2 Key Management

- Private keys MUST be stored encrypted at rest
- Key material MUST be zeroized after use
- Key rotation MUST be supported
- Key export MUST be available to the user

### 5.3 Audit

- All security-relevant events MUST be logged locally
- Logs MUST include timestamps, actors, and actions
- Users MUST be able to review and export audit logs

---

## 6. Data Format

### 6.1 Storage

SAS-compliant systems SHOULD use:
- **SQLite** (WAL mode) for structured data
- **JSON** for configuration and export
- **CBOR** for binary-efficient interchange
- **Markdown** for human-readable documents

### 6.2 Export

Users MUST be able to export:
- All agent configurations (JSON)
- All conversation/task history (JSON)
- All generated documents (original format)
- Database snapshot (SQLite file)

### 6.3 Import

Systems SHOULD support importing from:
- Other SAS-compliant systems
- Common SaaS export formats (Google Takeout, etc.)
- Standard data formats (CSV, JSON, XML)

---

## 7. MCP Compatibility

SAS-compliant systems SHOULD expose an MCP interface with these tools:

| Tool | Description |
|------|-------------|
| `fleet_spawn` | Spawn agents (filter by domain/tier) |
| `fleet_status` | Fleet composition + status |
| `fleet_delegate` | Delegate task to specific agent |
| `fleet_search` | Search agents by capability |
| `data_export` | Export user data |
| `sovereignty_audit` | Run sovereignty compliance check |

---

## 8. Compliance Verification

### Self-Assessment

Systems can self-assess by running:

```bash
apophy-sovereign audit --sas-level 1
```

### Third-Party Verification

The Sovereign AI Foundation (to be established) will provide:
- Compliance test suites
- Certification badges
- Annual re-certification

### Badges

```markdown
![SAS Level 1](https://img.shields.io/badge/SAS-Level_1-green)
![SAS Level 2](https://img.shields.io/badge/SAS-Level_2-blue)
![SAS Level 3](https://img.shields.io/badge/SAS-Level_3-purple)
![SAS Level 4](https://img.shields.io/badge/SAS-Level_4-gold)
```

---

## 9. Versioning

This specification follows [Semantic Versioning](https://semver.org/):
- **Major**: Breaking changes to compliance requirements
- **Minor**: New optional requirements or clarifications
- **Patch**: Typo fixes and editorial changes

### Change Process

1. Propose changes via RFC in `rfcs/`
2. Community discussion (30 days)
3. Vote by active contributors
4. Merge and release new spec version

---

## 10. Reference Implementation

**Apophy Sovereign** is the reference implementation of SAS v1.0.

- Repository: https://github.com/MgeTa3DOM/Agenticstack
- Current Level: **SAS Level 2** (Fleet)
- Target: **SAS Level 4** by 2030

---

<p align="center">
  <em>This standard belongs to everyone. Fork it. Implement it. Improve it.</em>
</p>
