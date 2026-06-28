<!-- ctx-guidance:start -->
## ctx

ctx guidance

- .context/ is managed by ctx.
- Do not directly edit .context documents except for recovery or repair work.
- Use ctx assemble before relevant work. With no predicates, it assembles the active corpus for the current directory by default; use --scope subtree when nested context corpora matter.
- Prefer ctx assemble --explain when you need the fullest deterministic picture of why documents are in scope.
- ctx assemble accepts repeated --path flags when multiple repo paths matter.
- Use ctx sync --cascade when parent directories should synthesize nearest child .context corpora into child-context concerns.
- Use ctx search or ctx suggest for discovery when explicit assemble predicates are not enough.
- Use ctx new, ctx append, ctx supersede, and ctx refresh for context updates.
- Capture enough detail that a later agent can act without another interview.
- Prefer semantic coverage over verbosity.
- For each concern, try to record: the current claim, why it is true, what it depends on, what it excludes, and what would cause it to be superseded.
- Include decisions, assumptions, constraints, tradeoffs, and concrete examples when they remove ambiguity.
- Do not overfit the context to incidental implementation details that will churn quickly.
- Read assembled context critically, not passively.
- Check for contradictions, unsatisfied prerequisites, stale assumptions, and mismatches between context and code.
- If context is incomplete, inconsistent, or no longer true, update it or supersede it explicitly.
- If the right semantic change is not clear from the code and current context, check with the operator before making the change.
- Run ctx check after context changes.
- Respect .contextignore and .contextrc when deciding what belongs in managed context.
## cli help

# ctx --help

```text
Manage workflow context as markdown documents with explicit concerns, scope, and supersession. ctx is designed for both humans and AI agents working on evolving engineering tasks.

Usage: ctx [OPTIONS] <COMMAND>

Commands:
  init            Initialize a .context corpus and registry in the current repository
  new             Create a new context document
  index           Build or refresh the derived code index
  list            List active concerns, owners, and roster notes
  guidance        Explain how agents and humans should use ctx in this repo
  search          Search context document bodies with active-only defaults
  suggest         Suggest likely relevant context from the derived code index
  append          Append body text to an existing document under an active concern
  backfill-ranks  Backfill rank metadata into existing context headings
  assemble        Assemble active context, optionally narrowed by explicit predicates
  supersede       Record concern-level supersession from one document to another
  refresh         Create a successor document for a stale concern
  sync            Rebuild registries and optionally synthesize child context up the tree
  check           Validate the context corpus and staged context changes
  gc              List fully superseded documents as cleanup candidates
  serve           Start an MCP server exposing ctx tools over stdio
  help            Print this message or the help of the given subcommand(s)

Options:
      --json
          Emit structured JSON output

      --porcelain
          Emit stable plain-text output for scripts

  -h, --help
          Print help (see a summary with '-h')
```

# ctx init --help

```text
Initialize a .context corpus and registry in the current repository

Usage: init

Options:
  -h, --help
          Print help
```

# ctx new --help

```text
Create a new context document

Usage: new [OPTIONS] <NAME>

Arguments:
  <NAME>
          Document name; .md is optional and will be stripped

Options:
      --non-interactive
          Disable prompts and require all needed metadata as flags

      --append
          Allow deliberate additive overlap with existing concern owners

      --concerns <CONCERNS>
          Comma-separated concern names owned by this document

      --paths <PATHS>
          Comma-separated path globs used for deterministic assembly

      --components <COMPONENTS>
          Comma-separated component labels used for deterministic assembly

      --text <TEXT>
          Initial text to write into the new document body

      --rank <RANK>
          Context rank for newly added text, from 1 to 5

  -h, --help
          Print help
```

# ctx index --help

```text
Build or refresh the derived code index

Usage: index

Options:
  -h, --help
          Print help
```

# ctx list --help

```text
List active concerns, owners, and roster notes

Usage: list

Options:
  -h, --help
          Print help
```

# ctx guidance --help

```text
Explain how agents and humans should use ctx in this repo

Usage: guidance [OPTIONS]

Options:
      --add
          Update any AGENTS.md files in the repo with ctx usage instructions

  -h, --help
          Print help
```

# ctx search --help

```text
Search context document bodies with active-only defaults

Usage: search [OPTIONS] --query <QUERY>

Options:
      --query <QUERY>
          Search for this literal string in context document bodies

      --include-superseded
          Include fully superseded documents in the search corpus

  -h, --help
          Print help
```

# ctx suggest --help

```text
Suggest likely relevant context from the derived code index

Usage: suggest [OPTIONS]

Options:
      --path <PATH>
          Return documents whose scoped paths are likely relevant to this repo path

  -h, --help
          Print help
```

# ctx append --help

```text
Append body text to an existing document under an active concern

Usage: append --concern <CONCERN> --text <TEXT> --rank <RANK> <ID>

Arguments:
  <ID>
          Document ID to update

Options:
      --concern <CONCERN>
          Active concern in the target document that this note belongs under

      --text <TEXT>
          Text to append to the document body

      --rank <RANK>
          Context rank for this appended text, from 1 to 5

  -h, --help
          Print help
```

# ctx backfill-ranks --help

```text
Backfill rank metadata into existing context headings

Usage: backfill-ranks --default-rank <DEFAULT_RANK>

Options:
      --default-rank <DEFAULT_RANK>
          Default rank to assign to headings that do not already have one

  -h, --help
          Print help
```

# ctx assemble --help

```text
Assemble active context, optionally narrowed by explicit predicates

Usage: assemble [OPTIONS]

Options:
      --path <PATH>
          Match documents whose scope.paths overlap this path pattern; repeat to include multiple paths

      --component <COMPONENT>
          Match documents that declare this component

      --concern <CONCERN>
          Match documents that currently own any of these concerns

      --paths
          Emit only matching document paths

      --explain
          Explain why each assembled document was included

      --scope <SCOPE>
          Assemble only this level's corpus or the full descendant subtree
          
          [default: current]
          [possible values: current, subtree]

  -h, --help
          Print help
```

# ctx supersede --help

```text
Record concern-level supersession from one document to another

Usage: supersede [OPTIONS] --by <BY_ID> <ID>

Arguments:
  <ID>
          Source document ID whose concern ownership is being replaced

Options:
      --concerns <CONCERNS>
          Comma-separated concerns to supersede on the source document

      --by <BY_ID>
          Replacement document ID that becomes the new owner

  -h, --help
          Print help
```

# ctx refresh --help

```text
Create a successor document for a stale concern

Usage: refresh [OPTIONS] --concern <CONCERN> --name <NAME>

Options:
      --concern <CONCERN>
          Concern to refresh

      --from <FROM>
          Source document ID when the concern has multiple active owners

      --name <NAME>
          New document name; .md is optional and will be stripped

      --draft-body
          Seed the new document body with the old concern section as a draft

  -h, --help
          Print help
```

# ctx sync --help

```text
Rebuild registries and optionally synthesize child context up the tree

Usage: sync [OPTIONS]

Options:
      --cascade
          Recursively synthesize child .context corpora into parent concern entries before syncing registries

  -h, --help
          Print help
```

# ctx check --help

```text
Validate the context corpus and staged context changes

Usage: check [OPTIONS]

Options:
      --strict
          Escalate warning-class issues to errors

  -h, --help
          Print help
```

# ctx gc --help

```text
List fully superseded documents as cleanup candidates

Usage: gc

Options:
  -h, --help
          Print help
```
<!-- ctx-guidance:end -->
