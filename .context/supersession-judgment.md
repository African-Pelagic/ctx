---
id: ctx-a1dd76
created: 2026-04-22T18:56:25.293080176Z
status: partially-superseded
concerns:
- code-aware-context-validation
- code-aware-indexing-extension
- supersession-decision-procedure
scope:
  paths:
  - context-document-management.md
  - context-management-solution.md
  - src/commands/assemble.rs
  - src/commands/check.rs
  - src/git.rs
  components:
  - ctx-cli
superseded_by:
- id: ctx-e55cf5
  concerns:
  - code-aware-indexing-extension
---
### supersession-decision-procedure

When a new document overlaps an existing concern owner, the additive-versus-superseding decision is a semantic judgment rather than a textual diff. The decision procedure is: read the current owning document, compare the new intended claim to the old one, ask whether the old claim is still true, ask whether both documents would help or confuse if assembled together tomorrow, and then either keep both current with additive co-ownership or mark the older concern as superseded. A good practical test is whether future assembly should return both documents as current for that concern.

### code-aware-context-validation

Supersession judgments should be informed by the code as well as by overlapping documents. The agent or human making the decision should inspect the relevant implementation and compare it to the existing context claim. If the code still matches the older context and the new document is complementary, additive ownership is appropriate; if the code no longer matches the older operational claim, the newer document should supersede it for the affected concern.

### code-aware-indexing-extension

A plausible extension is a code-aware indexing layer that maps context documents to files, symbols, and recent code activity so the tool can suggest candidate context, surface likely-stale documents, and hint at possible supersession. That index should remain advisory. The source of truth for ownership, assembly, and supersession should continue to be explicit frontmatter and explicit superseded_by records rather than fuzzy retrieval.
