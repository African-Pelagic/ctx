---
name: context-audit
description: Sweep the ctx corpus (.context/) for semantic integrity — surface contradictory claims, drifted claims the code no longer supports, and structural staleness, ranked as a findings report for resolution. Corpus-driven, not session-driven; read-first, report-then-confirm; mutates only via ctx (MCP tools or CLI). Use /context-audit (optionally with a scope: a concern, component, or repo path), or whenever asked to check whether .context is still internally consistent and true to the code.
---

# /context-audit

Sweep the `.context/` corpus for **semantic integrity** and surface every place it contradicts itself or the code, as a ranked findings report the operator resolves. The corpus exists so a later agent can act without re-interviewing anyone — a corpus that confidently asserts two incompatible things, or one thing the code disproves, is *worse than no corpus*, because the agent trusts it.

This is the **corpus-driven** counterpart to [`/capture-context`](../capture-context/SKILL.md). `capture-context` is session-driven and write-oriented ("I just did X → record it, reconcile what I touched"). `context-audit` is a standalone sweep independent of any session; its primary output is a **findings report**, not new context. They share one spine: **ctx-only mutations, verify against the repo, confirm before changing what the corpus says is true.**

## What "semantic integrity" means here — three finding classes

| Class | What it is | How it's found |
|---|---|---|
| **STRUCTURAL** | A doc scopes over code that no longer exists; a concern is orphaned or multi-owned; a stale/orphaned `.org` file; a fully-superseded doc lingering | `ctx` detects it deterministically — cheap, run first |
| **CONTRADICTION** | Two **active** claims describe the *same aspect at the same point in time* and disagree | Semantic reading of doc bodies; `ctx` is blind to this |
| **DRIFT** | An active claim the **code now contradicts** — its own `SUPERSEDE WHEN:` has effectively fired but no supersede was recorded | Semantic reading of doc *against* the code it scopes |

DRIFT is the highest-value class and the one nothing else catches — it requires reading the code, not just the docs.

## The one trap that makes this hard — do not misfire on intended temporal layering

This corpus **deliberately holds "target state" claims next to "current code" claims that look contradictory but are not.** A doc will say a mechanism is being removed *and* another will say it still exists — and both are correct, because the first is explicitly dated, marked `NOT YET IMPLEMENTED`, and says "these stay TRUE of current code until it ships." That is the corpus working as designed, not a defect.

The disambiguator is the corpus's **own idiom**. Before you flag two claims as a CONTRADICTION, check:

- **`SUPERSEDE WHEN:`** — read it as the claim's truth-condition. If claim A's supersede-condition is "when B ships" and B hasn't shipped, A and B are *layered*, not contradictory.
- **Dated "target vs current code" framing** — "agreed target", "NOT YET IMPLEMENTED", "verified <date>, to be changed", "stays true until X". A forward-looking claim and a present-state claim about the same code coexist on purpose.
- **`[[cross-links]]`** — docs that already point at each other and name the coexistence ("will supersede … those stay TRUE until then") have *declared* the relationship. Honour it.

A CONTRADICTION is only real when two claims both purport to describe the **same aspect at the same time** and cannot both be true. If one is a dated target and the other is present-state, that's correct layering — **not a finding.** When genuinely unsure, report it as `verdict: UNSURE` and let the operator judge; never silently drop it and never auto-supersede it.

## Non-negotiables

- **Never hand-edit `.context/` or `.registry.json`.** Every mutation goes through `ctx` (`new`, `append`, `supersede`, `refresh`). Direct edits are recovery-only.
- **Report first; confirm each semantic fix.** The deliverable is the findings report. STRUCTURAL fixes may be batched on one confirmation. Every CONTRADICTION/DRIFT resolution (`supersede`/`refresh`) is confirmed individually — it changes what the corpus says is true.
- **Verify against the repo, not the conversation.** A DRIFT finding is only real once you've read the code the claim scopes over and seen the mismatch. Convert relative dates to absolute.
- **Surface, don't paper over.** When two active claims disagree, the goal is to *resolve* the disagreement (one supersedes the other, or one is corrected) — never to invent a third claim that hides it.

## Use the ctx MCP tools when available

If your tool list has `mcp__ctx__ctx_*` tools (load with ToolSearch if deferred), call them instead of running the `ctx` commands shown below: `ctx_list`, `ctx_search`, `ctx_suggest`, `ctx_assemble`, `ctx_new`, `ctx_append`, `ctx_supersede`, `ctx_refresh`, `ctx_check`, `ctx_sync`, `ctx_index`, `ctx_gc`. They take the same parameters as the CLI flags and need no shell quoting for bodies. Use the CLI only for what the tools lack: `ctx guidance`, `ctx check --strict` (`ctx_check` is never strict), `ctx assemble --explain` / `--scope subtree`, `ctx new --append` / `--components`, and `search --include-superseded`. See the `using-ctx` skill for the full mapping.

## Workflow

### 0. Orient and scope
```bash
ctx guidance          # repo rules — re-read, they change
ctx list              # active concerns → owning doc ids (you need ids for supersede/append)
```
Establish the audit scope from the invocation:
- **No argument** → whole active corpus (fan-out, step 3).
- **A concern / component / repo path** → that cluster only, single-context (skip the fan-out; run steps 1, 4, 5 over just those docs). Use `ctx assemble --concern <c>` / `--component <c>` / `--path <p> --explain` to pull the scoped set.

### 1. Structural pass (deterministic — run first, it's cheap)
```bash
ctx check --strict    # MISSING_SCOPED_PATH, orphaned/stale .org files → STRUCTURAL findings
ctx gc                # fully-superseded docs lingering on disk → STRUCTURAL cleanup candidates
```
Also read `.context/.registry.json` for `orphaned_concerns` and `multi_owned_concerns` — a concern with no owner, or one legitimately vs. accidentally multi-owned, is a STRUCTURAL finding. Record each with its `ctx`-detected reason; these need no code-reading.

**Triage `MISSING_SCOPED_PATH` before trusting it** — the check matches git-*indexed* files, so it has three distinct causes:
- **Genuine** — the file/dir is truly gone (real staleness; fix it).
- **False positive** — the path exists but the index can't see it: an **untracked** file (e.g. an uncommitted `.claude/skills`), a **gitignored** file (e.g. `backend/.env`), or a **bare-directory** scope path written without a `/**` glob. Repair by re-globbing (`dir/**`) or repointing to a tracked equivalent — not by deleting the claim.
- **Superseded-doc noise** — the check still scope-checks superseded docs sitting on disk; these clear when their `ctx gc` candidates are removed.
Run `ctx index` first so the match set reflects the current tree.

### 2. Cluster the active corpus
Group the active docs by component/concern family so each fan-out unit is coherent and small enough to read in full. Typical clusters follow subsystems of the code, for example `storage`, `cli`, `api` or `release`. Derive the actual clusters from `ctx list` + the `components:` frontmatter, don't assume the list above is current.

### 3. Fan out — one subagent per cluster (whole-corpus scope)
Launch the clusters **in parallel** (Agent tool, multiple calls in one message). Each subagent reads every doc in its cluster **in full** plus the code those docs scope over, and returns structured findings. Give each this brief:

> Audit these `.context/` docs for semantic integrity: `<doc ids + paths>`. For every load-bearing claim, (a) check it against the other claims in this set for genuine CONTRADICTION, and (b) check it against the code it scopes over for DRIFT. **Apply the temporal-layering rule**: a dated "target/NOT-YET-IMPLEMENTED" claim coexisting with a "current code" claim about the same thing is NOT a contradiction — read each claim's `SUPERSEDE WHEN:` as its truth-condition first. Return JSON only.

Finding schema each subagent returns:
```json
{
  "class": "CONTRADICTION | DRIFT",
  "verdict": "CONFIRMED | UNSURE",
  "docs": ["ctx-xxxxxx", "ctx-yyyyyy"],   // one for DRIFT, two+ for CONTRADICTION
  "concern": "<owning concern>",
  "claim_a": "<quoted claim>",
  "claim_b": "<the conflicting claim, or the code fact for DRIFT>",
  "evidence": "<file:line the code fact lives at, for DRIFT>",
  "why_not_layering": "<why this survived the temporal-layering check>",
  "proposed_action": "supersede <old> by <new> | refresh <concern> | append correction | correct scope | operator-judgment"
}
```
Subagents **report only** — they do not run `ctx` mutations (per [[subagent-outward-actions]] discipline: gather and return, never act outward).

### 4. Cross-cluster pass
Contradictions that span clusters (a trust doc vs. a data-access doc) won't surface inside any single cluster agent. After the fan-out returns, scan the union of load-bearing claims for cross-cluster conflicts yourself — usually a handful of high-rank claims per cluster, cheap to hold together.

### 5. Synthesize and rank the report
Merge structural + semantic findings, dedupe, and rank: **DRIFT on load-bearing (r4–r5) claims first** (the corpus is actively lying about the code), then CONTRADICTIONs, then STRUCTURAL, then `UNSURE`/low-rank. For each finding give: class, the doc(s), the two conflicting facts, the evidence (`file:line` for drift), and the proposed `ctx` action. This report is the primary deliverable — present it even if the operator stops here.

### 6. Present, then confirm
Show the ranked report. Then:
- **STRUCTURAL** — propose the batch (scope-path corrections via `refresh`/`append`, `gc` candidates to remove, `.org` regen). One confirmation for the batch.
- **CONTRADICTION / DRIFT** — walk each one; confirm the specific `supersede`/`refresh`/correction individually. If the right resolution isn't clear from code + context, say so and ask — don't guess (ctx guidance: "check with the operator").

### 7. Execute (ctx only) and validate

**Pick the operation by what actually drifted:**
- **Body claim wrong, scope paths sound** → `ctx append` a dated "STALENESS CORRECTION" note to the existing doc (the corpus's native idiom). Keeps one owner; the newer dated note wins on assembly. This is the default for partial drift.
- **The whole doc's reason-for-being is false** (a closed gap, a replaced mechanism) → author a full successor and `ctx supersede`.
- **Only the `scope.paths` are stale** (body still true) → **`ctx` has no verb to edit `scope.paths`.** `append` can't touch them and a full successor is lossy for a rich doc, so a **manual repair-edit of the frontmatter** (the guidance's permitted "recovery/repair" exception) + `ctx sync` is the right tool — confirm with the operator first.

**`ctx new` transitioning an owned concern needs `--append`** — creating a doc that will take over a concern another doc still owns requires `--append` to declare the transitional multi-ownership; then `ctx supersede` transfers it. Without `--append` it errors on concern overlap.

Author replacements first, then record the pointer — never hand-edit bodies:
```bash
# DRIFT / CONTRADICTION where a new claim replaces an old one (--append: the concern
# is still owned by the doc you're about to supersede, so declare transitional overlap):
ctx new <name> --non-interactive --append --concerns <c> --paths "<globs>" --components <...> --rank <r> \
  --text "$(cat <<'EOF'
CLAIM: <corrected current claim>.
WHY: <grounded in the code you verified, with file refs>.
DEPENDS ON: <...>. EXCLUDES: <...>. Related: [[...]].
SUPERSEDE WHEN: <the new truth-condition>.
EOF
)"
ctx supersede <OLD_ID> --by <NEW_ID> --concerns <c>

# Stale concern worth carrying forward:
ctx refresh --concern <c> --name <new-name> --draft-body   # then fill via ctx append

# Additive correction under a still-valid concern:
ctx append <ID> --concern <c> --rank <r> --text "$(cat <<'EOF' ... EOF
)"
```
Then:
```bash
ctx check --strict    # confirm the corpus + staged changes validate; fix what it flags
ctx sync              # only if the registry drifted from disk
```
Report: findings by class, what was resolved and how, what the operator deferred, and any remaining `UNSURE` items.

## Smell test before finishing
- Did every CONTRADICTION survive the **temporal-layering** check — i.e. it's not a dated target coexisting with present-state by design?
- Is every DRIFT finding backed by an actual `file:line`, not by what the conversation assumed?
- Did I rank **DRIFT on load-bearing claims** to the top — the corpus lying about the code is the worst failure?
- Did I confirm each semantic supersede/refresh individually, and make every change through `ctx`?
- After my changes, does `ctx assemble` over the audited scope read as one consistent story — no doc still asserting what I just superseded, and no genuine contradiction left unreported?
