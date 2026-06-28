---
id: ctx-c4e7e3
created: 2026-06-26T17:01:54.945971979Z
status: current
concerns:
- mcp-server-implementation
scope:
  paths:
  - Cargo.toml
  - src/cli.rs
  - src/commands/mod.rs
  - src/commands/serve.rs
  components:
  - ctx-cli
superseded_by: []
---
### mcp-server-implementation [r3]

ctx now ships a native Rust MCP server via the `ctx serve` subcommand. Running `ctx serve` starts an MCP server over stdio using the official rmcp v1.8.0 Rust SDK (modelcontextprotocol/rust-sdk). The server is implemented in src/commands/serve.rs and wired into cli.rs and commands/mod.rs as the Serve variant.

The server exposes 8 tools: ctx_assemble, ctx_new, ctx_append, ctx_list, ctx_search, ctx_suggest, ctx_check, ctx_supersede. Each tool calls the same internal collect() or action function used by the corresponding CLI subcommand — there is no subprocess or shell-out. This means ctx serve is always in sync with the CLI by construction.

Dependencies added to Cargo.toml: rmcp = { version = "1.8.0", features = ["server"] }, rmcp-macros = "1.8.0", tokio = { version = "1", features = ["full"] }, schemars = "0.8". The #[tool_router(server_handler)] macro on the CtxMcpServer struct handles tool dispatch, schema generation, and ServerHandler wiring automatically.

Each command module was extended with a pub(crate) function (collect, create, append, supersede) that returns typed data rather than printing. The CLI run() functions now delegate to these. This separation cleanly supports both the print path and the MCP return path from the same logic.

To register with goose, add to ~/.config/goose/config.yaml:
  extensions:
    ctx:
      type: stdio
      cmd: ctx
      args: ["serve"]
      enabled: true

The implementation was live-tested: MCP handshake, tools/list returning all 8 tools, ctx_list, and ctx_assemble all responded correctly over stdio JSON-RPC.
