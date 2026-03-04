//! # Toolshed — Meta-tool for Dynamic Tool Selection
//!
//! Inspired by Stripe's Toolshed: a central MCP-like server with ~500 tools.
//! Agents don't memorize tool APIs — they query the Toolshed for the right tool.
//!
//! ## Key Insight
//!
//! Agents are bad at choosing from 500 tools. They're good at choosing from 5.
//! The Toolshed narrows the search: given a task description, it returns the
//! top-K most relevant tools with their schemas and examples.
//!
//! ## Architecture
//!
//! ```text
//! Agent → "I need to read a file" → Toolshed.select(query)
//!   → Returns: [ReadFile { path: String }, ReadFileRange { path, start, end }]
//!   → Agent picks ReadFile and invokes it
//! ```
//!
//! ## Sovereign Twist
//!
//! All tools are local. No cloud APIs. Tool invocations are logged and
//! permission-gated via `apophy-governance` PermissionEnvelopes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// =============================================================================
// TOOL DEFINITION — what a tool looks like in the registry
// =============================================================================

/// A tool registered in the Toolshed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique tool ID
    pub id: Uuid,
    /// Tool name (e.g., "read_file", "run_tests", "query_db")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Category for grouping (e.g., "filesystem", "testing", "database")
    pub category: String,
    /// Tags for semantic search
    pub tags: Vec<String>,
    /// Input schema (JSON Schema as serde_json::Value)
    pub input_schema: serde_json::Value,
    /// Output schema
    pub output_schema: serde_json::Value,
    /// Example invocations (for few-shot prompting)
    pub examples: Vec<ToolExample>,
    /// Whether this tool has side effects
    pub has_side_effects: bool,
    /// Required permission level
    pub permission_level: PermissionLevel,
    /// Registered at
    pub registered_at: DateTime<Utc>,
    /// Usage count (for popularity-based ranking)
    pub usage_count: u64,
}

/// An example of how to use a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    pub description: String,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
}

/// Permission levels for tool access
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum PermissionLevel {
    /// Read-only, no side effects
    ReadOnly,
    /// Can modify local state
    ReadWrite,
    /// Can execute commands
    Execute,
    /// Requires explicit human approval
    Privileged,
}

impl PermissionLevel {
    pub fn label(&self) -> &'static str {
        match self {
            PermissionLevel::ReadOnly => "read-only",
            PermissionLevel::ReadWrite => "read-write",
            PermissionLevel::Execute => "execute",
            PermissionLevel::Privileged => "privileged",
        }
    }
}

// =============================================================================
// TOOL INVOCATION — what a tool call looks like
// =============================================================================

/// A tool invocation record (for audit trail)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocation {
    pub invocation_id: Uuid,
    pub tool_id: Uuid,
    pub tool_name: String,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub invoked_by: String, // Agent or blueprint step name
    pub invoked_at: DateTime<Utc>,
    pub duration_ms: i64,
    pub permission_level: PermissionLevel,
}

// =============================================================================
// TOOL HANDLER — trait for tool execution
// =============================================================================

/// Trait for tools that can be executed.
/// Implement this for each tool in your Toolshed.
pub trait ToolHandler: Send + Sync {
    /// Execute the tool with given input
    fn execute(&self, input: &serde_json::Value) -> Result<serde_json::Value, String>;

    /// Get the tool definition
    fn definition(&self) -> &ToolDefinition;
}

// =============================================================================
// TOOLSHED — the central registry and selection engine
// =============================================================================

/// The Toolshed — central registry of all available tools.
/// Agents query it to find the right tool for their task.
pub struct Toolshed {
    /// All registered tool definitions (for search/selection)
    definitions: Vec<ToolDefinition>,
    /// Tool handlers (for execution)
    handlers: HashMap<Uuid, Box<dyn ToolHandler>>,
    /// Invocation log (audit trail)
    invocations: Vec<ToolInvocation>,
    /// Maximum tools to return in a selection query
    pub max_selection_results: usize,
    /// Maximum permission level allowed without escalation
    pub max_permission_level: PermissionLevel,
}

impl Toolshed {
    pub fn new() -> Self {
        Self {
            definitions: Vec::new(),
            handlers: HashMap::new(),
            invocations: Vec::new(),
            max_selection_results: 5,
            max_permission_level: PermissionLevel::ReadWrite,
        }
    }

    /// Register a tool with its handler
    pub fn register(&mut self, handler: Box<dyn ToolHandler>) {
        let def = handler.definition().clone();
        self.definitions.push(def.clone());
        self.handlers.insert(def.id, handler);
    }

    /// Register a tool definition only (for search, without handler)
    pub fn register_definition(&mut self, def: ToolDefinition) {
        self.definitions.push(def);
    }

    /// Select the most relevant tools for a given task query.
    /// Returns up to `max_selection_results` tools, ranked by relevance.
    pub fn select(&self, query: &str, max_permission: Option<&PermissionLevel>) -> Vec<ToolSelection> {
        let max_perm = max_permission.unwrap_or(&self.max_permission_level);
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(f64, &ToolDefinition)> = self.definitions.iter()
            .filter(|d| &d.permission_level <= max_perm)
            .map(|d| {
                let score = self.relevance_score(d, &query_lower, &query_words);
                (score, d)
            })
            .filter(|(score, _)| *score > 0.0)
            .collect();

        // Sort by score descending, then by usage count (popularity)
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.1.usage_count.cmp(&a.1.usage_count))
        });

        scored.into_iter()
            .take(self.max_selection_results)
            .map(|(score, def)| ToolSelection {
                tool: def.clone(),
                relevance_score: score,
            })
            .collect()
    }

    /// Compute relevance score for a tool against a query
    fn relevance_score(&self, def: &ToolDefinition, query_lower: &str, query_words: &[&str]) -> f64 {
        let mut score = 0.0;

        // Name match (highest weight)
        let name_lower = def.name.to_lowercase();
        if query_lower.contains(&name_lower) || name_lower.contains(query_lower) {
            score += 10.0;
        }

        // Description match
        let desc_lower = def.description.to_lowercase();
        for word in query_words {
            if word.len() < 3 {
                continue; // Skip short words
            }
            if name_lower.contains(word) {
                score += 3.0;
            }
            if desc_lower.contains(word) {
                score += 1.0;
            }
        }

        // Tag match
        for tag in &def.tags {
            let tag_lower = tag.to_lowercase();
            for word in query_words {
                if tag_lower.contains(word) || word.contains(tag_lower.as_str()) {
                    score += 2.0;
                }
            }
        }

        // Category match
        let cat_lower = def.category.to_lowercase();
        for word in query_words {
            if cat_lower.contains(word) {
                score += 1.5;
            }
        }

        // Popularity bonus (logarithmic)
        if def.usage_count > 0 {
            score += (def.usage_count as f64).ln() * 0.1;
        }

        score
    }

    /// Execute a tool by ID with given input
    pub fn execute(&mut self, tool_id: Uuid, input: &serde_json::Value, invoked_by: &str) -> Result<serde_json::Value, String> {
        let handler = self.handlers.get(&tool_id)
            .ok_or_else(|| format!("Tool {} not found or has no handler", tool_id))?;

        let def = handler.definition().clone();

        // Permission check
        if def.permission_level > self.max_permission_level {
            return Err(format!(
                "Tool '{}' requires {} permission, but max allowed is {}",
                def.name,
                def.permission_level.label(),
                self.max_permission_level.label()
            ));
        }

        let started_at = Utc::now();
        let result = handler.execute(input);
        let completed_at = Utc::now();

        // Record invocation
        let invocation = ToolInvocation {
            invocation_id: Uuid::new_v4(),
            tool_id,
            tool_name: def.name.clone(),
            input: input.clone(),
            output: result.as_ref().ok().cloned(),
            error: result.as_ref().err().cloned(),
            invoked_by: invoked_by.into(),
            invoked_at: started_at,
            duration_ms: (completed_at - started_at).num_milliseconds(),
            permission_level: def.permission_level.clone(),
        };
        self.invocations.push(invocation);

        // Increment usage count
        if let Some(d) = self.definitions.iter_mut().find(|d| d.id == tool_id) {
            d.usage_count += 1;
        }

        result
    }

    /// Get the audit trail
    pub fn invocation_log(&self) -> &[ToolInvocation] {
        &self.invocations
    }

    /// Total registered tools
    pub fn tool_count(&self) -> usize {
        self.definitions.len()
    }

    /// Tools by category
    pub fn categories(&self) -> HashMap<String, usize> {
        let mut cats = HashMap::new();
        for def in &self.definitions {
            *cats.entry(def.category.clone()).or_insert(0) += 1;
        }
        cats
    }

    /// Summary for display
    pub fn summary(&self) -> ToolshedSummary {
        ToolshedSummary {
            total_tools: self.definitions.len(),
            categories: self.categories(),
            total_invocations: self.invocations.len(),
            tools_with_handlers: self.handlers.len(),
        }
    }
}

impl Default for Toolshed {
    fn default() -> Self {
        Self::new()
    }
}

/// A tool selection result (tool + relevance score)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSelection {
    pub tool: ToolDefinition,
    pub relevance_score: f64,
}

/// Summary of the Toolshed state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolshedSummary {
    pub total_tools: usize,
    pub categories: HashMap<String, usize>,
    pub total_invocations: usize,
    pub tools_with_handlers: usize,
}

// =============================================================================
// SOVEREIGN DEFAULTS — pre-built tool definitions for Apophy
// =============================================================================

/// Create a sovereign filesystem tool definition
pub fn filesystem_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            id: Uuid::new_v4(),
            name: "read_file".into(),
            description: "Read the contents of a file at the given path".into(),
            category: "filesystem".into(),
            tags: vec!["file".into(), "read".into(), "content".into()],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute file path" }
                },
                "required": ["path"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "content": { "type": "string" },
                    "size_bytes": { "type": "integer" }
                }
            }),
            examples: vec![
                ToolExample {
                    description: "Read a Rust source file".into(),
                    input: serde_json::json!({"path": "/src/main.rs"}),
                    output: serde_json::json!({"content": "fn main() {}", "size_bytes": 14}),
                },
            ],
            has_side_effects: false,
            permission_level: PermissionLevel::ReadOnly,
            registered_at: Utc::now(),
            usage_count: 0,
        },
        ToolDefinition {
            id: Uuid::new_v4(),
            name: "write_file".into(),
            description: "Write content to a file, creating or overwriting it".into(),
            category: "filesystem".into(),
            tags: vec!["file".into(), "write".into(), "create".into()],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["path", "content"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "bytes_written": { "type": "integer" }
                }
            }),
            examples: vec![],
            has_side_effects: true,
            permission_level: PermissionLevel::ReadWrite,
            registered_at: Utc::now(),
            usage_count: 0,
        },
        ToolDefinition {
            id: Uuid::new_v4(),
            name: "list_directory".into(),
            description: "List files and directories at the given path".into(),
            category: "filesystem".into(),
            tags: vec!["directory".into(), "list".into(), "files".into(), "ls".into()],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "recursive": { "type": "boolean", "default": false }
                },
                "required": ["path"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "entries": { "type": "array", "items": { "type": "string" } }
                }
            }),
            examples: vec![],
            has_side_effects: false,
            permission_level: PermissionLevel::ReadOnly,
            registered_at: Utc::now(),
            usage_count: 0,
        },
    ]
}

/// Create sovereign testing tool definitions
pub fn testing_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            id: Uuid::new_v4(),
            name: "run_tests".into(),
            description: "Run the test suite and return results".into(),
            category: "testing".into(),
            tags: vec!["test".into(), "cargo".into(), "verify".into(), "check".into()],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "filter": { "type": "string", "description": "Test name filter" },
                    "package": { "type": "string", "description": "Specific package to test" }
                }
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "total": { "type": "integer" },
                    "passed": { "type": "integer" },
                    "failed": { "type": "integer" }
                }
            }),
            examples: vec![],
            has_side_effects: false,
            permission_level: PermissionLevel::Execute,
            registered_at: Utc::now(),
            usage_count: 0,
        },
        ToolDefinition {
            id: Uuid::new_v4(),
            name: "run_clippy".into(),
            description: "Run Clippy linter and return warnings/errors".into(),
            category: "testing".into(),
            tags: vec!["lint".into(), "clippy".into(), "warnings".into(), "quality".into()],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "package": { "type": "string" }
                }
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "warnings": { "type": "integer" },
                    "errors": { "type": "integer" },
                    "messages": { "type": "array", "items": { "type": "string" } }
                }
            }),
            examples: vec![],
            has_side_effects: false,
            permission_level: PermissionLevel::Execute,
            registered_at: Utc::now(),
            usage_count: 0,
        },
    ]
}

/// Create sovereign search tool definitions
pub fn search_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            id: Uuid::new_v4(),
            name: "grep_search".into(),
            description: "Search file contents using regex pattern".into(),
            category: "search".into(),
            tags: vec!["grep".into(), "search".into(), "regex".into(), "find".into(), "content".into()],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string" },
                    "path": { "type": "string" },
                    "file_type": { "type": "string" }
                },
                "required": ["pattern"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "matches": { "type": "array" }
                }
            }),
            examples: vec![],
            has_side_effects: false,
            permission_level: PermissionLevel::ReadOnly,
            registered_at: Utc::now(),
            usage_count: 0,
        },
        ToolDefinition {
            id: Uuid::new_v4(),
            name: "glob_search".into(),
            description: "Find files matching a glob pattern".into(),
            category: "search".into(),
            tags: vec!["glob".into(), "find".into(), "files".into(), "pattern".into()],
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string" },
                    "path": { "type": "string" }
                },
                "required": ["pattern"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "files": { "type": "array", "items": { "type": "string" } }
                }
            }),
            examples: vec![],
            has_side_effects: false,
            permission_level: PermissionLevel::ReadOnly,
            registered_at: Utc::now(),
            usage_count: 0,
        },
    ]
}

/// Build a fully-loaded sovereign Toolshed with all default tools
pub fn sovereign_toolshed() -> Toolshed {
    let mut shed = Toolshed::new();
    for def in filesystem_tools() {
        shed.register_definition(def);
    }
    for def in testing_tools() {
        shed.register_definition(def);
    }
    for def in search_tools() {
        shed.register_definition(def);
    }
    shed
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Simple test tool handler
    struct EchoTool {
        def: ToolDefinition,
    }

    impl EchoTool {
        fn new(name: &str, category: &str, tags: Vec<&str>) -> Self {
            Self {
                def: ToolDefinition {
                    id: Uuid::new_v4(),
                    name: name.into(),
                    description: format!("Echo tool: {}", name),
                    category: category.into(),
                    tags: tags.into_iter().map(String::from).collect(),
                    input_schema: serde_json::json!({}),
                    output_schema: serde_json::json!({}),
                    examples: vec![],
                    has_side_effects: false,
                    permission_level: PermissionLevel::ReadOnly,
                    registered_at: Utc::now(),
                    usage_count: 0,
                },
            }
        }
    }

    impl ToolHandler for EchoTool {
        fn execute(&self, input: &serde_json::Value) -> Result<serde_json::Value, String> {
            Ok(serde_json::json!({
                "echo": input,
                "tool": self.def.name,
            }))
        }
        fn definition(&self) -> &ToolDefinition {
            &self.def
        }
    }

    #[test]
    fn test_toolshed_register_and_count() {
        let mut shed = Toolshed::new();
        shed.register(Box::new(EchoTool::new("tool_a", "cat1", vec!["tag1"])));
        shed.register(Box::new(EchoTool::new("tool_b", "cat2", vec!["tag2"])));

        assert_eq!(shed.tool_count(), 2);
        assert_eq!(shed.categories().len(), 2);
    }

    #[test]
    fn test_toolshed_select_by_name() {
        let mut shed = Toolshed::new();
        shed.register(Box::new(EchoTool::new("read_file", "fs", vec!["file", "read"])));
        shed.register(Box::new(EchoTool::new("write_file", "fs", vec!["file", "write"])));
        shed.register(Box::new(EchoTool::new("run_tests", "testing", vec!["test"])));

        let results = shed.select("I need to read a file", None);
        assert!(!results.is_empty());
        assert_eq!(results[0].tool.name, "read_file");
    }

    #[test]
    fn test_toolshed_select_by_tag() {
        let mut shed = Toolshed::new();
        shed.register(Box::new(EchoTool::new("tool_a", "cat", vec!["database", "query"])));
        shed.register(Box::new(EchoTool::new("tool_b", "cat", vec!["filesystem"])));

        let results = shed.select("query the database", None);
        assert!(!results.is_empty());
        assert_eq!(results[0].tool.name, "tool_a");
    }

    #[test]
    fn test_toolshed_select_respects_permission() {
        let mut shed = Toolshed::new();
        let mut privileged = EchoTool::new("dangerous", "admin", vec!["admin"]);
        privileged.def.permission_level = PermissionLevel::Privileged;
        shed.register(Box::new(privileged));
        shed.register(Box::new(EchoTool::new("safe_tool", "general", vec!["admin"])));

        // With default max permission (ReadWrite), privileged tool should be filtered
        let results = shed.select("admin tool", None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tool.name, "safe_tool");

        // With explicit Privileged permission, it should appear
        let results = shed.select("admin tool", Some(&PermissionLevel::Privileged));
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_toolshed_execute() {
        let mut shed = Toolshed::new();
        let tool = EchoTool::new("echo", "general", vec![]);
        let tool_id = tool.def.id;
        shed.register(Box::new(tool));

        let input = serde_json::json!({"message": "hello"});
        let result = shed.execute(tool_id, &input, "test_agent").unwrap();

        assert_eq!(result["tool"], "echo");
        assert_eq!(result["echo"]["message"], "hello");
        assert_eq!(shed.invocation_log().len(), 1);
    }

    #[test]
    fn test_toolshed_execute_increments_usage() {
        let mut shed = Toolshed::new();
        let tool = EchoTool::new("counter", "general", vec![]);
        let tool_id = tool.def.id;
        shed.register(Box::new(tool));

        let input = serde_json::json!({});
        shed.execute(tool_id, &input, "agent").unwrap();
        shed.execute(tool_id, &input, "agent").unwrap();

        let def = shed.definitions.iter().find(|d| d.id == tool_id).unwrap();
        assert_eq!(def.usage_count, 2);
    }

    #[test]
    fn test_toolshed_execute_nonexistent() {
        let mut shed = Toolshed::new();
        let result = shed.execute(Uuid::new_v4(), &serde_json::json!({}), "agent");
        assert!(result.is_err());
    }

    #[test]
    fn test_sovereign_toolshed_defaults() {
        let shed = sovereign_toolshed();
        assert_eq!(shed.tool_count(), 7); // 3 fs + 2 testing + 2 search
        assert!(shed.categories().contains_key("filesystem"));
        assert!(shed.categories().contains_key("testing"));
        assert!(shed.categories().contains_key("search"));
    }

    #[test]
    fn test_sovereign_toolshed_select_file_operations() {
        let shed = sovereign_toolshed();

        let results = shed.select("read file content", None);
        assert!(!results.is_empty());
        // read_file should be the top result
        assert!(results[0].tool.name.contains("read") || results[0].tool.name.contains("file"));
    }

    #[test]
    fn test_sovereign_toolshed_select_testing() {
        let shed = sovereign_toolshed();

        let results = shed.select("run the tests", Some(&PermissionLevel::Execute));
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.tool.name.contains("test")));
    }

    #[test]
    fn test_toolshed_summary() {
        let shed = sovereign_toolshed();
        let summary = shed.summary();
        assert_eq!(summary.total_tools, 7);
        assert_eq!(summary.total_invocations, 0);
        assert_eq!(summary.tools_with_handlers, 0); // Definitions only
    }

    #[test]
    fn test_permission_level_ordering() {
        assert!(PermissionLevel::ReadOnly < PermissionLevel::ReadWrite);
        assert!(PermissionLevel::ReadWrite < PermissionLevel::Execute);
        assert!(PermissionLevel::Execute < PermissionLevel::Privileged);
    }

    #[test]
    fn test_toolshed_max_results() {
        let mut shed = Toolshed::new();
        shed.max_selection_results = 2;

        for i in 0..10 {
            shed.register(Box::new(EchoTool::new(
                &format!("file_tool_{}", i),
                "filesystem",
                vec!["file"],
            )));
        }

        let results = shed.select("file tool", None);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_tool_invocation_audit_trail() {
        let mut shed = Toolshed::new();
        let tool = EchoTool::new("audited", "general", vec![]);
        let tool_id = tool.def.id;
        shed.register(Box::new(tool));

        shed.execute(tool_id, &serde_json::json!({"key": "value"}), "worker_1").unwrap();

        let log = shed.invocation_log();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].tool_name, "audited");
        assert_eq!(log[0].invoked_by, "worker_1");
        assert!(log[0].error.is_none());
        assert!(log[0].output.is_some());
    }

    #[test]
    fn test_select_empty_query() {
        let shed = sovereign_toolshed();
        let results = shed.select("", None);
        // Empty query may still match via name contains empty string
        // Just verify it doesn't panic and returns bounded results
        assert!(results.len() <= shed.max_selection_results);
    }

    #[test]
    fn test_definition_only_registration() {
        let mut shed = Toolshed::new();
        let def = ToolDefinition {
            id: Uuid::new_v4(),
            name: "phantom_tool".into(),
            description: "A tool with no handler".into(),
            category: "ghost".into(),
            tags: vec![],
            input_schema: serde_json::json!({}),
            output_schema: serde_json::json!({}),
            examples: vec![],
            has_side_effects: false,
            permission_level: PermissionLevel::ReadOnly,
            registered_at: Utc::now(),
            usage_count: 0,
        };
        let tool_id = def.id;
        shed.register_definition(def);

        assert_eq!(shed.tool_count(), 1);
        // Execute should fail (no handler)
        let result = shed.execute(tool_id, &serde_json::json!({}), "agent");
        assert!(result.is_err());
    }
}
