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

The guidance now includes a concrete authoring rubric for detail. It tells agents to prefer semantic coverage over verbosity and, for each concern, to record the current claim, why it is true, what it depends on, what it excludes, and what would cause it to be superseded. It also tells them to add concrete examples only when those examples remove ambiguity and to avoid overfitting context to incidental implementation details that will churn quickly.

### guidance-command

The guidance now also tells agents to read assembled context critically rather than passively. They should check for contradictions, unsatisfied prerequisites, stale assumptions, and mismatches between context and code, and treat any inconsistency as a signal to update or supersede context explicitly.

### guidance-command

The guidance now adds an explicit human-in-the-loop rule for ambiguous semantic changes. Agents should use their judgment for routine upkeep, but if the right semantic change is not clear from the code and current context, they should check with the operator before superseding, reframing, or otherwise changing the meaning of the corpus.
