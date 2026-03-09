# Contributing to Apophy Sovereign

First off, thank you for considering contributing to Apophy Sovereign. Every contribution makes the sovereign AI movement stronger.

## Quick Start

```bash
git clone https://github.com/MgeTa3DOM/Agenticstack.git
cd Agenticstack
cargo build --workspace
cargo test --workspace
```

## How to Contribute

### 1. Find Something to Work On

- Check [open issues](https://github.com/MgeTa3DOM/Agenticstack/issues) for `good first issue` or `help wanted`
- See the "Areas where we need help" section in README.md
- Propose a new agent domain, specialist, or capability

### 2. Development Workflow

```bash
# Create a feature branch
git checkout -b feat/your-feature

# Make changes
# ...

# Verify everything
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo test --workspace

# Commit and push
git commit -m "feat: description of your change"
git push -u origin feat/your-feature
```

### 3. Submit a Pull Request

- Fill out the PR template
- Link any related issues
- Ensure CI passes (build, test, clippy, fmt)

## Code Standards

- **Zero warnings**: `cargo clippy -- -D warnings` must pass
- **Formatted**: `cargo fmt --all` before committing
- **No `unwrap()` in production code**: Use `?` or proper error handling
- **No hardcoded secrets**: All configuration via environment or config files
- **Document public APIs**: Add doc comments to public functions and types

## Architecture Guide

### Crate Structure

The workspace has 18 crates. Each crate owns a specific domain:

| Crate | Responsibility |
|-------|---------------|
| `apophy-sovereign` | Main binary: CLI + API server + fleet |
| `apophy-crypto` | All cryptographic operations |
| `apophy-commons` | Contribution token economy |
| `apophy-graph` | DAG workflow engine |
| `apophy-fortress` | Security scanning |
| See `Cargo.toml` | Full list |

### Adding a New Agent Specialist

1. Open `crates/apophy-sovereign/src/fleet/catalog.rs`
2. Add your specialist to the appropriate domain's `tactical_specialists()` method
3. Define prompts, expertise areas, and capabilities
4. Add tests
5. Update the README dropdown table

### Adding a New API Endpoint

1. Create handler in `crates/apophy-sovereign/src/routes/`
2. Register the route in `router()`
3. Add the endpoint to README's API table
4. Add integration test

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add new legal compliance agent
fix: resolve fleet spawn race condition
docs: update MCP integration guide
refactor: simplify crucible alignment logic
test: add fortress scanner edge cases
ci: add ARM64 release build
```

## Community

- GitHub Issues for bugs and features
- Pull Requests for code contributions
- Discussions for questions and ideas

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
