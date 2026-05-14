---
id: ctx-71f267
created: 2026-04-30T15:11:31.212027613Z
status: current
concerns:
- concern-centric-model
- warning-refinement-plan
scope:
  paths:
  - README.md
  - src/commands/check.rs
  - src/commands/refresh.rs
  - src/document.rs
  components:
  - ctx-cli
superseded_by: []
---
### concern-centric-model

The current implementation is still document-oriented, but semantically ctx cares more about concerns than documents. Documents are best understood as the persistence layer that groups one or more workflow claims so they can be versioned, scoped, assembled, and superseded together. Concern ownership and concern-level supersession remain the real semantic core of the system.

That does not imply that ctx should immediately manage concern sections inside markdown bodies as first-class machine-owned subdocuments. Doing so would require stronger structure than the current loose heading convention, would increase authoring overhead, and would shift the tool from document management toward explicit concern-block management. That is a meaningful complexity increase and should not be taken on accidentally through warning cleanup features.

The practical implication is that diagnostics and workflows should respect the current document-first storage model even while reasoning semantically about concerns. When the tool cannot safely and cheaply mutate concern-level body structure without stronger markup, it should avoid pretending that those sections are already first-class managed objects.

### warning-refinement-plan

The current inactive-concern-heading warning should be refined so it fits the existing document-first model and stays high-signal. The main problem with the first implementation is that ordinary supersession can leave historical concern sections behind in partially superseded documents, which then produces warnings that are semantically routine rather than clearly problematic.

The refinement plan should preserve deterministic checks but narrow their scope. First, inactive concern heading warnings should only trigger by default for documents whose status is current. Partially superseded documents should either suppress that warning or downgrade it behind an explicit strict or verbose mode, because residual sections in those documents often reflect ordinary supersession history rather than immediate corpus danger.

Second, the warning should remain targeted at body structure that is likely to mislead current reads. A good next step is to warn only when an inactive heading appears in a document that still presents itself as current for other concerns and where the stale heading could plausibly be read as active guidance. This keeps the signal aligned with user-facing risk instead of treating every historical leftover section as equally important.

Third, the fully superseded document reference warning can remain as a general deterministic signal, but its message should be framed as advisory stale-reference detection rather than structural invalidity.

Fourth, ctx should not add first-class concern-section mutation commands yet. If warning cleanup eventually needs tool support, that should be treated as a separate design decision that likely requires stronger concern-block structure in the markdown format. Until then, warning refinement is preferable to introducing pseudo-structured section editing on top of loosely formatted bodies.

A concrete implementation order is: 1. narrow inactive-concern-heading to current documents only; 2. review whether partially superseded documents need a separate lower-severity diagnostic or none at all; 3. keep fully superseded document references as warnings; 4. update README and guidance wording so these diagnostics are described as drift signals rather than hard invariants about markdown section layout. This plan should be superseded if ctx later adopts explicit concern-block structure in document bodies.

### warning-refinement-plan

The first warning-refinement slice is now implemented. inactive-concern-heading warnings are limited to documents whose status is current, which keeps ordinary supersession residue in partially superseded documents from generating routine warning noise. fully superseded document references remain warning-class diagnostics, but their message is now framed explicitly as advisory stale-reference detection rather than structural invalidity.

This keeps the warning surface aligned with the current document-first model while preserving deterministic stale-context signals for currently active guidance.
