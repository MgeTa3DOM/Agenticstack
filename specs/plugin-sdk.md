# Apophy Plugin SDK Specification

> How to extend Apophy Sovereign with custom domains, agents, and integrations.

**Status:** Draft v0.1
**Date:** 2026-03-05

---

## Overview

The Plugin SDK allows anyone to:
1. **Add new domains** (e.g., Finance, Healthcare, Education)
2. **Add new specialists** to existing domains
3. **Add new capabilities** to agents
4. **Add new integrations** (Slack, Discord, APIs)
5. **Add new API endpoints** to the sovereign server

---

## Plugin Trait (Rust)

Every plugin implements the `ApophyPlugin` trait:

```rust
use apophy_sdk::prelude::*;

#[async_trait]
pub trait ApophyPlugin: Send + Sync {
    /// Plugin identity
    fn manifest(&self) -> PluginManifest;

    /// Called when the plugin is loaded
    async fn on_load(&mut self, ctx: &PluginContext) -> Result<()>;

    /// Called when the plugin is unloaded
    async fn on_unload(&mut self) -> Result<()>;

    /// Register agents contributed by this plugin
    fn agents(&self) -> Vec<AgentSpec> {
        vec![]
    }

    /// Register API routes contributed by this plugin
    fn routes(&self) -> Vec<RouteSpec> {
        vec![]
    }

    /// Register MCP tools contributed by this plugin
    fn mcp_tools(&self) -> Vec<McpToolSpec> {
        vec![]
    }

    /// Handle a task delegated to this plugin's agents
    async fn handle_task(&self, task: Task) -> Result<TaskResult>;
}
```

### Plugin Manifest

```rust
pub struct PluginManifest {
    pub name: String,           // "apophy-finance"
    pub version: String,        // "0.1.0"
    pub domain: String,         // "Finance & Accounting"
    pub description: String,
    pub author: String,
    pub license: String,        // "MIT"
    pub min_apophy_version: String, // "0.1.0"
    pub capabilities: Vec<String>,
}
```

### Agent Spec

```rust
pub struct AgentSpec {
    pub name: String,
    pub tier: AgentTier,        // General, Tactical, Operational
    pub domain: String,
    pub prompts: u32,
    pub expertise: Vec<String>,
    pub capabilities: Vec<Capability>,
}
```

---

## Example Plugin: Finance Domain

```rust
use apophy_sdk::prelude::*;

pub struct FinancePlugin;

#[async_trait]
impl ApophyPlugin for FinancePlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            name: "apophy-finance".into(),
            version: "0.1.0".into(),
            domain: "Finance & Accounting".into(),
            description: "Invoicing, bookkeeping, tax prep, financial forecasting".into(),
            author: "Community".into(),
            license: "MIT".into(),
            min_apophy_version: "0.2.0".into(),
            capabilities: vec![
                "Financial Analysis".into(),
                "Invoice Generation".into(),
                "Tax Calculation".into(),
            ],
        }
    }

    async fn on_load(&mut self, ctx: &PluginContext) -> Result<()> {
        tracing::info!("Finance domain loaded with {} agents", self.agents().len());
        Ok(())
    }

    async fn on_unload(&mut self) -> Result<()> {
        Ok(())
    }

    fn agents(&self) -> Vec<AgentSpec> {
        vec![
            AgentSpec {
                name: "CFO Strategist".into(),
                tier: AgentTier::General,
                domain: "Finance & Accounting".into(),
                prompts: 30,
                expertise: vec![
                    "Financial planning".into(),
                    "Budget allocation".into(),
                    "Cash flow management".into(),
                    "Investor reporting".into(),
                ],
                capabilities: vec![Capability::FinancialAnalysis, Capability::DocumentGeneration],
            },
            // ... more specialists
        ]
    }

    async fn handle_task(&self, task: Task) -> Result<TaskResult> {
        match task.action.as_str() {
            "generate_invoice" => self.generate_invoice(task).await,
            "financial_forecast" => self.financial_forecast(task).await,
            _ => Err(anyhow!("Unknown task: {}", task.action)),
        }
    }
}
```

---

## Plugin Lifecycle

```
1. Discovery    apophy scans plugins/ directory
2. Validation   Manifest checked against min version
3. Loading      on_load() called, agents registered
4. Running      Tasks delegated, routes active
5. Unloading    on_unload() called on shutdown
```

---

## Plugin Installation

```bash
# From registry (future)
apophy plugin install apophy-finance

# From Git
apophy plugin install https://github.com/user/apophy-finance.git

# From local path
apophy plugin install ./my-plugin

# List installed
apophy plugin list

# Remove
apophy plugin remove apophy-finance
```

---

## Security

- Plugins run in the same process (native speed)
- Future: WASM sandbox for untrusted plugins
- Plugins declare required permissions in manifest
- User approves permissions on install
- Plugins cannot access other plugins' data without declaration
