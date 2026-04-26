---
id: ctx-e97c9b
created: 2026-04-26T22:14:14.797983465Z
status: current
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
superseded_by: []
---
### refresh-flow

A first-class refresh flow should cover the common case where an existing concern is broadly right but partly stale. The intended shape is ctx refresh --concern <name>, operating on one concern at a time, carrying forward scope metadata, optionally carrying forward the old body as a draft, and recording supersession automatically. It should refuse ambiguous multi-owned concerns unless the operator or agent disambiguates them explicitly.

### deterministic-drift-diagnostics

Diagnostics should stay deterministic and live in ctx check. For now the focus should be explicit drift signals rather than full semantic contradiction detection. Good candidates are signals that strongly risk invalidating the utility of the corpus, such as references to superseded terms, missing scoped references, or other rule-based mismatches between active context and current code. Wording-based heuristics and concern-name similarity should be deferred for now because they are likely to be noisy.

### active-only-search

A search command should default to active concern owners only and ignore superseded documents unless explicitly asked otherwise. The goal is to reduce cognitive noise from stale files that still exist on disk. The intended shape is an active-only search or grep surface with an opt-in include-superseded mode.
