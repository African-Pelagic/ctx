---
id: ctx-3fedd5
created: 2026-06-27T14:58:25.177445606Z
status: current
concerns:
- corpus-link-model
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
superseded_by: []
---
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

A corpus link may carry optional pre-filter to limit which documents are considered
during cross-corpus assembly. These are applied at link resolution time, before the
caller's own assembly predicates, as a noise-reduction tool:

```toml
[corpus "platform"]
path = ../platform-infra
concerns = infra-networking, infra-auth
components = platform-cli
```

## Link Identity and Stability

Each link is identified by its name (the `"platform"` part of `[corpus "platform"]`).
Names are local to the declaring corpus — there is no global registry. The same
external corpus may be linked under different names in different repos; ctx does not
deduplicate or merge them.

Link names appear in assembly output as the `source` field and as the namespace prefix
on all concern names from that corpus (see concern namespacing below).

## Concern Namespacing

**The fundamental problem with cross-corpus concerns is ambiguity.** Two independent
repositories commonly use the same short concern names — `auth`, `deployment`,
`validation`, `error-handling` — and mean entirely different things. Silently merging
these into a single assembly result misleads agents and humans alike: a document about
platform-infra's `auth` model appears alongside a document about the local service's
`auth` model, with no indication they are unrelated.

Equally, *deliberate* reuse of a concern name across repos is a real pattern. If the
`platform` corpus and the `data-team` corpus both actively own a concern called
`event-schema`, they may well have agreed on a shared contract and be tracking their
respective half of it. Treating that as the same problem as accidental overlap — by
either silently merging them or uniformly warning — would suppress a useful coordination
signal.

The model must distinguish accidental overlap from deliberate alignment. The mechanism
is **namespace-first by default, bridge-optional for intentional unification**.

#### Namespace-First Default

All concern names from a linked corpus are prefixed with `{link-name}:` when they
enter the local assembly context. A document in the `platform` corpus that owns the
concern `auth` is visible locally as `platform:auth`. A document in the `data-team`
corpus owning `event-schema` is visible as `data-team:event-schema`.

This means:

- There is never silent accidental concern collision across corpora. `auth` and
  `platform:auth` are distinct names; they do not match each other.
- Assembly predicates must use the namespaced form to select foreign concerns:
  `ctx assemble --concern platform:auth` retrieves documents from `platform` that own
  `auth`; `ctx assemble --concern auth` retrieves only local documents.
- The `LINKED_CORPUS_CONCERN_OVERLAP` check from the previous design is no longer
  needed in its original form. Overlap is structurally prevented by namespacing.
  The check is replaced by a softer informational note in `ctx corpus status` that
  lists concern base-names present in both the local corpus and a linked corpus, in
  case the operator wants to consider bridging them.

#### Bridges: Opt-In Concern Unification

When two corpora deliberately share a concern — both sides track their half of an
agreed interface — the operator declares a *bridge* in `.contextrc`:

```toml
[corpus "platform"]
path = ../platform-infra
bridge = auth -> platform:auth
bridge = event-schema -> data-team:event-schema
```

A bridge declares: *"for the purposes of this corpus, my local concern `auth` and
the linked corpus's concern `platform:auth` refer to the same real-world thing."*

The effects of a bridge:

1. **Assembly unification.** `ctx assemble --concern auth` retrieves documents
   matching `auth` from the local corpus *and* documents matching `platform:auth`
   from the `platform` corpus, in a single result set. Both are included without
   needing `--corpus` or `--all-corpora` flags.

2. **Explain source tagging.** Documents matched via a bridge are tagged in explain
   output: `Included because: concern auth [bridged from platform:auth]`. The bridge
   origin is always explicit; it is never invisible.

3. **Check signal.** `ctx check` emits `BRIDGE_CONCERN_ABSENT` (warning) if a
   declared bridge refers to a foreign concern that no longer exists in the linked
   corpus. This catches the case where the linked corpus renamed or superseded the
   concern the bridge was tracking.

Bridges are unidirectional and local. Declaring `bridge = auth -> platform:auth` in
repo A does not create any entry in repo B. Repo B may declare its own bridge in the
opposite direction if it wants `ctx assemble --concern auth` to pull from repo A, but
that is its own independent decision.

Bridges are intentional coordination artifacts. They should be created deliberately,
not as a convenience to avoid typing the namespace prefix. An undeclared bridge forces
the operator to be explicit (`--concern platform:auth`) and is the correct default
posture for foreign concerns.

## Concern Naming Discipline in Multi-Repo Settings

Namespacing makes the ambiguity problem structural rather than relying on naming
convention discipline. However, concerns that are expected to be bridged benefit from
stable, unambiguous names on both sides. Terse names like `auth` are fine locally;
for concerns that model cross-repo contracts, the local name in a bridge declaration
should be unambiguous enough that a reader immediately understands what real-world
claim it tracks (e.g. `platform-auth-contract` rather than `auth`).

The bridge declaration itself is the explicit record of the alignment decision, which
is more reliable than convention alone.

## Link Cache Layout

For `remote`-linked corpora, the cache lives at:

```
.context/.link-cache/{link-name}/
  manifest.json          — cached manifest from the most recent fetch
  corpus/                — extracted .context/*.md files from the snapshot
  .registry.json         — registry rebuilt from the cached corpus
```

The cache directory is gitignored (`.context/.link-cache/` added to `.contextignore`
by `ctx init`). It is managed entirely by ctx and must not be manually edited.

For `path`-linked corpora there is no cache — ctx reads directly from the linked
path's `.context/` directory. The link is always live.

## Fetch Semantics

`ctx corpus fetch [link-name]` refreshes the cache for one or all remote-linked
corpora. It resolves the S3 URL from `.contextrc`, determines the target snapshot
(default: `refs/latest`; optionally `--at <sha|branch>`), downloads `manifest.json`,
`corpus.tar.gz`, and `registry.json`, extracts them into the cache directory, and
rebuilds the cached registry. Fetch is always explicit; ctx never auto-fetches during
assembly.
