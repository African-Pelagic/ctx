---
id: ctx-0451f6
created: 2026-04-30T13:42:22.615737255Z
status: current
concerns:
- context-horizon-evaluation
- read-refresh-implementation-plan
scope:
  paths:
  - README.md
  - src/cli.rs
  - src/commands/assemble.rs
  - src/commands/check.rs
  - src/commands/list.rs
  - src/commands/new.rs
  - src/commands/supersede.rs
  - src/registry.rs
  components:
  - ctx-cli
superseded_by: []
---
### context-horizon-evaluation

A horizon field remains a plausible future refinement, but it is not currently judged necessary for improving context assembly. The current retrieval pain points are more directly about deterministic selection, explainability, stale local context, and noise from stale or over-broad documents than about missing hierarchy.

Horizon would most likely help interpretation rather than inclusion. It can tell a human or agent what to read first, what is more durable, and what should be treated as a local execution plan, but it would not by itself determine which documents belong in an assembly because inclusion is still driven by explicit concerns, scope, and supersession state.

For now the intended priority is to improve active-only search, assemble-explain, deterministic drift diagnostics, and refresh flow before adding horizon. Horizon should be reconsidered later if the corpus begins to accumulate a clear mix of durable operating context and short-lived execution context that is hard to interpret once assembled.

If horizon is revisited later, the preferred shape is a small numeric range such as 1 through 5 with a strict documented meaning, used for interpretation, ordering, filtering, and diagnostics rather than as part of concern identity or supersession truth.

### read-refresh-implementation-plan

The next implementation slice should focus on four related improvements and defer suggest changes: active-only search, assemble-explain, deterministic drift diagnostics, and refresh flow. The recommended implementation order is active-only search first, then assemble-explain, then deterministic drift diagnostics, and finally refresh flow. That order improves read-time trust and corpus hygiene before introducing a higher-leverage write workflow.

Phase 1: active-only search. Add a dedicated search surface that searches active concern owners by default and excludes fully superseded documents unless explicitly asked otherwise. The command should support human, JSON, and porcelain output and should report document id, path, and matching lines or matched concern headings in a stable format. Success criteria: a query over text present only in a fully superseded document is omitted by default and returned when an include-superseded flag is set. Failure criteria: the default search output includes fully superseded documents or returns unstable machine-readable shapes across output modes.

Phase 2: assemble-explain. Extend ctx assemble with an opt-in explain mode that reports deterministic inclusion reasons per assembled document. Reasons should be structured as concern-match, component-match, and path-match and should include the requested predicate and matched document value where applicable. Human output should show a compact Included because line; JSON and porcelain should expose stable structured reason data. Success criteria: for a document matched by multiple predicates, all reasons are reported consistently in every output mode. Failure criteria: explain mode invents ranking or semantic confidence claims rather than reporting deterministic inclusion reasons only.

Phase 3: deterministic drift diagnostics. Extend ctx check with explicit rule-based warnings that catch strong signs of stale or misleading active context without introducing fuzzy semantic heuristics. The first diagnostics should focus on mismatches that are already inspectable from current metadata and file state, such as documents whose scoped paths no longer match repo files, active documents that mention superseded concern names or document ids in a way that likely indicates stale references, and other rule-based scope-to-code mismatches that strongly threaten corpus usefulness. Success criteria: the new warnings are deterministic, explainable, and reproducible from current repo state. Failure criteria: the checks depend on wording similarity, subjective ranking, or other noisy inference that causes warnings to fluctuate or become hard to trust.

Phase 4: refresh flow. Add ctx refresh --concern <name> as the first-class way to update a concern that is broadly right but partly stale. The flow should operate on one concern at a time, identify the current owner, carry forward scope metadata automatically, optionally seed the new document body with the previous body as a draft, and record supersession from the old owner to the new document during the same workflow. It should refuse ambiguous multi-owned concerns unless the operator or agent disambiguates them explicitly. Success criteria: refreshing a singly owned concern produces a new current document with preserved scope and an explicit supersession record on the old owner. Failure criteria: refresh silently chooses among multiple current owners, loses scope metadata, or encourages direct in-place rewriting of the old document instead of producing a superseding successor.

Cross-cutting implementation guidance: keep all four features deterministic and inspectable; do not let them introduce fuzzy retrieval or semantic ranking into the default read path. Prefer stable machine-readable output shapes from the start because these features are primarily useful to agents. Update README and CLI help alongside each slice so the human and agent usage surface stays aligned with the implementation. This implementation-plan concern should be superseded once the four slices are either completed or replaced by a materially different roadmap.
