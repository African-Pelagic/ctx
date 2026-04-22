---
id: ctx-1e6057
created: 2026-04-22T21:11:21.791748306Z
status: current
concerns:
- contextignore-security-boundary
scope:
  paths:
  - .contextignore
  - README.md
  - src/commands/check.rs
  - src/commands/new.rs
  - src/ignore.rs
  - src/index.rs
  - src/registry.rs
  components:
  - ctx-cli
superseded_by: []
---
### contextignore-security-boundary

The repo now supports a root .contextignore file as a safety boundary for committed workflow context. Matching .context markdown documents are excluded from the managed corpus, matching repo paths are excluded from the derived code index, and ctx new rejects scope paths that would pull ignored material into managed context. This reduces accidental inclusion of sensitive or local-only material, but it does not sanitize secrets written directly into markdown body text.
