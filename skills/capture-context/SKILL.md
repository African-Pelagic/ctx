---
name: capture-context
description: Capture the decisions, rationale, and constraints from a recent discussion or work session into the ctx corpus (.context/), then reconcile the corpus — supersede claims the session proved wrong, refresh stale concerns, append genuinely new detail. Uses the ctx CLI only (never hand-edits .context). Use /capture-context, or whenever asked to record/update project context after a discussion, decision, or piece of work.
---

# /capture-context

Turn what was just decided or discovered into durable project context, and leave the `.context/` corpus **semantically consistent with the code as it now stands**. The corpus exists so a later agent can act without re-interviewing anyone; your job is to keep it true, not just to add to it.

Two failure modes this skill guards against, in order of importance:
1. **Recording something false** — a decision that was discussed but not shipped, or a claim the code contradicts. Worse than recording nothing.
2. **Leaving a contradiction** — adding a new true claim while an old doc still asserts the opposite. A reader who assembles both is now misled.

## Non-negotiables

- **Never hand-edit files under `.context/` or `.registry.json`.** All changes go through `ctx` commands (`new`, `append`, `supersede`, `refresh`). Bodies are written via `--text`. Direct edits are for recovery/repair only.
- **Verify before you record.** Every claim must be checked against the actual repo state (code, migrations, schema, tests) — not against what the conversation *hoped* was true. This corpus has been burned by descriptive docs that drifted false; catching that drift is half the value here.
- **A claim without a `SUPERSEDE WHEN:` is unfinished.** Every concern records what would make it stale. It's the repo idiom that keeps the corpus self-maintaining — honour it.
- **Confirm before mutating existing concerns.** Adding new context is low-risk; `supersede` and `refresh` change what the corpus *says is true*. Present the plan and get a go-ahead before executing those (see step 5).

## Workflow

### 0. Orient
```bash
ctx guidance          # repo-specific rules (re-read; they can change)
ctx list              # active concerns, owning doc ids
```
Note the concern names and ids — you'll need ids for `append`/`supersede`.

### 1. Distill candidate claims from the discussion
From the recent session (and, if relevant, recent commits / PR threads), extract the **decisions, rationale, tradeoffs, and constraints that explain why the code is the way it is** — not a narrative of the work. Prefer semantic coverage over verbosity; drop churny implementation detail that will rot in a week.

For each candidate, draft the five things a concern should carry:
- the **current claim**,
- **why** it's true,
- what it **depends on**,
- what it **excludes**,
- **SUPERSEDE WHEN:** the condition that would make it stale.

### 2. Verify each candidate against the repo
Read the code/migrations/schema/tests the claim describes. Drop or correct anything the repo doesn't actually support. Convert relative dates to absolute. If a candidate turns out to *contradict* an existing doc, that's not a blocker — it's a supersede target for step 4.

### 3. Locate the concerns each candidate touches
```bash
ctx search --query "<key term>"          # literal search of active bodies
ctx assemble --path <repo/path> --explain # what context already claims scope over this code
ctx suggest --path <repo/path>            # index-derived relevance
```
Read the owning docs in full. Check their `SUPERSEDE WHEN:` clauses against what the session established — that clause often tells you exactly whether to supersede.

### 4. Classify each candidate

| Situation | Action |
|---|---|
| New concern, no existing doc owns it | `ctx new` |
| Additive detail under an existing concern, existing claims still true | `ctx append` |
| An existing concern's claim is now **wrong or replaced** | author replacement, then `ctx supersede` |
| An existing concern is **stale and needs rewriting** (predecessor worth carrying forward) | `ctx refresh` |

`supersede` vs `refresh`: use **supersede** when you're recording that doc B replaces doc A's ownership of a concern (you write B). Use **refresh** when you want ctx to scaffold the successor for a stale concern (optionally seeded from the old body with `--draft-body`), then you fill it in.

### 5. Present the plan, then confirm
Summarise: candidates to add (new/append), and — flagged clearly — any **supersede/refresh** with the doc and the reason each existing claim is no longer true. Get the operator's go-ahead before executing supersede/refresh. Proceed with plain adds without waiting unless the operator asked to review everything.

### 6. Execute (ctx commands only)

New concern:
```bash
ctx new <name> --non-interactive \
  --concerns <concern-name> \
  --paths "backend/app/domain/reconciliation/**,docs/design/<file>.md" \
  --components reconciliation,trust \
  --rank 4 \
  --text "$(cat <<'EOF'
CLAIM: <one-line current claim>.
WHY: <why it is true, grounded in the code/decision>.
DEPENDS ON: <what it rests on>. EXCLUDES: <what it does not cover>.
Related: [[other-concern]], [[another-concern]].
SUPERSEDE WHEN: <the condition that makes this stale>.
EOF
)"
```

Append to an existing concern (get `<ID>` from `ctx list`):
```bash
ctx append <ID> --concern <concern-name> --rank 3 --text "$(cat <<'EOF'
<additive note, same claim/why/supersede-when discipline>
EOF
)"
```

Supersede — author the replacement first (`ctx new …`), then record the pointer:
```bash
ctx supersede <OLD_ID> --by <NEW_ID> --concerns <concern-name>
```

Refresh a stale concern into a successor doc:
```bash
ctx refresh --concern <concern-name> --name <new-doc-name> --draft-body
# then fill the successor via ctx append (do not hand-edit)
```

Rank guide: `r5` load-bearing/normative, `r4` important domain claim, `r3` useful detail, `r1–2` supporting. Cross-link related concerns with `[[concern-name]]`.

### 7. Validate and report
```bash
ctx check --strict     # validate corpus + staged changes; fix what it flags
ctx sync               # only if the registry drifted from disk
ctx gc                 # surface fully-superseded docs as cleanup candidates (report, don't delete unprompted)
```
Report: concerns added, concerns superseded/refreshed and why, any `ctx check` warnings resolved, and any `gc` candidates for the operator to consider.

## Smell test before finishing
- Did I **verify** every recorded claim against the repo, or did I record something the conversation only *intended*?
- Does every new/changed concern carry a **SUPERSEDE WHEN:**?
- After my changes, does `ctx assemble` over the affected paths read as one consistent story — no doc still asserting what I just superseded?
- Did I make every change through `ctx` (no hand-edits to `.context/`)?
- Did I get confirmation before superseding or refreshing?
