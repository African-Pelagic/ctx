---
id: ctx-9aec99
created: 2026-04-22T18:56:25.305713355Z
status: current
concerns:
- cli-help-surface
scope:
  paths:
  - src/cli.rs
  components:
  - ctx-cli
superseded_by: []
---
### cli-help-surface

The CLI help surface should describe each command in task-oriented language so both humans and AI agents can discover the command set quickly from --help output. The top-level help should explain ctx as a workflow-context tool with explicit concerns, scope, and supersession, and each subcommand should have a short description that makes its role obvious at a glance. Flag help should also clarify non-interactive usage and deterministic predicate-based assembly.
