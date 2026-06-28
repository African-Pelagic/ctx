---
id: ctx-a5491a
created: 2026-06-27T14:58:27.774581630Z
status: current
concerns:
- cross-corpus-assembly
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
### cross-corpus-assembly [r4]

## Assembly with Linked Corpora

Cross-corpus assembly is activated by two new flags on `ctx assemble`:

```
ctx assemble --corpus <link-name>
ctx assemble --all-corpora
```

`--corpus` may be repeated. Both flags compose with all existing predicates
(`--concern`, `--path`, `--component`, `--scope`) and output modes.

## Concern Namespace Semantics During Assembly

Concern names from linked corpora are always prefixed with `{link-name}:` in the
assembled candidate pool (see corpus-link-model for the full namespace rationale).
This has direct consequences for how predicates work:

**Unnamespaced predicates match only the local corpus.**
`ctx assemble --concern auth` selects documents from the local corpus that own `auth`.
It does not match `platform:auth` from a linked corpus, even if `--corpus platform`
is also present.

**Namespaced predicates select from the named corpus.**
`ctx assemble --corpus platform --concern platform:auth` selects documents from the
`platform` link that own `auth` in that corpus.

**Bridges allow transparent unification.**
If `.contextrc` declares `bridge = auth -> platform:auth`, then
`ctx assemble --concern auth` automatically includes matching documents from the
`platform` corpus as if they were local — without `--corpus` or a namespaced concern.
The bridge is the explicit operator declaration that this unification is intentional.

This model means that adding `--all-corpora` without any concern predicate returns
the full active corpus of every linked corpus with all concerns namespaced. This is
intentionally verbose and most useful for exploration or debugging; normal cross-corpus
assembly is driven by explicit concern or component predicates.

## Assembly Execution Sequence

1. Collect candidates from the local corpus (as today, using the local registry).
2. For each named linked corpus (from `--corpus` flags, `--all-corpora`, or active
   bridges):
   - Resolve documents from the linked path or the link cache.
   - Apply the link's pre-filter (concerns/components declared in `.contextrc`), if any.
   - Namespace all concern names as `{link-name}:{concern}`.
   - Mark each candidate with `source = link-name`.
3. Merge all candidates into a single pool.
4. Apply caller predicates across the merged pool:
   - Concern predicates match namespaced concerns unless a bridge maps them.
   - Path predicates match rebased paths (see path rebasing below).
   - Component predicates match as-is (component names are not namespaced).
5. Return results sorted by source then file. Each result carries `source`,
   `active_concerns` (namespaced), `matched_concerns`, and `reasons`.

## Path Rebasing for Linked Corpora

Scope paths from linked corpora are prefixed with `@{link-name}:` to prevent
accidental cross-corpus matches:

```
"src/auth/**"  (local)               →  matches --path src/auth/**
"src/auth/**"  (from platform link)  →  exposed as @platform:src/auth/**
                                        matches --path @platform:src/auth/**
```

Callers select linked-corpus documents by path using the sigil:
```
ctx assemble --corpus platform --path @platform:src/networking/**
```

For remote-linked corpora, paths are rebased identically from the cached registry.

## Explain Mode with Cross-Corpus Sources

`--explain` reports the source for every document and the bridge origin when
applicable:

```
# ctx-abc123 - @platform:.context/networking.md
Active concerns: platform:infra-networking
Included because: concern platform:infra-networking [source: platform]

# ctx-def456 - .context/auth.md
Active concerns: auth
Included because: concern auth [bridged from platform:auth, source: platform]
```

The bridge origin is always surfaced in explain output; it is never invisible.

## When `--all-corpora` Is Appropriate

`--all-corpora` without predicates is useful for:
- Exploring what linked corpora contain (`ctx assemble --all-corpora --paths`)
- Debugging bridge declarations
- Assembling full cross-repo context for a top-level agent orchestrating work across
  many repos simultaneously

For normal task-scoped work, `--corpus <name> --concern <namespaced-concern>` or a
bridge-enabled `--concern <local-name>` is the right pattern.

## Component Predicates Across Corpora

Component names are not namespaced. If the local corpus and a linked corpus both use
a component label `api-gateway`, `--component api-gateway` matches both. This is
intentional: component labels often reflect shared architectural vocabulary (service
names, layer names) that legitimately applies across repos. If this produces noise,
the link's pre-filter (`components = ...` in `.contextrc`) narrows the candidate set
before component predicates are applied.

## What Does Not Change

`ctx assemble` without `--corpus` or `--all-corpora` is identical to today. No
existing behaviour changes. The local-corpus default is preserved unconditionally.
Bridges declared in `.contextrc` are the only case where foreign corpus documents
appear without an explicit `--corpus` flag — and bridges are always visible as
declared configuration in `.contextrc`, never implicit.
