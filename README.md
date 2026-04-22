# ctx

`ctx` is a workflow context management tool for software development.

It helps teams and agents keep short-to-medium-lived engineering context in markdown documents with explicit metadata, explicit concern ownership, and explicit supersession.

The goal is not to build a general knowledge base. The goal is to make it easier for humans and AI agents to answer questions like:

- What context is relevant to the code I am changing right now?
- Which earlier notes are still current, and which have been replaced?
- How do I add new implementation context without letting old context silently linger?
- How do humans and agents collaborate on context updates without relying on implicit memory?

## The Problem

Workflow context is awkward to manage in normal development workflows.

During feature work, debugging, refactoring, migrations, and rollout work, teams produce a lot of local context:

- why a change is being made
- what tradeoffs were accepted
- what assumptions are currently in force
- what should happen next
- what earlier understanding has been invalidated

This context matters, but it does not fit cleanly into code, tickets, or long-lived architecture docs.

If it is left in chat logs, branch-local notes, PR comments, scratch files, or people’s heads, it becomes hard to reuse and hard to trust. In agentic workflows this gets worse, because multiple agents may be producing or consuming context, and old notes can easily be retrieved alongside new ones without any explicit indication that they have been superseded.

The result is familiar:

- relevant context is hard to find
- stale context is hard to identify
- overlapping notes accumulate
- agents and humans read the same concern differently
- there is no explicit record of when one claim replaced another

## The General Solution

The general solution is to treat workflow context as a managed artifact rather than as incidental text.

That means:

- store context in explicit documents
- attach machine-readable metadata to those documents
- declare which concerns each document owns
- scope documents to the code they are about
- make supersession explicit instead of implicit
- assemble context deterministically for a given task

There is also an important boundary to hold.

This is not a tool for all context. Some context is too durable, too broad, or too directionally relevant to belong in workflow context management:

- company mission
- product vision
- roadmap narratives
- tickets and epics as work-organization objects
- broad architectural style guides
- code conventions
- onboarding material

Those things still matter, but they usually belong in other artifacts. `ctx` is for context that is close enough to implementation work that it may need explicit supersession later.

## How `ctx` Solves It

`ctx` models workflow context as markdown documents under `.context/`, each with YAML frontmatter.

Each document declares:

- a stable `id`
- a `created` timestamp
- a derived `status`
- a set of `concerns`
- a `scope` with `paths` and `components`
- any `superseded_by` records

This gives `ctx` a model with a few important properties:

- assembly is deterministic
- concern ownership is explicit
- supersession is explicit
- current and partially superseded documents can coexist safely
- humans and agents can both inspect the same state

The current source of truth is the markdown corpus plus frontmatter. Derived files such as the registry and code index are helpful, but they are not authoritative.

## Core Ideas

### Concerns

A concern is a named slice of workflow meaning.

Examples:

- `document-lifecycle`
- `validation-rules`
- `cli-help-surface`
- `supersession-decision-procedure`

Concerns matter because supersession happens at the concern level. A document can stop being current for one concern while remaining current for others.

In practice, concerns are usually best thought of as versionable engineering assertions rather than broad business or technical categories.

Good concern shapes are things like:

- current implementation strategy for a feature
- migration constraints for an in-flight change
- rollout assumptions
- current validation behavior
- temporary subsystem invariants
- debugging findings tied to an active problem
- deferred tradeoffs that matter to current code changes

Less useful concern shapes are broad, durable buckets like:

- frontend
- backend
- payments
- architecture
- coding standards
- product vision

Those are usually domains or reference topics, not workflow claims that are likely to be explicitly superseded later.

The main authoring rule is simple:

Group only concerns that are likely to be superseded together.

If you put unrelated concerns into the same document, future supersession becomes coarse and misleading.

That rule is intentionally strong, but it is also hard for humans to follow consistently in the middle of software work. Real tasks often touch several overlapping ideas at once, and predicting future supersession boundaries is not easy.

This is one reason `ctx` is best treated as primarily agent-operated infrastructure with a human-readable interface, rather than as a system that depends on humans manually curating every context document themselves.

### Scope

Each document also declares scope:

- `paths`: path globs that indicate what code the document is about
- `components`: stable labels for deterministic assembly

This is how `ctx` selects relevant documents without relying on fuzzy retrieval by default.

### Supersession

Supersession is explicit. It is not inferred from overlap alone.

This is important. Whether a new document should co-own a concern or supersede an older claim is a judgment call. The tool should record that decision, not invent it.

The practical decision procedure is:

1. Read the current owning document.
2. Compare the new claim to the old claim.
3. Inspect the relevant code when needed.
4. Ask whether the old claim is still true.
5. Ask whether assembling both documents tomorrow would help or confuse.
6. If both should remain current, use additive ownership.
7. If the older operational claim is no longer true, supersede it explicitly.

That judgment is exactly where humans and agents collaborate well:

- the tool maintains explicit structure
- humans and agents examine code and intent
- humans and agents decide whether a claim is still current
- the tool records the resulting state

## Humans and Robots

`ctx` is designed for human and AI-agent collaboration.

More specifically, it is designed to work best when agents do most of the mechanical context maintenance and humans stay focused on judgment.

That is not just because agents can read markdown. It is because the model makes judgment points explicit while leaving the repetitive bookkeeping to software.

Humans and robots are the components in the system that can decide things like:

- whether a new note is additive or replacing
- whether an older claim is outdated relative to the code
- whether a concern name is too broad
- whether two documents should assemble together as current

The CLI gives both sides a shared way to record those decisions.

In the intended operating model:

- the human discusses the work with the agent
- the human and agent decide what is true, what changed, and what should remain current
- the agent updates the context corpus
- the agent infers likely concerns, document boundaries, and supersession candidates
- the human reviews or corrects those decisions when needed

This is a better fit than asking humans to manually manage concern structure in the middle of implementation work. Humans are good at deciding meaning. Agents are good at repeatedly turning those decisions into structured updates.

A typical collaboration pattern looks like this:

1. An agent assembles relevant current context before touching code.
2. A human or agent notices that some context is incomplete or outdated.
3. A new context document is created.
4. A human or agent inspects the old claim, the new claim, and the current code.
5. The new document either co-owns the concern or supersedes the older one.
6. Future agents and humans see the updated state directly.

This is much safer than relying on chat history, loose notes, or semantic retrieval alone.

## What `ctx` Is Not

`ctx` is not:

- a general knowledge base
- a replacement for product docs
- a replacement for ADRs
- a ticketing system
- a semantic search engine that decides truth automatically

The default read path is deterministic from frontmatter. There is a code-aware index and advisory suggestions, but those remain derived and non-authoritative.

## Installation

From the repo root:

```bash
cargo install --path .
```

Or while developing locally:

```bash
cargo run -- --help
```

## Document Format

A context document is markdown with managed frontmatter:

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

`ctx` manages the frontmatter shape and derived status. The document body remains plain markdown.

## Command Guide

### `ctx init`

Initialize a `.context/` corpus in the current repository.

Use it when:

- starting to manage workflow context in a repo
- bootstrapping a new project

It creates:

- `.context/`
- `.context/.registry.json`
- `.context/.index.json` is also recognized as a derived file and ignored by `init`

### `ctx new`

Create a new context document shell.

Use it when:

- you have a new workflow claim to record
- you need a separate document rather than more text in an existing one

Important options:

- `--concerns`
- `--paths`
- `--components`
- `--non-interactive`
- `--append`

Use `--append` only when overlap with an existing concern owner is deliberate and you want additive co-ownership rather than immediate replacement.

Example:

```bash
ctx new cli-help-surface \
  --non-interactive \
  --concerns cli-help-surface \
  --paths src/cli.rs \
  --components ctx-cli
```

### `ctx append`

Append body text to an existing document under one of its active concerns.

Use it when:

- the existing document is still the right owner
- you are adding more detail under a concern it already owns

Do not use it to change ownership. It changes content, not concern semantics.

Example:

```bash
ctx append ctx-9aec99 \
  --concern cli-help-surface \
  --text "The top-level help should explain ctx as a workflow-context tool."
```

### `ctx supersede`

Record that one document has replaced another for one or more concerns.

Use it when:

- an older operational claim is no longer current
- a new document should become the active owner of a concern

Example:

```bash
ctx supersede ctx-899588 \
  --concerns read-side-commands,validation-rules \
  --by ctx-e55cf5
```

### `ctx sync`

Rebuild the derived registry from the markdown corpus.

Use it when:

- you changed documents on disk directly
- you want to regenerate the registry explicitly

Many command flows already keep the registry current, so this is mainly a repair or manual-sync tool.

### `ctx list`

Show the active concern roster.

Use it when:

- you want to see who currently owns each concern
- you want to inspect multi-ownership or staleness notes

This is the quickest way to understand the current semantic state of the corpus.

### `ctx assemble`

Assemble current context relevant to explicit predicates.

Use it when:

- starting work in a scoped part of the repo
- gathering context before making a change
- feeding a deterministic context set to an agent

Predicates:

- `--path`
- `--component`
- `--concern`

Examples:

```bash
ctx assemble --component ctx-cli
ctx assemble --path 'src/commands/*.rs' --paths
ctx assemble --concern validation-rules --json
```

`assemble` includes current and partially superseded documents, and excludes fully superseded documents.

### `ctx check`

Validate the context corpus and staged `.context` changes.

Use it when:

- reviewing the health of the corpus
- validating before commit
- looking for stale or malformed context

It currently checks:

- invalid frontmatter
- orphaned concerns
- stale documents
- multi-owned concerns
- append-only violations in staged `.context` changes
- managed frontmatter tampering
- missing scoped paths in the repo

Exit codes:

- `0`: clean
- `1`: errors present
- `2`: warnings only

Use `--strict` to escalate warning-class issues to errors.

### `ctx gc`

List fully superseded documents that are candidates for cleanup.

Use it when:

- you want to prune old context deliberately
- you want to review what has been fully replaced

It does not delete anything.

### `ctx index`

Build or refresh the derived code index.

Use it when:

- you want fresh advisory indexing data
- repo files or scope paths have changed significantly
- you want to drive `ctx suggest`

The index is stored in `.context/.index.json` and is derived, not authoritative.

### `ctx suggest`

Suggest likely relevant context from the derived code index.

Use it when:

- you are entering unfamiliar code
- you want candidate context near a path quickly
- an agent needs hints before a deterministic `assemble`

Example:

```bash
ctx suggest --path src/cli.rs
```

This is advisory retrieval. It helps discovery, but it does not replace `assemble` as the default deterministic read path.

## When To Use Which Write Command

Use `ctx new` when:

- this is a separate workflow claim
- you want a new document to own or co-own concerns

Use `ctx append` when:

- the document already owns the concern
- you are just adding more body text

Use `ctx new --append` when:

- a new document should deliberately co-own a concern with an existing current document

Use `ctx supersede` when:

- the new document replaces the older operational truth for that concern

The key question is:

If I assemble this concern tomorrow, do I want both documents returned as current?

If yes, additive ownership may be right. If no, supersede.

## Recommended Workflow

For humans:

1. Start with `ctx assemble` for the area you are about to change.
2. Read the current documents.
3. Make the code change.
4. Discuss new findings or changed assumptions with the agent.
5. Let the agent propose or perform the context update.
6. Review whether the new document should be additive or superseding.
7. Run `ctx check`.

For agents:

1. Use `ctx assemble` as the default source of relevant current context.
2. Optionally use `ctx suggest --path` as an advisory discovery step.
3. Infer narrow concerns that can be cleanly superseded later.
4. Inspect the code before deciding supersession.
5. Propose or perform the document update on the human’s behalf.
6. Use `ctx supersede` only when the older claim is no longer current.
7. Run `ctx check` before handing work back.

The important point is that `ctx` should reduce the human burden of maintaining workflow context. A human should often be able to stay at the level of:

- discussing the work
- deciding what is true
- deciding whether old context still stands

Then the agent can translate that into:

- concern selection
- document creation
- append operations
- supersession updates
- corpus validation

## Why This Works

`ctx` works because it separates three things that are often conflated:

- durable reference knowledge
- workflow context for current engineering work
- judgment about whether context is still true

The tool manages the structure. Humans and agents make the semantic decisions. The resulting state is explicit, inspectable, and reusable.

That is the point of the system: not to eliminate judgment, but to give judgment a durable place to land.
