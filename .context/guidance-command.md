---
id: ctx-c58676
created: 2026-04-24T15:38:55.152146192Z
status: current
concerns:
- agents-protocol
- guidance-command
scope:
  paths:
  - AGENTS.md
  - src/cli.rs
  - src/commands/guidance.rs
  components:
  - ctx-cli
superseded_by: []
---
### guidance-command

ctx guidance provides a concise protocol for humans and agents working in a repo that uses ctx. It explains that .context/ is managed by ctx, that direct edits should be limited to recovery or repair work, that relevant work should start with ctx assemble, that updates should go through ctx new, ctx append, and ctx supersede, and that ctx check should run after context changes.

### agents-protocol

ctx guidance --add updates any AGENTS.md files in the repo with the same protocol and creates a root AGENTS.md if none exist. The inserted section is marker-delimited so rerunning the command refreshes the guidance block instead of duplicating it.

### guidance-command

The guidance now also states that managed context should be detailed enough for a later agent to act without another interview. In practice that means capturing decisions, assumptions, constraints, tradeoffs, and concrete examples when they matter, rather than leaving only high-level summaries.
