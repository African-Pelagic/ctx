---
id: ctx-e55cf5
created: 2026-04-22T19:41:24.647302135Z
status: current
concerns:
- code-aware-indexing-extension
- read-side-commands
- validation-rules
scope:
  paths:
  - phase-8-code-aware-indexing.md
  - src/cli.rs
  - src/commands/check.rs
  - src/commands/index.rs
  - src/commands/init.rs
  - src/commands/suggest.rs
  - src/index.rs
  components:
  - ctx-cli
superseded_by: []
---
### read-side-commands

The read surface now includes list, assemble, gc, and suggest. suggest is an advisory retrieval command backed by the derived code index; it accepts a repo path and returns documents whose indexed matched paths or declared scope patterns overlap that path. It does not replace deterministic assembly, but it gives humans and agents a fast way to discover candidate context near unfamiliar code.

### validation-rules

ctx check still validates frontmatter integrity, orphaned concerns, staleness, multi-owned concerns, append-only staged changes, and managed frontmatter tampering. Phase 8 adds an index-derived warning when a document declares scoped paths that no longer match any repo files. That keeps path drift visible without making the derived index the source of truth for assembly or supersession.

### code-aware-indexing-extension

The first code-aware indexing slice is now implemented. ctx index writes a derived .context/.index.json file containing repo file inventory plus document-level matched_repo_paths, missing_scope_paths, and commit metadata. ctx suggest --path uses that derived index to surface candidate context for a repo path, while falling back to declared scope patterns. The index is advisory and derived; explicit frontmatter and superseded_by remain authoritative.
