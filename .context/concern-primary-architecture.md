---
id: ctx-8818d1
created: 2026-06-27T15:51:37.013869149Z
status: current
concerns:
- concern-primary-architecture
- concern-publication-lifecycle
- concern-registry-model
scope:
  paths:
  - .contextrc
  - src/cli.rs
  - src/commands/check.rs
  - src/commands/new.rs
  - src/commands/supersede.rs
  - src/document.rs
  - src/registry.rs
  components:
  - ctx-cli
superseded_by: []
---
### concern-primary-architecture [r4]

## The Document-Primary / Concern-Primary Tension

ctx is currently document-primary in storage and concern-primary in semantics. Documents are the persistence unit: the thing you create, append to, commit to git, and supersede. Concerns are what you actually care about: what assembly selects, what supersession tracks, what cross-corpus bridges reference. The registry, assembly, and check commands all reason about concerns — but concerns have no independent existence beyond being strings owned by documents. They are named by the document that holds them and have no state of their own.

This is a deliberate pragmatic choice. Markdown files are editable, diffable, committable, and human-readable without tooling. But it creates a structural gap: the richest semantic object in the system — the concern — is currently just a string. A name. Its full state must be inferred from which document owns it, what that document's status is, and what supersession records exist. Concerns cannot express their own lifecycle, stability, or publication intent.

The concern-primary architecture is the intended long-term direction. It does not abandon document-as-storage; it adds a layer where concerns can be declared as first-class objects with explicit state, lifecycle, and publication intent. Documents remain the authoring and persistence surface. The concern registry is the semantic layer above them.

## Why Concern-Primary Is the Right Direction

Three converging pressures push toward concern-primary:

**Cross-corpus assembly.** When another corpus links to this one and bridges a concern, they are coupling to a specific claim, not to a document. The document is an implementation detail. The consumer needs to know whether the concern they are bridging to is stable, still maintained, and safe to depend on. That cannot be expressed with document status alone because a document can be current for five concerns at different stability levels simultaneously.

**Publication intent.** The corpus has working memory — active concerns that are true and maintained locally but are not ready to be someone else's dependency — and a published surface — concerns the corpus has committed to keeping stable and signalling when they change. Currently there is no way for a corpus to declare this boundary. The cross-corpus namespace model prevents accidental overlap but cannot express deliberate publication intent. Only the corpus that owns a concern knows whether it is an internal working claim or an external contract.

**Supersession semantics.** Currently supersession is always whole-concern replacement: document A transfers concern X to document B. Concerns can be split, merged, deprecated, or renamed, but these operations have no first-class representation. A consumer who was depending on concern X has no way to discover that it was split into X-tokens and X-sessions, or that it was deprecated in favour of a new concern in a different document. Concern-primary with an explicit lifecycle makes these transitions expressible and discoverable.

### concern-publication-lifecycle [r4]

## Publication State as a Lifecycle

Concern publication state is a four-stage lifecycle:

**draft** — the concern is being formulated. Its name, scope, and body may change structurally. It may be discarded. It should not be depended on by any consumer, internal or external. draft concerns are excluded from cross-corpus assembly by default even with --all-corpora.

**active** — the concern is currently true and maintained within the local corpus. It is used in local assembly, referenced in local documents, and kept current by the owning team. It is not a published external contract. Other corpora should not bridge to active concerns; ctx check warns on any such bridge as BRIDGE_TO_ACTIVE_CONCERN. Active is the default state for all concerns not in the concern registry — this preserves full backward compatibility with the existing corpus.

**published** — the concern is a stable external surface. The owning corpus commits to: keeping it current, signalling transitions (deprecation, split, rename) explicitly, and not removing it without a deprecation period. Cross-corpus bridges are valid only against published concerns without a warning. This is the only state where the owning corpus is making a promise to external consumers.

**deprecated** — the concern was published and is still technically accurate but is being retired. It carries a successor pointer: either a replacement concern name (renamed), a list of split successors (split), or nothing (withdrawn). Bridges to deprecated concerns produce a BRIDGED_CONCERN_DEPRECATED warning with the successor information so consumers can update their bridge declarations. Deprecated concerns remain in cross-corpus assembly for a migration window; they are not immediately excluded.

The lifecycle is forward-only in normal operation: draft → active → published → deprecated. Concerns do not move backward (a published concern does not become active again) except in rare corpus repair scenarios. Withdrawal without a successor (going from published to deprecated with no replacement) is allowed but should be rare and deliberate.

## Concern Transitions as First-Class Facts

Beyond the linear lifecycle, concerns can undergo structural transitions that the current supersession model cannot represent:

**Rename.** The concern name changes but the claim is the same. The old name is deprecated with a successor pointer to the new name. Bridges to the old name receive a BRIDGED_CONCERN_RENAMED warning with the new name.

**Split.** One concern becomes two or more. The original is deprecated with a list of successors. Consumers bridging to the original receive a BRIDGED_CONCERN_SPLIT warning listing the successors so they can update their bridge declarations to whichever successor is relevant.

**Merge.** Two or more concerns are unified into one. The originals are deprecated with a pointer to the merged successor. Less common than split but valid when two concerns turn out to be the same claim expressed differently.

**Withdraw.** A published concern is removed with no successor. This is a breaking change and should be rare. Bridges to withdrawn concerns receive a BRIDGED_CONCERN_WITHDRAWN error (the only cross-corpus check that rises to error severity, because there is no recovery path — the consumer must remove the bridge).

These transition types are stored on the deprecated concern entry in the concern registry, not inferred from document supersession records. Document supersession remains the mechanism for updating the body text; concern transitions are the mechanism for updating the semantic contract.

### concern-registry-model [r4]

## The Concern Registry

The concern registry is a new managed file at .context/.concerns.yaml. It is the authoritative record of concerns that have been explicitly declared as first-class objects with lifecycle state. Concerns not in the registry behave exactly as today: they are active by default, local only, with no publication intent expressed.

The registry is managed by ctx tooling, not hand-edited. It is committed to git as part of the corpus (it is not gitignored). For corpora using the S3 remote backend, it is included in the snapshot bundle so linked corpora can read the publication surface without reading individual documents.

#### Schema

Each entry in the registry is keyed by concern name:



Fields:
- state: draft | active | published | deprecated
- since: ISO date when the concern entered this state
- owner: document ID that currently owns the concern body (redundant with the roster but explicit here for offline reads without full corpus scan)
- description: optional human-readable summary of what the concern represents, especially important for published concerns so consumers understand what they are bridging to
- transition: rename | split | merge | withdraw (deprecated concerns only)
- successors: list of concern names (deprecated concerns only, for rename/split/merge)

#### Commands

ctx concern list                          List all registered concerns with their state
ctx concern list --published              Filter to published only (the corpus API surface)
ctx concern list --deprecated             Filter to deprecated with successor info
ctx concern publish <name>               Move a concern from active to published
ctx concern deprecate <name>             Move a concern from published to deprecated
  --transition rename|split|merge|withdraw
  --successor <name>                     Repeat for multiple successors
ctx concern draft <name>                 Register a new concern explicitly as draft

ctx new and ctx append auto-register concerns as active if they are not already in the registry. ctx concern publish is the explicit act of committing to an external contract.

#### Backward Compatibility

All existing corpora work without a .concerns.yaml file. If the file is absent, all concerns are treated as active (the current implicit default). The registry is strictly additive: adding it to an existing corpus changes no behaviour until a concern is explicitly published or deprecated.

The existing [publish] concerns = ... shorthand in .contextrc (Option C) can be treated as a migration surface: if a .contextrc [publish] block exists and no .concerns.yaml exists, ctx concern list warns that the corpus is using the flat publication model and suggests running ctx concern migrate to generate a .concerns.yaml from the [publish] list with all listed concerns set to published state and all others set to active. This preserves a clean upgrade path from Option C to Option D without requiring a flag day.

## Relation to Document Supersession

Document supersession and concern lifecycle are complementary mechanisms with distinct responsibilities:

Document supersession answers: which document currently expresses a concern's body text? It is the write-side mechanism for updating what a concern says.

Concern lifecycle answers: what is the publication state of this concern, and how has its external contract evolved? It is the semantic mechanism for expressing whether a concern is safe to depend on and how it relates to other concerns.

They are independent: a concern can move through draft → active → published using the same owner document throughout (the body evolves via append, the concern is published via ctx concern publish). A concern can be deprecated and split into two successors while its current body document remains current. Neither operation requires the other.

The concern registry does not replace the document supersession model. It adds a parallel semantic layer that the document model cannot express.

### concern-primary-architecture [r4]

## Migration Path: Option C to Option D

Option C (a flat [publish] concerns list in .contextrc) is the near-term implementation stepping stone toward Option D (the full concern registry). It is worth implementing exactly as described in the cross-corpus-cli-surface concern, but it must be designed so the concern registry can grow out of it without breaking Option C declarations.

The concrete constraint this places on Option C implementation: the [publish] block in .contextrc should be treated as a read surface for cross-corpus assembly from day one, but the tooling should also support reading .context/.concerns.yaml when it is present and preferring it over the [publish] block. When both exist, .concerns.yaml wins. This makes Option C a flat subset of Option D rather than a parallel mechanism that must later be migrated away from.

The upgrade path from Option C to Option D is: ctx concern migrate reads the [publish] block from .contextrc, creates .concerns.yaml with all listed concerns set to published state and all others set to active, and removes the [publish] block from .contextrc. After migration the corpus has a full concern registry and the [publish] block is no longer needed. The migration is idempotent and can be re-run safely.

## Architectural Constraints This Places on Near-Term Work

Any work on cross-corpus assembly, corpus links, or bridge declarations should treat concern names in linked corpora as potentially registry-backed. The cross-corpus assembly code should be written to read .concerns.yaml from a linked corpus if present and use it to determine whether a concern is published, rather than assuming all active concerns are equally valid bridge targets. This avoids a flag-day rewrite when Option D lands.

Concretely: when ctx corpus fetch downloads a remote snapshot, the snapshot bundle should include .concerns.yaml if present (it is already committed to git and therefore in the corpus). When ctx assemble resolves candidates from a linked corpus, it should check the concern registry to classify concerns as draft, active, published, or deprecated before applying bridge resolution and selection. Bridge declarations against non-published concerns should produce the appropriate warning from the start, even if the registry is absent (in which case all concerns are treated as active and the BRIDGE_TO_ACTIVE_CONCERN warning fires by default for any bridge).

### concern-registry-model [r4]

## Relation to the concern-centric-model Concern

The existing concern-centric-model concern (ctx-71f267) records that ctx cares semantically about concerns more than documents, but deliberately defers making concerns first-class managed objects because doing so requires stronger structure than the current loose heading convention and increases authoring overhead. That constraint was correct at the time: attempting to manage concern-body sections as machine-owned subdocuments would have been premature.

The concern registry does not violate that constraint. It does not touch document body structure at all. It is a separate managed file that tracks concern metadata — state, publication lifecycle, transitions — without requiring any change to how concern sections are written inside markdown documents. The document body remains a loosely structured human-authored text. The concern registry is a structured sidecar that tracks the semantic layer above the document.

The concern-centric-model concern should be superseded by concern-primary-architecture once the concern registry is implemented, because the architectural direction has moved from deferral to an explicit design.
