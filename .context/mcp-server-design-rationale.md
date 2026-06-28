---
id: ctx-3121d7
created: 2026-06-26T17:02:05.484530453Z
status: current
concerns:
- mcp-server-design-rationale
scope:
  paths: []
  components:
  - ctx-cli
superseded_by: []
---
### mcp-server-design-rationale [r3]

The decision to build the MCP server natively in Rust rather than as a Python wrapper was made for the following reasons:

1. Single binary — cargo install ctx gives you both the CLI and the MCP server. No Python, uv, venv, or separate install step.
2. Correctness — tool handlers call the same internal functions as the CLI. A Python wrapper would shell out to ctx and re-parse stdout, introducing a fragile re-serialisation layer that could drift from the real behaviour.
3. Performance — no subprocess spawn per tool call. The MCP server operates in-process.
4. Maintenance — one codebase. Adding a new ctx subcommand naturally extends the MCP surface without a separate wrapper to update.

Alternative considered: a Python MCP server wrapping ctx CLI via subprocess. Rejected because it creates two codebases to keep in sync, adds subprocess spawn overhead, and requires a separate distribution path. The Python approach was useful as a proof-of-concept but the native implementation is the correct long-term design.

The rmcp crate was chosen because it is the official Rust SDK from the modelcontextprotocol organisation, is macro-driven (#[tool_router], #[tool], #[tool_handler]), and is tokio-native, matching the async runtime needed for MCP stdio transport.
