# ctx

`ctx` manages workflow context for software development.

It stores current engineering context in markdown files under `.context/`. Each file declares the concerns it owns, the code it applies to, and any newer files that supersede it.

`ctx` is not a general knowledge base. It is for context that is close to implementation work and likely to change.

## Why This Exists

Teams produce a lot of useful context while they build software:

- why a change exists
- what assumptions are in force
- what tradeoffs were made
- what older understanding is no longer true

That context rarely lives in one good place. It ends up in chat logs, branch notes, PR comments, scratch files, and people’s heads. In agentic workflows, the problem gets worse: multiple agents may read and write context, and stale notes can survive beside current ones with no explicit replacement record.

`ctx` solves that by making workflow context a managed artifact.

## What `ctx` Stores

Each `.context/*.md` document has YAML frontmatter with:

- `id`
- `created`
- `status`
- `concerns`
- `scope.paths`
- `scope.components`
- `superseded_by`

The markdown body stays human-readable. The frontmatter gives the tool enough structure to:

- assemble relevant context deterministically
- track current concern ownership
- record concern-level supersession
- validate the corpus

The markdown corpus is the source of truth. The registry and code index are derived files.

## What Counts As Workflow Context

Good workflow context is short-to-medium-lived engineering context that helps someone change code correctly now.

Typical examples:

- current implementation strategy for a feature
- migration constraints
- rollout assumptions
- debugging findings
- temporary invariants
- deferred tradeoffs that still affect the work

Things that usually do not belong here:

- company mission
- product vision
- roadmap narratives
- tickets and epics
- broad architecture guidance
- coding standards
- onboarding material

Those things matter, but they belong in other artifacts.

## Core Ideas

### Concerns

A concern is a named workflow claim.

Examples:

- `document-lifecycle`
- `validation-rules`
- `cli-help-surface`
- `supersession-decision-procedure`

Think of concerns as versionable engineering assertions, not broad categories like `frontend` or `architecture`.

The main authoring rule is:

Group concerns that are likely to be superseded together.

That rule is hard for humans to follow consistently during normal work. For that reason, `ctx` works best when agents do most of the mechanical upkeep and humans stay focused on judgment.

### Scope

Each document declares the code it applies to:

- `paths` for path globs
- `components` for stable component labels

This lets `ctx` assemble context from explicit metadata instead of fuzzy retrieval.

### Supersession

Supersession is explicit and concern-level.

A document can stay current for one concern while being superseded for another. `ctx` computes active concerns from `concerns` minus any concerns named in `superseded_by`.

The tool records supersession. Humans and agents decide it.

The practical test is simple:

If you assemble this concern tomorrow, should both documents still appear as current?

- If yes, keep additive ownership.
- If no, supersede the older concern explicitly.

That judgment should use the old document, the new document, and the current code.

## Humans and Agents

`ctx` is designed for human and agent collaboration, but it is best treated as agent-operated infrastructure with a human-readable interface.

The intended workflow is:

1. A human and agent discuss the work.
2. They decide what is true, what changed, and what should remain current.
3. The agent updates `.context/` through `ctx`.
4. The human reviews when needed.

This is a better fit than asking humans to manage concern structure by hand in the middle of implementation work.

## Safety Boundary

`.context/` is meant to be committed, so the repo can define a `.contextignore` file at the root.

`ctx` uses `.contextignore` to:

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

## Installation

```bash
cargo install --path .
```

For local development:

```bash
cargo run -- --help
```

## Document Shape

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

## Command Guide

### `ctx init`

Initialize `.context/` and the derived registry in the current repo.

Use it when you adopt `ctx` in a project.

### `ctx new`

Create a new context document.

Use it when you have a new workflow claim to record.

Important flags:

- `--concerns`
- `--paths`
- `--components`
- `--non-interactive`
- `--append`

Use `--append` only when overlap with an existing owner is deliberate and both documents should remain current.

### `ctx append`

Append body text to an existing document under one of its active concerns.

Use it when the document is still the right owner and you only need to add detail.

### `ctx supersede`

Record that one document replaces another for one or more concerns.

Use it when an older operational claim is no longer current.

### `ctx sync`

Rebuild the derived registry from the markdown corpus.

Use it after direct recovery or repair work on `.context/`.

### `ctx list`

Show the active concern roster, owners, files, and notes.

Use it to inspect the current semantic state of the corpus.

### `ctx assemble`

Assemble current context from explicit predicates.

Predicates:

- `--path`
- `--component`
- `--concern`

You can supply multiple concerns with repeated flags or comma-separated values. Concern matching uses OR semantics.

Examples:

```bash
ctx assemble --component ctx-cli
ctx assemble --path 'src/commands/*.rs' --paths
ctx assemble --concern read-side-commands --concern validation-rules
```

`assemble` includes current and partially superseded documents, and excludes fully superseded documents.

### `ctx check`

Validate the context corpus and staged `.context` changes.

It checks:

- invalid frontmatter
- orphaned concerns
- stale documents
- multi-owned concerns
- append-only violations
- managed frontmatter tampering
- missing scoped paths

Use `--strict` to treat warning-class issues as errors.

### `ctx gc`

List fully superseded documents that are candidates for cleanup.

It reports cleanup candidates but does not delete anything.

### `ctx index`

Build or refresh the derived code index in `.context/.index.json`.

Use it when you want fresh path-based advisory data.

### `ctx suggest`

Suggest likely relevant context for a repo path using the derived code index.

Example:

```bash
ctx suggest --path src/cli.rs
```

This command is advisory. It does not replace deterministic assembly.

## When To Use Which Write Command

Use `ctx new` when:

- this is a new workflow claim
- you want a new document to own or co-own concerns

Use `ctx append` when:

- the document already owns the concern
- you are adding more text, not changing ownership

Use `ctx new --append` when:

- a new document should deliberately co-own a concern

Use `ctx supersede` when:

- the new document replaces the older operational truth

## Recommended Workflow

For agents:

1. Run `ctx assemble` before changing code.
2. Optionally run `ctx suggest --path` for discovery.
3. Infer narrow concerns.
4. Inspect the code before deciding supersession.
5. Update the corpus through `ctx`.
6. Run `ctx check`.

For humans:

1. Discuss the work with the agent.
2. Decide what changed and what remains true.
3. Let the agent update the corpus.
4. Review the result when needed.

## Bottom Line

`ctx` gives workflow context a durable structure.

It keeps current claims separate from superseded ones, makes assembly predictable, and gives humans and agents a shared way to maintain context as the code changes.
