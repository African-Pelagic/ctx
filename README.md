# ctx

`ctx` is a CLI for keeping workflow context in Git beside the code it explains.

It gives humans and agents a shared way to store current engineering claims, scope them to code, and supersede stale context explicitly instead of letting it pile up in chats, scratch notes, and prompts.

## Status

`ctx` is usable now, but it is still early and evolving. The current focus is a deterministic read path, concern-level supersession, and agent-friendly write workflows.

## Why It Exists

Git tells you what changed.
`ctx` tells you why that change was reasonable at the time.

That matters most when:

- work spans many sessions
- multiple agents touch the same stream of work
- assumptions and tradeoffs change over time
- old notes are dangerous if they remain visible as if they were still current

`ctx` handles that by treating workflow context as a managed artifact:

- markdown documents under `.context/`
- explicit concerns owned by each document
- explicit scope via repo paths and components
- explicit concern-level supersession
- deterministic assembly instead of fuzzy retrieval for the primary read path

## What Makes It Different

### Concern-level supersession

One document can stay current for one concern while being superseded for another. That lets context evolve without destructive rewriting or endless additive notes.

### Deterministic assembly

`ctx assemble` defaults to the active corpus and can be narrowed with explicit concerns, paths, and components. The main read path is predictable, testable, and debuggable.

### Built for agent workflows

The structure is human-readable, but the upkeep is agent-friendly. Humans decide meaning. Agents can do the mechanical work of assembling, appending, refreshing, and superseding context.

## Quickstart

Install from the repo:

```bash
cargo install --git https://github.com/African-Pelagic/ctx.git
```

Or for local development:

```bash
cargo run -- --help
```

Minimal flow:

```bash
ctx init
ctx new auth-token-expiry --concerns token-expiry --paths src/auth.rs --non-interactive
ctx assemble --explain
ctx assemble --concern token-expiry --explain
```

If you create a document with initial body text or append new context, provide a rank from `1` to `5`:

```bash
ctx new auth-token-expiry --concerns token-expiry --paths src/auth.rs --non-interactive --text "Auth tokens expire after 15 minutes." --rank 4
ctx append ctx-123abc --concern token-expiry --text "Mobile clients still assume 30 minutes." --rank 3
ctx backfill-ranks --default-rank 3
```


## Core Commands

### `ctx init`

Initializes `.context/` and the derived registry in the current repository.

### `ctx new`

Creates a new context document and assigns concerns, scope paths, and optional components.
When `--text` is used, `--rank` is required.

### `ctx assemble`

Builds the active context set by default, or a narrower set from explicit predicates:

- `--path`
- `--component`
- `--concern`

`--path` may be repeated.

Use `--explain` to show why each document was included.

### `ctx search`

Searches context bodies for literal text. By default it searches current and partially superseded documents and ignores fully superseded ones.

### `ctx append`

Adds text under an existing active concern when the current owner is still correct and you only need to record more detail.
`--rank` is required.

### `ctx backfill-ranks`

Rewrites existing `###` concern headings to carry rank metadata inline as `[rN]`.
Legacy `Rank: N` lines are folded into the heading, and unranked headings receive `--default-rank`.

### `ctx supersede`

Marks one document as replacing another for specific concerns.

### `ctx refresh`

Creates a successor document for a stale concern while carrying forward scope metadata and recording supersession in the same workflow.

### `ctx check`

Validates the corpus and warns on deterministic drift signals such as missing scoped paths or references to fully superseded documents.

## Example Document Shape

```md
---
id: ctx-123abc
created: 2026-04-22T18:00:00Z
status: current
concerns:
  - validation-rules
  - read-side-commands
scope:
  paths:
    - src/commands/check.rs
    - src/commands/suggest.rs
  components:
    - ctx-cli
superseded_by: []
---
### validation-rules [r4]

Notes about validation behavior.

### read-side-commands [r3]

Notes about read-side behavior.
```

## When To Use It

`ctx` is a good fit when the work is large enough that prompt-carried context starts to rot:

- feature work that spans days or weeks
- migrations
- multi-step refactors
- rollout coordination
- investigations where findings change what is true
- any agent-heavy workflow where stale notes are expensive

It is much less useful for tiny one-shot changes where the cost of maintaining context is higher than the value of preserving it.

## Safety Boundary

`.context/` is intended to be committed, so `ctx` supports a root `.contextignore` file.

It uses `.contextignore` to:

- exclude matching `.context/*.md` files from the managed corpus
- exclude matching repo paths from the derived code index
- reject new documents that scope ignored paths

Example:

```text
secrets/**
*.tfstate
*.tfstate.*
*.hcl
.context/private-*.md
```

Important: `.contextignore` excludes files and paths. It does not redact secrets written directly into markdown text.

## Recommended Agent Workflow

1. Run `ctx assemble` before changing code.
2. Use `ctx search` or `ctx suggest` when explicit assembly predicates are not enough.
3. Inspect the code, not just the context.
4. Capture decisions, assumptions, constraints, tradeoffs, and concrete examples when they remove ambiguity.
5. Use `ctx new`, `ctx append`, `ctx supersede`, or `ctx refresh` to keep the corpus current.
6. Run `ctx check`.


## License

GPL-3.0-only. See [LICENSE](LICENSE).
