---
id: ctx-fbbc96
created: 2026-04-30T14:01:06.855425166Z
status: current
concerns:
- active-only-search
- assemble-explain
- deterministic-drift-diagnostics
- read-side-commands
- refresh-flow
- validation-rules
scope:
  paths:
  - README.md
  - src/cli.rs
  - src/commands/assemble.rs
  - src/commands/check.rs
  - src/commands/mod.rs
  - src/commands/refresh.rs
  - src/commands/search.rs
  - src/document.rs
  components:
  - ctx-cli
superseded_by: []
---
### read-side-commands [r3]

The read surface now includes search and refresh in addition to the earlier list, assemble, gc, and suggest commands. search provides literal body-text discovery across current and partially superseded documents by default, with an explicit include-superseded mode when stale hits are intentionally needed. refresh provides a first-class path for creating a successor document for a stale concern without rewriting the old document in place.

assemble now supports an opt-in --explain mode that reports deterministic inclusion reasons for each selected document. The reasons are derived only from the explicit selector model: concern matches, component matches, and path matches. This keeps explanation inspectable for both humans and agents and avoids introducing ranking or semantic confidence language into the default read path.

### validation-rules [r3]

ctx check now includes an additional deterministic drift layer beyond frontmatter integrity, multi-ownership, staleness, and missing scoped paths. Active documents are warned when their bodies still contain section headings for concerns that are no longer active in that document, and when they explicitly reference fully superseded document IDs. These checks are rule-based and inspectable from current repo state rather than inferred from fuzzy wording similarity.

The new diagnostics are intentionally conservative. They are meant to surface strong stale-context signals without claiming semantic contradiction detection or ranking likely fixes.

### active-only-search [r3]

The active-only search slice is now implemented as ctx search --query <text>. The default search corpus includes current and partially superseded documents and excludes fully superseded ones unless --include-superseded is set. Output is available in human, JSON, and porcelain forms, and the machine-readable outputs include document identity plus stable line-match data for agents.

### assemble-explain [r3]

assemble-explain is now implemented as an opt-in flag on ctx assemble. When --explain is present, each assembled document reports all deterministic inclusion reasons rather than forcing the caller to infer them from the result set. Human output shows a compact Included because line, and machine-readable outputs expose stable structured reason records with concern-match, component-match, and path-match kinds.

### deterministic-drift-diagnostics [r3]

The first deterministic drift-diagnostics slice is now implemented inside ctx check. The current checks focus on signals that are directly inspectable from corpus metadata and document bodies: missing scoped paths, non-active concern headings left in active documents, and explicit references to fully superseded document IDs. This keeps the warning surface deterministic and debuggable while still surfacing stale-context patterns that strongly risk misleading later agents.

### refresh-flow [r3]

A first usable refresh flow is now implemented as ctx refresh --concern <name> --name <new-doc-name> with optional --from for multi-owner disambiguation and optional --draft-body to seed the successor body from the old concern section. The command identifies the current owner, carries forward scope metadata, creates a new successor document for the single refreshed concern, and records concern-level supersession on the old owner during the same workflow.

The command intentionally refuses ambiguous multi-owner refresh unless the caller specifies --from. The current implementation is agent-oriented and non-interactive: it favors explicit flags and deterministic failure over hidden prompting or automatic owner selection.
