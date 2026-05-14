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

`ctx assemble` uses explicit concerns, paths, components, and supersession state. The main read path is predictable, testable, and debuggable.

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
ctx assemble --concern token-expiry --explain
```

## Demo Walkthrough

This is the shortest credible story to show in a terminal demo:

1. A repo already has multiple context documents.
2. You assemble context for one part of the codebase.
3. You discover an assumption is stale.
4. You create or refresh a successor document.
5. You re-assemble and show the new current truth.
6. You run `ctx check` to prove the corpus is still coherent.

### Placeholder: terminal demo video

Add a short GIF or asciinema link here.

Suggested placement:

```md
[![Terminal demo placeholder](docs/placeholders/demo-still.png)](REPLACE_WITH_DEMO_URL)
```

### Placeholder: sample demo transcript

```text
$ ctx assemble --path src/auth.rs --explain
# ctx-a1b2c3 - .context/auth-rollout.md
Active concerns: token-expiry, session-invalidation
Included because: path src/auth.rs

$ ctx refresh --concern token-expiry --name token-expiry-refresh --draft-body
created .context/token-expiry-refresh.md
superseded token-expiry on ctx-a1b2c3

$ ctx check
Context corpus is clean.
```

## Core Commands

### `ctx init`

Initializes `.context/` and the derived registry in the current repository.

### `ctx new`

Creates a new context document and assigns concerns, scope paths, and optional components.

### `ctx assemble`

Builds the relevant context set from explicit predicates:

- `--path`
- `--component`
- `--concern`

Use `--explain` to show why each document was included.

### `ctx search`

Searches context bodies for literal text. By default it searches current and partially superseded documents and ignores fully superseded ones.

### `ctx append`

Adds text under an existing active concern when the current owner is still correct and you only need to record more detail.

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
### validation-rules

Notes about validation behavior.

### read-side-commands

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

## LinkedIn / Launch Assets

Useful assets to add before posting publicly:

- terminal demo video: `REPLACE_WITH_DEMO_URL`
- architecture diagram: `REPLACE_WITH_DIAGRAM_PATH`
- sample repo walkthrough: `REPLACE_WITH_EXAMPLE_REPO_URL`
- crates.io release link if published: `REPLACE_WITH_CRATES_URL`

## License

GPL-3.0-only. See [LICENSE](LICENSE).
