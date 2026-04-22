---
id: ctx-489a2c
created: 2026-04-22T17:56:46.900009419Z
status: current
concerns:
- document-lifecycle
- frontmatter-model
- supersession-model
scope:
  paths:
  - implementation-plan.md
  - src/commands/append.rs
  - src/commands/new.rs
  - src/commands/supersede.rs
  - src/document.rs
  - src/id.rs
  - src/registry.rs
  components:
  - ctx-cli
superseded_by: []
---
### frontmatter-model

A context document is markdown with YAML frontmatter managed by tooling. The current core fields are id, created, status, concerns, scope { paths, components }, and superseded_by. IDs are opaque and stable, status is derived, and active_concerns are computed from concerns minus any concerns listed in superseded_by entries.

### document-lifecycle

The write-side lifecycle currently works as init -> new -> append -> supersede -> sync. ctx init scaffolds .context and the registry, ctx new creates a frontmatter shell, ctx append adds body content under an active concern heading, ctx supersede records concern-level replacement, and ctx sync rebuilds the derived registry from disk.

### supersession-model

Supersession is concern-level rather than whole-document by default. A document may be current, partially-superseded, or superseded depending on whether any active concerns remain. The practical authoring rule is that a document should group only concerns that are likely to be superseded together; otherwise future supersession becomes coarse and misleading.
