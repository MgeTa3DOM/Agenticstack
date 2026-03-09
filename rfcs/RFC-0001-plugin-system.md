# RFC-0001: Plugin System Architecture

- **Status:** Draft
- **Author:** Apophy Core Team
- **Date:** 2026-03-05
- **Related Issues:** Roadmap Year 2

## Summary

Introduce a trait-based plugin system that allows community members to extend Apophy Sovereign with new domains, agents, capabilities, and integrations without modifying the core codebase.

## Motivation

The current 9 domains cover common business functions, but every organization has unique needs. A plugin system enables:

1. Community-driven domain expansion (Finance, Healthcare, Education, etc.)
2. Custom integrations (Slack, Discord, CRM connectors)
3. Specialized agent behaviors
4. Third-party AI model backends

Without a plugin system, all extensions require PRs to the core repo, creating a bottleneck.

## Detailed Design

See `specs/plugin-sdk.md` for the full trait specification.

Key decisions:
- **Native plugins** (dynamic libraries) for maximum performance
- **WASM plugins** (future) for sandboxed untrusted code
- **Plugin manifest** declares metadata, permissions, and dependencies
- **Plugin registry** (registry.apophy.dev) for discovery and distribution

## Alternatives Considered

1. **Script-based plugins (Lua/Python):** Rejected — adds runtime dependency, performance overhead
2. **gRPC microservice plugins:** Rejected — adds network complexity, violates single-binary principle
3. **Configuration-only extensions:** Rejected — too limited for real agent behaviors

## Sovereignty Check

- [x] Increases sovereignty — users can customize without depending on core team

## Open Questions

- Should plugins have access to the full SQLite database or a sandboxed subset?
- What is the minimum viable permission model for v1?
- How do we handle plugin conflicts (two plugins claiming the same domain)?
