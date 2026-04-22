---
id: ctx-899588
created: 2026-04-22T17:58:46.160746329Z
status: partially-superseded
concerns:
- assembly-behavior
- read-side-commands
- validation-rules
scope:
  paths:
  - implementation-plan.md
  - src/commands/assemble.rs
  - src/commands/check.rs
  - src/commands/gc.rs
  - src/commands/list.rs
  - src/git.rs
  - src/main.rs
  - src/output.rs
  components:
  - ctx-cli
superseded_by:
- id: ctx-e55cf5
  concerns:
  - read-side-commands
  - validation-rules
---
### assembly-behavior

Read-side selection currently includes current and partially-superseded documents and excludes fully superseded ones. The implemented predicates are concern, component, and a path-pattern match against declared scope.paths. assemble can emit full body content, structured JSON, or just document paths, which makes it usable for both human review and agent pipelines.

### validation-rules

ctx check currently validates parseable frontmatter, orphaned concerns, stale documents, multi-owned concerns, append-only enforcement on staged .context changes, and tampering with managed frontmatter fields. Exit codes are 0 for clean, 1 for errors, and 2 for warnings only; under --strict the warning-class checks are escalated to errors.

### read-side-commands

The current read surface is list, assemble, and gc. list shows the active concern roster and related notes such as multi-ownership or staleness, assemble emits the subset of documents relevant to an explicit predicate, and gc reports fully superseded documents as cleanup candidates without deleting anything.
