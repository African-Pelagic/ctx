# ctx

`ctx` is a version-controlled, queryable decision layer for a codebase.

It stores current engineering context in markdown files under `.context/`. Each file declares the concerns it owns, the code it applies to, and any newer files that supersede it.

`ctx` is not a general knowledge base. It is for context that is close to implementation work, likely to change, and worth keeping under version control beside the code.

## Mental Model

Git stores what changed.
`ctx` stores why the change was reasonable at the time.

Git history is a sequence of code states.
`ctx` is a sequence of evolving engineering claims.

Used together, they let a human or agent reconstruct both:

- the code delta
- the reasoning delta

`ctx` does not require context for every commit. It enables Git-tracked decision recording alongside commits when the change is semantic, decision-bearing, or likely to matter later.

## Why This Exists

Git records what changed. It does not reliably record why the change was reasonable at the time.

Teams produce a lot of useful decision context while they build software:

- why a change exists
- what assumptions are in force
- what tradeoffs were made
- what older understanding is no longer true

That context rarely lives in one good place. It ends up in chat logs, branch notes, PR comments, scratch files, commit-message fragments, and people’s heads. In agentic workflows, the problem gets worse: multiple agents may read and write context, and stale notes can survive beside current ones with no explicit replacement record.

`ctx` solves that by making workflow context a managed artifact that can be tracked in Git alongside the code.

Put simply:

- Git records code state transitions.
- `ctx` records the workflow and decision context around those transitions.

`ctx` does not require context for every commit. It gives teams a way to keep decision-bearing context under version control when it matters.

## What `ctx` Is Good At

Three parts of the current model matter most.

### Concern-level supersession

This is the core differentiator.

A document can stay current for one concern while being superseded for another. That gives you non-destructive semantic updates instead of overwrite-or-append-everything behavior.

### Deterministic assembly

`ctx` uses explicit concerns, scoped paths, scoped components, and supersession state. It does not depend on fuzzy retrieval or embeddings for its primary read path.

That makes assembly:

- predictable
- testable
- debuggable

### Agent-first structure

`ctx` works best as agent-operated infrastructure with a human-readable interface. Humans decide meaning. Agents do the mechanical upkeep.

That is important because most teams will not maintain this structure manually at high frequency.

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

## What `ctx` Eliminates

`ctx` is useful because it removes a few recurring context failures in agent workflows.

### Context loss across sessions

Without `ctx`, context often lives in prompts, chats, and local notes. When sessions reset, humans or agents reconstruct it imperfectly.

With `ctx`, context is externalized, versioned, and queryable. Sessions become stateless clients of a persistent corpus.

### Summarisation decay

Without `ctx`, long-lived work often depends on repeated compaction. Context is summarized to fit prompts, and subtle constraints disappear.

With `ctx`, retrieval is selective rather than compressive. The original reasoning remains intact in the corpus.

### Fragmented truth

Without `ctx`, relevant reasoning is scattered across PRs, chats, tickets, docs, and agent logs.

With `ctx`, workflow context has one structured corpus, explicit supersession, and deterministic assembly.

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

The best fit for `ctx` is context around semantic changes:

- why a feature was implemented this way
- what constraint ruled out another approach
- what changed in the current understanding of the work
- what earlier context is now obsolete

That is why `ctx` works well alongside commits without trying to replace Git history, ADRs, or ticket systems.

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

The agent should also treat assembled context as something to evaluate, not just repeat. It should:

- capture enough detail that a later agent can act without another interview
- prefer semantic coverage over verbosity
- record the current claim, why it is true, what it depends on, what it excludes, and what would cause it to be superseded
- check for contradictions, unsatisfied prerequisites, stale assumptions, and mismatches between context and code
- ask the operator before making an ambiguous semantic change

This separation is subtle but important:

- `AGENTS.md` tells an agent how to behave
- `ctx` tells an agent what is currently true about the system

That keeps stable operating instructions separate from changing workflow state.

## Why This Works For Agent Workflows

Most agent workflows still treat context as something fragile and ephemeral. They stuff it into prompts, compress it, and restitch it across sessions.

`ctx` supports a different loop:

1. query the current state with explicit predicates
2. act on that state
3. update that state through structured documents

In practice, that lets agents operate more like stateless executors:

- `ctx assemble`
- act
- `ctx update` through `new`, `append`, or `supersede`

That does not solve judgment automatically, but it does replace prompt-based context carrying with a persistent, queryable system.

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

### `ctx guidance`

Print the repo’s `ctx` usage protocol for humans and agents.

Use it when:

- an agent is new to the repo
- you want a concise reminder of the `ctx` workflow
- you want to refresh repo-level instructions

`ctx guidance --add` updates any `AGENTS.md` files in the repo with the current `ctx` guidance block. If no `AGENTS.md` exists, it creates one at the repo root.

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
3. Use `ctx guidance` if the repo workflow is unclear.
4. Infer narrow concerns.
5. Capture decisions, assumptions, constraints, tradeoffs, and examples when they remove ambiguity.
6. Inspect the code before deciding supersession.
7. Read context critically rather than passively.
8. Ask the operator before making an ambiguous semantic change.
9. Update the corpus through `ctx`.
10. Run `ctx check`.

For humans:

1. Discuss the work with the agent.
2. Decide what changed and what remains true.
3. Let the agent update the corpus.
4. Review the result when needed.

## Bottom Line

`ctx` gives workflow context a durable structure.

It keeps current claims separate from superseded ones, makes assembly predictable, and gives humans and agents a shared way to maintain context as the code changes.
