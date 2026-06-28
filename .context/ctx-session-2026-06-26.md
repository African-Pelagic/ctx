---
id: ctx-49b50a
created: 2026-06-26T17:02:27.906813080Z
status: current
concerns:
- ctx-session-2026-06-26
scope:
  paths: []
  components: []
superseded_by: []
---
### ctx-session-2026-06-26 [r2]

Session with goose on 2026-06-26. Topics covered:

1. Flutter voice agent for Amazon Bedrock — discussed architecture and mobile agent harness landscape. Identified langchain_dart as the best fit. Not implemented; deferred.

2. CTX tool exploration — goose ran ctx --help, ctx guidance, ctx list, ctx assemble, ctx new, ctx index, ctx check. Confirmed ctx is working correctly with an empty corpus in /home/harry/org.

3. Building a ctx MCP extension for goose — initially prototyped as a Python MCP server wrapping the ctx CLI via subprocess. Then recognised the correct approach is native Rust using the rmcp crate.

4. Native Rust MCP server implemented in /home/harry/code/ctx — see concerns mcp-server-implementation and mcp-server-design-rationale for full detail. Result: ctx serve subcommand, 8 tools, live tested, cargo build clean.

5. ctx strategic positioning discussion — covered: concern definition fragility and mitigation strategies (bound damage, usage-based constraints, soft heuristics, agent-driven concern refactoring); supersession discipline (make ambiguity visible rather than forcing automation; ctx check as pressure mechanism; resolution flow); positioning against ADRs/Notion/tribal knowledge; ctx as decision ledger alongside Git as code ledger; eliminating context compaction / cross-session loss / fragmentation as structural consequences of the design, not just features.
