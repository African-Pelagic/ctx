---
id: ctx-73c9a5
created: 2026-06-27T15:01:31.834483777Z
status: current
concerns:
- ctx-session-2026-06-27
scope:
  paths:
  - .context/cross-corpus-assembly-namespace.md
  - .context/cross-corpus-namespace-model.md
  - .context/cross-corpus-traversal.md
  - .context/s3-remote-backend.md
  components: []
superseded_by: []
---
### ctx-session-2026-06-27 [r2]

Session with goose on 2026-06-27. Topics covered:

1. S3 remote backend design — documented in ctx-7f4c27 (.context/s3-remote-backend.md). Core tension: ctx derives value from git-coupling (every commit has its context) but committing .context/ to shared/public repos exposes sensitive material. Design: commit-SHA-keyed S3 object snapshots as a private versioned shadow store. Visibility model: per-document frontmatter field (git | remote | both | local). New commands: ctx remote, ctx push, ctx pull, ctx log. .contextrc gains [remote "name"] sections. S3 bucket versioning is the audit trail; no DAG needed. Push/pull sit outside the local write lock. 4 implementation phases; aws-sdk-s3 + tar + flate2 enter at Phase 2.

2. Cross-corpus traversal design — documented across ctx-eb1647 (cross-corpus-traversal.md), ctx-3fedd5 (cross-corpus-namespace-model.md), ctx-a5491a (cross-corpus-assembly-namespace.md). Core problem: peer repos, ancestor corpora, and remote corpora all lie outside --scope subtree. Design: explicit named corpus links declared in .contextrc as [corpus "name"] sections with path= or remote= fields.

3. Concern namespace model — the central insight of the session. Peer corpora will share concern names either accidentally (two teams both call something auth and mean unrelated things) or deliberately (both sides track their half of a shared contract). Silent merging on accidental overlap is a false positive trap; treating deliberate overlap as an error suppresses a coordination signal. Solution: namespace-first by default (all foreign concerns prefixed {link-name}:{concern}; platform:auth is structurally distinct from auth), with opt-in bridge declarations for intentional unification (bridge = auth -> platform:auth in .contextrc). LINKED_CORPUS_CONCERN_OVERLAP warning removed; replaced by informational note in ctx corpus status. BRIDGE_CONCERN_ABSENT warns when a bridge targets a no-longer-active foreign concern. Bridges are unidirectional and local: each repo declares its own.

4. Cross-corpus assembly mechanics — ctx assemble --corpus <name> / --all-corpora. Unnamespaced predicates match local corpus only; namespaced predicates select from the named link; bridges allow transparent unification without --corpus flags. Path rebasing uses @{link-name}: sigil. Component predicates are not namespaced. ctx corpus summarise is the opt-in way to pull a linked corpus summary into local synthesis; sync --cascade stays local. 4 implementation phases; Phase 3 depends on S3 remote backend Phase 2/3.

### ctx-session-2026-06-27 [r2]

5. Concern-primary architecture — documented in ctx-8818d1 (.context/concern-primary-architecture.md). Concerns covering: concern-primary-architecture, concern-publication-lifecycle, concern-registry-model.

Core insight: ctx is document-primary in storage but concern-primary in semantics. The gap is that concerns are currently just strings — they have no independent state, lifecycle, or publication intent. Three pressures converge on making concerns first-class: cross-corpus assembly (consumers bridge to concerns, not documents), publication intent (working memory vs. external contract), and supersession semantics (split/merge/rename/withdraw have no current representation).

Publication lifecycle: draft → active → published → deprecated. Active is the default (backward-compatible). Published is the explicit external contract. Deprecated carries a transition type (rename, split, merge, withdraw) and successor pointers. Bridges to non-published concerns produce warnings; bridges to withdrawn concerns produce errors.

Concern registry: .context/.concerns.yaml — a structured sidecar managed by ctx tooling, committed to git, included in S3 snapshots. Keys are concern names; values carry state, since-date, owner document ID, optional description, and transition metadata for deprecated entries. Does not touch document body structure at all — purely a semantic metadata layer above documents.

Migration path: Option C ([publish] block in .contextrc) is the near-term stepping stone. ctx concern migrate upgrades a flat [publish] list to a full .concerns.yaml. .concerns.yaml takes precedence when both exist. Near-term cross-corpus assembly code must be written to read .concerns.yaml from linked corpora when present, so concern-primary lands without a flag-day rewrite.

Relation to concern-centric-model (ctx-71f267): that concern correctly deferred making concerns first-class managed objects because the approach was risky at the time. The concern registry resolves this by operating as a structured sidecar with no impact on document body structure. concern-centric-model should be superseded by concern-primary-architecture once the registry is implemented.
