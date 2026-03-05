#!/usr/bin/env bash
# Example: Configure Apophy Sovereign as MCP server for Claude Code
# Usage: ./examples/mcp_claude_setup.sh
set -e

echo "=== Apophy Sovereign MCP Setup for Claude Code ==="

# Show MCP config
echo ""
echo "Add this to ~/.claude/settings.json:"
echo ""
cat <<'JSON'
{
  "mcpServers": {
    "apophy-sovereign": {
      "command": "apophy-sovereign",
      "args": ["mcp-serve"],
      "env": {}
    }
  }
}
JSON

echo ""
echo "=== Available MCP Tools ==="
curl -s "http://localhost:8080/api/v1/mcp/tools" 2>/dev/null | python3 -m json.tool || echo "(Start server first: apophy-sovereign start)"

echo ""
echo "=== Test MCP Config ==="
curl -s "http://localhost:8080/api/v1/mcp/config" 2>/dev/null | python3 -m json.tool || echo "(Start server first: apophy-sovereign start)"
