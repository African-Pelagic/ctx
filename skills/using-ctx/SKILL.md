---
name: using-ctx
description: Use ctx to read, maintain and verify the project context corpus in .context/. MUST be invoked at the start of any task in a repo that has a .context/ directory (before reading or changing code), whenever a decision, constraint, discovery or tradeoff comes up that a later agent would need, before finishing or committing work, and whenever context may be stale or contradictory. Routes to /capture-context for recording and /context-audit for sweeps. Covers every ctx command.
---

# Using ctx

`.context/` holds the decisions, rationale and constraints that explain why the code is the way it is. It exists so a later agent can act without re-interviewing anyone. A corpus that is stale or contradictory is worse than none, because agents trust it. You are responsible for keeping it true, not just for reading it.

Applies when the repo has a `.context/` directory. If it does not, say so and ask before running `ctx init`.

## Rules that always hold

- **Never hand-edit `.context/` or `.registry.json`.** Mutate only through `ctx new`, `append`, `supersede`, `refresh`. Direct edits are for recovery or repair only.
- **Verify before you record.** Check every claim against the code, schema or tests. A decision that was discussed but not shipped is not a fact.
- **Every claim ends with `SUPERSEDE WHEN:`**, the condition that would make it stale.
- **Read assembled context critically.** Check it for contradictions, unmet prerequisites, stale assumptions and mismatches with the code. If it is wrong, fix it through ctx instead of working around it.
- **Confirm before changing what the corpus says is true.** New documents and plain appends can go ahead. `supersede` and `refresh` need the operator's go-ahead.
- **Respect `.contextignore` and `.contextrc`.** Do not record what they exclude.
- If the right semantic change is unclear from the code and current context, ask the operator.

## Prefer the ctx MCP tools over the CLI

If your tool list has `mcp__ctx__ctx_*` tools (load them with ToolSearch if they are deferred), call those instead of shelling out to `ctx`. They run the same in-process functions as the CLI, return structured JSON, and need no shell quoting for `--text` bodies. Everywhere this skill says `ctx <command>`, the MCP tool is `ctx_<command>`: `ctx_assemble`, `ctx_list`, `ctx_search`, `ctx_suggest`, `ctx_new`, `ctx_append`, `ctx_supersede`, `ctx_refresh`, `ctx_check`, `ctx_sync`, `ctx_index`, `ctx_gc`. Parameters mirror the CLI flags (`ctx_supersede` takes `source_id` and `by_id` instead of a positional id and `--by`).

The MCP tools do not cover every flag, so fall back to the CLI for these:

| Need | Use the CLI |
|---|---|
| `--explain`, `--paths` or `--scope subtree` on assemble | `ctx assemble ...` (`ctx_assemble` is always `--scope current`) |
| `check --strict` | `ctx check --strict` (`ctx_check` is never strict) |
| `new --append` or `--components` | `ctx new ...` |
| `search --include-superseded` | `ctx search ...` |
| `init`, `guidance`, `publish`, `service`, `backfill-ranks`, `--json`/`--porcelain` | the CLI only |

The MCP server runs in the directory it was started in, so `ctx_check`, `ctx_sync` and `ctx_index` act on that repo. If a tool call fails with a corpus-not-found error, the server was started elsewhere: use the CLI from the repo root. If no ctx MCP tools are available, use the CLI as written below.

## The lifecycle: what to do at each moment

| Moment | Do this | Then |
|---|---|---|
| **Starting a task** | `ctx assemble --path <each file or dir you will touch> --explain`. Add `--scope subtree` in a monorepo or when nested `.context/` corpora exist. | Read all of it. Note contradictions or claims the code no longer supports. |
| **Area is unfamiliar or the scope is unclear** | `ctx list` for concerns and owners, `ctx suggest --path <p>` for index-derived relevance, `ctx search --query "<term>"` for literal text. | Assemble the documents you found. |
| **A decision, constraint, discovery or tradeoff appears** | Record it. See "Recording" below. | Invoke the `capture-context` skill for anything beyond one small append. |
| **Existing context turns out wrong** | Author the replacement, then `ctx supersede <old-id> --concerns <c> --by <new-id>`. Use `ctx refresh --concern <c> --name <n> [--draft-body]` to scaffold a successor for a stale concern. | Confirm with the operator first. |
| **Before finishing or committing** | Re-run `ctx assemble` for the paths you changed and reconcile what the work changed. Then `ctx check`. | Invoke `capture-context` for the reconcile pass. |
| **Suspected drift, a long gap, or periodic hygiene** | Invoke the `context-audit` skill. | It reports findings and you confirm before changes. |
| **Corpus shape changed** (docs added or moved, child corpora, code moved) | `ctx sync` (add `--cascade` for child `.context/` corpora), `ctx index` after code moves, `ctx gc` to list fully superseded documents as cleanup candidates. | `ctx check`. |

## Recording

| Situation | Command |
|---|---|
| No existing document owns the concern | `ctx new <name> --non-interactive --concerns <c> --paths <globs> [--components <c>] --text "<body>" --rank <1-5>` |
| Additive detail under an existing concern, old claims still true | `ctx append <doc-id> --concern <c> --text "<body>" --rank <1-5>` |
| An existing claim is wrong or replaced | write the successor, then `ctx supersede` |
| An existing concern is stale and needs rewriting | `ctx refresh`, then fill in the successor |

Find document ids with `ctx list`. When `--text` is used, `--rank` is required.

Each concern should carry: the current claim, why it is true, what it depends on, what it excludes, and `SUPERSEDE WHEN:`. Include decisions, assumptions, constraints, tradeoffs and concrete examples where they remove ambiguity. Prefer semantic coverage over verbosity. Do not record incidental detail that will churn within a week. Convert relative dates to absolute.

Run `ctx check` after every context change. `ctx check --strict` escalates warnings to errors.

## Choosing a rank (1-5)

Rank sets the order of `###` concern sections **inside one assembled document**: higher rank is listed first, unranked last, ties keep document order. Today that is the only thing rank does. It does not make a section override another, and `ctx check` does not enforce it.

When choosing a rank, use it to express how foundational the claim is:

- **5**: durable, abstract principles and invariants that other claims must be consistent with.
- **4**: architecture-level decisions and boundaries.
- **3**: component-level decisions and constraints (the usual default).
- **2**: implementation choices that follow from the above.
- **1**: local, volatile detail.

A lower-ranked claim that contradicts a higher-ranked one in the same area is a contradiction to resolve by superseding one of them, not a case where the higher one silently wins. `ctx backfill-ranks --default-rank <n>` assigns a rank to headings that have none.

## Command reference

| Command | Use it for | Common mistake |
|---|---|---|
| `ctx assemble` | Load the context relevant to the work. Predicates: `--path` (repeatable), `--component`, `--concern`. `--paths` lists files only, `--explain` shows why each was included. | Running it with no path and skimming. Pass the paths you will touch. |
| `ctx list` | Active concerns, owners and roster notes. Gives the ids that `append` and `supersede` need. | Guessing document ids. |
| `ctx search --query` | Literal search of document bodies. Active only unless `--include-superseded`. | Expecting fuzzy matching. |
| `ctx suggest --path` | Documents likely relevant to a path, from the code index. | Using it without a fresh `ctx index`. |
| `ctx new` | A new document for a concern nothing owns. `--append` allows deliberate overlap with an existing owner. | Creating a second owner for a concern. Append or supersede instead. |
| `ctx append` | Add to an existing concern. | Appending a claim that contradicts the body. Supersede instead. |
| `ctx supersede` | Record that one document replaces another's concern ownership. | Doing it without confirming. |
| `ctx refresh` | Scaffold a successor for a stale concern. `--from` picks the source when a concern has several owners. | Leaving the scaffold unfilled. |
| `ctx check` | Validate the corpus and staged changes. | Skipping it after a change. |
| `ctx sync [--cascade]` | Rebuild registries, synthesize child corpora into parent concerns. | Forgetting `--cascade` when child corpora exist. |
| `ctx index` | Rebuild the derived code index that `check` and `suggest` use. | Trusting `suggest` after large code moves without re-indexing. |
| `ctx gc` | List fully superseded documents as cleanup candidates. | Deleting by hand. Confirm with the operator. |
| `ctx init` | Create `.context/` and the registry. | Running it in a repo that already has one. |
| `ctx guidance [--add]` | Print usage guidance, or write it into `AGENTS.md` files. | n/a |
| `ctx serve` | MCP server exposing ctx tools over stdio (the tools listed above). | n/a |

Add `--json` for structured output or `--porcelain` for stable text when parsing. Run `ctx <command> --help` before relying on a flag.

## Handing off to other skills

- **`capture-context`**: session-driven and write-oriented. Use it to turn a discussion or piece of work into context and reconcile the corpus.
- **`context-audit`**: corpus-driven sweep for contradictions and drift against the code. It reports; you confirm.
- **`magellan`**: for structural questions about the code (callers, blast radius). Use it alongside ctx. ctx says why the code is the way it is, magellan says how it is wired.
