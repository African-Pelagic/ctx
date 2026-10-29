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

### mcp-server-implementation [r3]

Update 2026-10-14: the server now exposes 12 tools. Added ctx_refresh, ctx_sync, ctx_index and ctx_gc, each backed by a pub(crate) function in refresh.rs, sync.rs, index.rs and gc.rs that the CLI run() also calls. Still not exposed: init, guidance, publish, service, backfill-ranks. Parameter gaps versus the CLI: ctx_assemble has no explain, paths or subtree scope; ctx_check is never strict; ctx_new has no --append or --components; ctx_search cannot include superseded documents. The skills using-ctx, capture-context and context-audit tell agents to prefer the mcp__ctx__ctx_* tools and fall back to the CLI for those gaps. Correction to the text above: the dependency is rmcp version 1 with features server and transport-io, and the server struct is CtxServer, not CtxMcpServer. SUPERSEDE WHEN: the tool set or the skills' MCP-first guidance changes.
