---
id: ctx-e97c9b
created: 2026-04-26T22:14:14.797983465Z
status: superseded
concerns:
- active-only-search
- assemble-explain
- deterministic-drift-diagnostics
- refresh-flow
scope:
  paths:
  - README.md
  - src/cli.rs
  - src/commands/assemble.rs
  - src/commands/check.rs
  - src/commands/new.rs
  - src/commands/suggest.rs
  - src/registry.rs
  components:
  - ctx-cli
superseded_by:
- id: ctx-fbbc96
  concerns:
  - active-only-search
  - assemble-explain
  - deterministic-drift-diagnostics
  - refresh-flow
---
### refresh-flow [r3]

A first-class refresh flow should cover the common case where an existing concern is broadly right but partly stale. The intended shape is ctx refresh --concern <name>, operating on one concern at a time, carrying forward scope metadata, optionally carrying forward the old body as a draft, and recording supersession automatically. It should refuse ambiguous multi-owned concerns unless the operator or agent disambiguates them explicitly.

### deterministic-drift-diagnostics [r3]

Diagnostics should stay deterministic and live in ctx check. For now the focus should be explicit drift signals rather than full semantic contradiction detection. Good candidates are signals that strongly risk invalidating the utility of the corpus, such as references to superseded terms, missing scoped references, or other rule-based mismatches between active context and current code. Wording-based heuristics and concern-name similarity should be deferred for now because they are likely to be noisy.

### active-only-search [r3]

A search command should default to active concern owners only and ignore superseded documents unless explicitly asked otherwise. The goal is to reduce cognitive noise from stale files that still exist on disk. The intended shape is an active-only search or grep surface with an opt-in include-superseded mode.

### assemble-explain [r3]

assemble-explain should make the output of ctx assemble easier to trust and easier for agents to consume by exposing why each document was included. The core behavior should be an opt-in explain mode that reports the matching predicates for each assembled document rather than leaving the caller to infer them from the result set.

The primary reasons should be deterministic and derived directly from the current selector model: path match against declared scope.paths, exact component match against scope.components, and active concern match against the requested concern set. If multiple predicates matched, the explanation should report all of them rather than collapsing to one winning reason.

For human output, the explain surface should appear inline with each assembled document in a compact form, for example showing Included because: concern token-expiry, component auth-service, or path src/auth/**. For JSON and porcelain outputs, the explanation should be structured so agents can branch on stable reason kinds rather than parsing prose. A good shape is a reasons array with entries such as kind=concern-match, kind=component-match, and kind=path-match, each carrying the requested predicate and the matched document value where applicable.

The explain feature should remain about inclusion reasoning, not ranking or semantic justification. It should not claim that a document is globally the best context or that its body is still semantically correct; it should only explain why the current deterministic assembly logic selected it.

This concern also supports future diagnostics and refresh workflows. If agents can see exactly which predicates pulled a document into scope, they can better judge whether the document is overly broad, whether assembly is noisy, and whether a more precise concern, path, or component boundary is needed.

This concern would be superseded if assemble adopts a materially different selection model or if explanation expands beyond deterministic inclusion reasons into a broader retrieval or ranking explanation surface.
