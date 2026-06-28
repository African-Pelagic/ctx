---
id: ctx-eb1647
created: 2026-06-27T15:02:00Z
status: partially-superseded
concerns:
- cross-corpus-design
- corpus-link-model
- cross-corpus-assembly
- cross-corpus-sync-and-check
- cross-corpus-cli-surface
scope:
  paths:
  - src/cli.rs
  - src/commands/assemble.rs
  - src/commands/sync.rs
  - src/subtree.rs
  - .contextrc
  - src/registry.rs
  components:
  - ctx-cli
superseded_by:
- id: ctx-3fedd5
  concerns:
  - corpus-link-model
- id: ctx-a5491a
  concerns:
  - cross-corpus-assembly
---
### cross-corpus-design [r4]

## Problem

ctx already handles traversal within a single filesystem subtree. `ctx assemble
--scope subtree` walks all descendant `.context/` corpora below the working
directory, rebasing scope paths as it goes. `ctx sync --cascade` synthesises child
corpora bottom-up into parent concern summaries. Both operations share a hard
assumption: the target corpora live below the current directory on the same
filesystem.

This assumption breaks in three real situations:

**Peer repositories.** A backend service, a shared library, and an infrastructure
repo each carry their own `.context/`. Work that spans them — a migration, a breaking
API change, a platform upgrade — needs context from all three. There is no common
parent directory to put the CWD in.

**Ancestor corpora.** An organisation may keep a `.context/` corpus above the git
repo root (a monorepo parent, an org-level knowledge layer, or a team directory). The
current traversal model only walks downward; it never crosses a git boundary or climbs
the directory tree.

**Remote corpora.** With the S3 remote backend, a team can push context snapshots
independently of git access. An agent working in one repo may need relevant context
from a sibling repo's latest S3 snapshot, without checking that repo out locally at all.

The cross-corpus feature adds a named-link model: corpora declare explicit links to
other corpora (local paths or S3 remote URLs), and assembly, sync, and check operations
can optionally traverse those links to include, summarise, or validate context from
outside the local subtree.

## Design Principles

1. **Explicit over ambient.** Cross-corpus links are declared in `.contextrc`. ctx
   never auto-discovers foreign corpora by climbing the directory tree or scanning
   known paths. Operators decide which external corpora are in scope.

2. **Read-only by default.** ctx never writes to a linked corpus. Push, append,
   supersede, and refresh only operate on the local corpus. Cross-corpus assembly is
   always a read operation. This preserves corpus ownership.

3. **Same selection model.** Concern, path, and component predicates work identically
   whether the source document is local or linked. The assembly result is a flat list
   of documents regardless of origin; each document carries a `source` field
   identifying which corpus it came from.

4. **Opt-in traversal.** `ctx assemble` without flags assembles only the local corpus
   as today. Cross-corpus assembly requires an explicit flag or predicate. This keeps
   the common case fast and predictable.

5. **Local cache, explicit sync.** For remote (S3) linked corpora, ctx maintains a
   local cache in `.context/.link-cache/{link-name}/`. The cache is populated and
   refreshed explicitly via `ctx corpus fetch`. Assembly reads from the cache, not
   from the network, so normal workflows are offline-safe and latency-free.

6. **Composability with existing hierarchy.** Linked corpora are traversed as peers,
   not as children. Their documents are included in assembly results but are not
   synthesised into the local corpus automatically. Explicit `ctx corpus summarise`
   is the opt-in way to pull a summary of a linked corpus into the local corpus as a
   managed concern document.

### cross-corpus-design [r4]

## Why Not Simply Share a Git Repo / Monorepo

A monorepo with a shared `.context/` at the root is a valid pattern and ctx already
supports it via `--scope subtree`. The cross-corpus feature does not replace that
pattern; it complements it for cases where repos cannot or should not be merged:
different release cadences, different access controls, third-party dependencies,
infrastructure-as-code living in a separate system, or independently deployed services
with their own owners.

## Why Not a Central Context Database

A shared database or hosted context service would solve the traversal problem but
would introduce: a service to operate, a new auth surface, a write-path that crosses
repo boundaries, and a dependency that breaks offline workflows. ctx's value comes
partly from being a local-first, git-native tool. The cross-corpus model preserves
that: links are declared locally, remote content is cached locally, and assembly is
deterministic from local state.

### corpus-link-model [r4]

## What a Corpus Link Is

A corpus link is a named reference from the current corpus to another ctx corpus.
Links are declared in `.contextrc` in a new `[corpus "name"]` section alongside
the existing `[remote "name"]` sections:

```toml
[corpus "platform"]
path = ../platform-infra

[corpus "shared-libs"]
path = /home/harry/code/shared-libs

[corpus "data-team"]
remote = s3://acme-ctx/data-team
```

`path` links point to a local directory that contains a `.context/` subdirectory.
The path is resolved relative to the current repo root (the directory containing
`.contextrc`). Both relative and absolute paths are accepted.

`remote` links point to an S3 URL using the same `s3://bucket/prefix` scheme as the
remote backend design. A `remote`-linked corpus is read from the local link cache
(populated by `ctx corpus fetch`); assembly never makes live S3 requests.

A corpus link may carry optional filter metadata to limit which documents are
considered during cross-corpus assembly:

```toml
[corpus "platform"]
path = ../platform-infra
concerns = infra-networking, infra-auth
components = platform-cli
```

These filters are applied at link resolution time, not at document selection time.
They narrow the candidate set before the caller's own assembly predicates are applied.
They are a performance and noise-reduction tool, not a security boundary.

## Link Identity and Stability

Each link is identified by its name (the `"platform"` part of `[corpus "platform"]`).
Names are local to the declaring corpus — there is no global registry. The same
external corpus can be linked under different names in different repos; ctx does not
attempt to deduplicate or merge them.

Link names appear in assembly output as the `source` field:

```json
{ "id": "ctx-abc123", "source": "platform", "file": "platform-infra/.context/networking.md", ... }
```

## Link Cache Layout

For `remote`-linked corpora, the cache lives at:

```
.context/.link-cache/{link-name}/
  manifest.json        — cached manifest from the most recent fetch
  corpus/              — extracted .context/*.md files from the snapshot
  .registry.json       — registry rebuilt from the cached corpus
```

The cache directory is gitignored (`.context/.link-cache/` added to `.contextignore`
by `ctx init`). It is managed entirely by ctx and should not be edited manually.

For `path`-linked corpora, there is no cache — ctx reads directly from the linked
path's `.context/` directory. The link is always fresh.

## Fetch Semantics

`ctx corpus fetch [link-name]` refreshes the cache for one or all remote-linked
corpora:

- Resolves the S3 URL from `.contextrc`
- Determines the target snapshot: by default `refs/latest`; optionally `--at
  <sha|branch>` for pinned or branch-tracking fetches
- Downloads `manifest.json`, `corpus.tar.gz`, and `registry.json`
- Extracts into `.context/.link-cache/{link-name}/`
- Rebuilds the local registry for the cached corpus

Fetch is always explicit. ctx never auto-fetches during assembly. This ensures
assembly is reproducible and offline-capable.

### cross-corpus-assembly [r4]

## Assembly with Linked Corpora

Cross-corpus assembly is activated by two new flags on `ctx assemble`:

```
ctx assemble --corpus <link-name>
ctx assemble --all-corpora
```

`--corpus` may be repeated. `--all-corpora` includes every declared link. Both
flags may be combined with existing predicates (`--concern`, `--path`, `--component`,
`--scope`).

The selection model is identical to local assembly:

1. Collect candidates from the local corpus (as today).
2. For each named linked corpus, resolve its documents (from the linked path or the
   link cache).
3. Merge candidates into a single pool.
4. Apply predicates (concern, path, component) across the merged pool.
5. Return results, with each document tagged with its `source` (local corpus = `"."`,
   linked corpus = the link name).

If no predicates are given with `--all-corpora`, the full active corpus of every
linked corpus is included alongside the local active corpus. This is intentionally
verbose — the expectation is that `--all-corpora` is paired with at least a concern
or component predicate in normal use.

## Path Rebasing for Linked Corpora

Documents from a `path`-linked corpus carry scope paths that are relative to the
linked repo's root. When these documents appear in cross-corpus assembly results, scope
paths are prefixed with a `@{link-name}:` sigil to make the origin unambiguous:

```
scope.paths: ["src/auth/**"]   →   "src/auth/**"        (local)
                               →   "@platform:src/auth/**"  (linked)
```

This prevents path predicates from accidentally matching paths in linked corpora by
local-corpus path strings. To deliberately select documents from a linked corpus by
path, callers use the sigil:

```
ctx assemble --corpus platform --path @platform:src/networking/**
```

For `remote`-linked corpora, paths are rebased the same way using the cached registry.

## Explain Mode with Cross-Corpus Sources

`--explain` works across linked corpora and reports the source alongside the inclusion
reason:

```
# ctx-abc123 - @platform:.context/networking.md
Active concerns: infra-networking
Included because: concern infra-networking [source: platform]
```

## Relevance Without Shared Paths

When work spans corpora with entirely different path namespaces (common in cross-repo
scenarios), path predicates are less useful for cross-corpus assembly. The practical
selection model in that case is concern-based: both corpora declare concerns using
the same vocabulary, and `ctx assemble --concern <name> --all-corpora` collects
matching documents from any linked corpus.

This makes concern-naming discipline more important in multi-repo settings. Operators
should prefer stable, unambiguous concern names over terse ones when a concern is
expected to be relevant across repo boundaries (e.g. `platform-auth-contract` rather
than `auth`).

### cross-corpus-sync-and-check [r4]

## Sync with Linked Corpora

`ctx sync --cascade` does not traverse corpus links. Cascade sync is a local-subtree
operation: it synthesises child corpora into parent summaries within the local
filesystem subtree, bounded by `.contextrc` exclusions. Cross-corpus traversal is
explicitly separate from the cascade model.

The opt-in operation for pulling linked-corpus content into the local synthesis is
`ctx corpus summarise`:

```
ctx corpus summarise <link-name>
ctx corpus summarise --all
```

This command reads the linked corpus (from the linked path or the link cache), builds
a summary document in the local `.context/` — structurally equivalent to the
child-context synthesis documents produced by `sync --cascade`, but named
`ctx-linked-{link-name}.md` and carrying the concern `linked-context:{link-name}` —
and scopes it so it appears in future local assemblies. The generated document is
managed by ctx and will be overwritten on the next `ctx corpus summarise` call; it
must not be manually edited.

Whether to commit the generated summary document is the operator's choice: committing
it gives peers a summary of linked context without requiring them to run fetch
themselves, but it may not be appropriate if the linked corpus is sensitive. This
decision is independent of whether the link itself is committed (`.contextrc` is
typically committed).

`ctx sync --cascade` never calls `ctx corpus summarise` automatically. The invariant
is that cascade sync is purely local and deterministic from local state.

## Check with Linked Corpora

`ctx check` without flags validates only the local corpus, as today. With
`--all-corpora` or `--corpus <name>`, it extends validation to include cross-corpus
diagnostics. All cross-corpus diagnostics are advisory — they never escalate to errors
even under `--strict`, because the local corpus cannot control the state of linked
corpora.

**LINKED_CORPUS_UNAVAILABLE** (warning): A `path`-linked corpus's `.context/`
directory does not exist or is not readable. The linked corpus is skipped during
assembly without error, but the missing link is reported.

**LINKED_CORPUS_STALE** (warning): A `remote`-linked corpus's cache is older than a
configurable threshold (default: 7 days). The cache is still usable but may be out of
date.

**BRIDGE_CONCERN_ABSENT** (warning): A bridge declared in `.contextrc` refers to a
foreign concern name that no longer exists as an active concern in the linked corpus.
This fires when the linked corpus has renamed, split, or superseded the concern that
the bridge was tracking. The bridge is not automatically removed — ctx reports the
signal and leaves the decision to the operator — because the concern may have been
intentionally renamed and the bridge should be updated, or may have been accidentally
superseded and a new owner should be checked.

The previous LINKED_CORPUS_CONCERN_OVERLAP diagnostic is removed in this design.
Concern namespacing (see corpus-link-model) eliminates silent accidental overlap
structurally: local concerns and foreign concerns always carry distinct names unless
a bridge explicitly unifies them. The diagnostic is replaced by an informational note
in `ctx corpus status` listing concern base-names present in both the local corpus and
a linked corpus without a bridge, so operators can see potential alignment
opportunities without being warned about routine independent naming.

**LINKED_CORPUS_ORPHANED_REFERENCE** (warning): A local document's body explicitly
references a document ID (`ctx-abc123`) that belongs to a linked corpus but is no
longer active there. Requires the linked corpus's registry to be available
(path-linked or recently cached remote-linked).

### cross-corpus-cli-surface [r4]

## New Commands

#### `ctx corpus`

Top-level subcommand grouping all cross-corpus operations. Subcommands:

```
ctx corpus list                     List declared corpus links from .contextrc
ctx corpus fetch [link-name]        Refresh cache for one or all remote-linked corpora
ctx corpus fetch --all
ctx corpus fetch --at <sha|branch>  Pin to a specific snapshot
ctx corpus summarise [link-name]    Write a local summary document for one or all links
ctx corpus summarise --all
ctx corpus status                   Show cache age, document count, and health per link
```

`ctx corpus list` output (human):

```
NAME          TYPE    SOURCE                          CACHED        DOCUMENTS
platform      path    ../platform-infra               (live)        14 active
data-team     remote  s3://acme-ctx/data-team         2026-06-25    8 active
shared-libs   path    /home/harry/code/shared-libs    (live)        5 active
```

#### Extended `ctx assemble`

```
ctx assemble --corpus <link-name>   Include documents from a named linked corpus
ctx assemble --all-corpora          Include documents from all declared corpus links
```

Both flags may be combined with all existing `assemble` predicates and output modes.
`--scope subtree` and `--corpus` may be combined: subtree walks local descendant
corpora; `--corpus` adds named linked corpora on top.

#### Extended `ctx check`

```
ctx check --corpus <link-name>      Check against a specific linked corpus
ctx check --all-corpora             Check against all declared corpus links
```

Emits LINKED_* diagnostic codes described in the sync-and-check concern above.

## Implementation Phasing

**Phase 1 — Link configuration and introspection**
Parse `[corpus "name"]` sections from `.contextrc` (alongside existing remote
sections), including `path`, `remote`, `concerns`, `components`, and `bridge`
declarations. Implement `ctx corpus list` and `ctx corpus status`. Extend `ctx init`
to add `.context/.link-cache/` to `.contextignore`. Implement the informational
overlap note in `ctx corpus status` (concern base-names present in both local and
linked corpus without a bridge). No assembly changes yet. No new crate dependencies.

**Phase 2 — Path-linked assembly with namespacing**
Implement `--corpus` and `--all-corpora` on `ctx assemble` for `path`-linked corpora.
Apply concern namespacing (`{link-name}:{concern}`) and path-rebasing (`@{link-name}:`)
to all linked-corpus candidates. Implement bridge resolution: bridged concerns assemble
transparently without `--corpus`. Implement `--explain` source and bridge-origin
tagging. Extend `ctx check --corpus` with LINKED_CORPUS_UNAVAILABLE and
BRIDGE_CONCERN_ABSENT for path-linked corpora. No new crate dependencies.

**Phase 3 — Remote-linked assembly**
Implement `ctx corpus fetch` using the S3 read path from the remote backend (requires
s3-remote-backend Phase 2/3). Implement assembly from the link cache. Implement
LINKED_CORPUS_STALE check. This phase depends on the S3 remote backend being
implemented first.

**Phase 4 — Synthesis, full check, and guidance**
Implement `ctx corpus summarise`. Implement LINKED_CORPUS_ORPHANED_REFERENCE check.
Implement `ctx corpus fetch --at`. Update `ctx guidance` and AGENTS.md to describe
cross-corpus assembly and bridge declarations.

## Non-Goals

Writing to a linked corpus from the local repo. Auto-discovering linked corpora from
environment variables, git config, or directory-tree climbing. Bidirectional or
symmetric bridge declarations (each side declares its own bridges independently).
Merging concern ownership across corpus boundaries (each corpus owns its concerns
locally; bridges are a read-time assembly alias, not a write-side ownership transfer).
Real-time or event-driven sync (fetch is always explicit). Conflict resolution across
corpora (the supersession model applies only within a corpus). Treating linked corpora
as children in `ctx sync --cascade` (they are peers). Global concern namespaces or
registries (link names are local to the declaring corpus).
