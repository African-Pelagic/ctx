---
id: ctx-c30c19
created: 2026-06-25T11:37:10.076334150Z
status: current
concerns:
- assembly-behavior
- contextignore-security-boundary
- document-lifecycle
- guidance-command
- read-side-commands
scope:
  paths:
  - .contextrc
  - README.md
  - src/cli.rs
  - src/commands/assemble.rs
  - src/commands/guidance.rs
  - src/commands/sync.rs
  - src/ignore.rs
  - src/registry.rs
  - src/subtree.rs
  components:
  - ctx-cli
superseded_by: []
---
### assembly-behavior [r4]

ctx assemble now has explicit scope modes. The default scope is current, which assembles only the current directory's own .context corpus. --scope subtree recursively assembles raw context documents from every descendant .context corpus, rebases child scope paths to the origin directory, and excludes synthesized child-summary documents so subtree reads do not collapse back to parent summaries.

### read-side-commands [r4]

The read surface still centers on list, assemble, gc, search, and suggest, but assemble now has two distinct hierarchy modes. current reads only the local corpus, while subtree walks nested ctx corpora under the working directory and returns the raw active documents from each level. This keeps hierarchical discovery explicit without forcing every caller to rely on synthesized parent summaries.

### contextignore-security-boundary [r3]

The traversal boundary now has two files with distinct roles. .contextignore still excludes managed markdown files, repo paths, and blocked scope paths, while .contextrc excludes directories from recursive subtree traversal. That means nested corpora under excluded globs are skipped by both ctx assemble --scope subtree and ctx sync --cascade, but .contextrc does not redact markdown content on its own.

### document-lifecycle [r4]

The write-side lifecycle now has an optional cascading maintenance step: init -> new/append/supersede/refresh -> sync, with sync --cascade available when nested .context corpora exist. sync --cascade walks the subtree bottom-up, lets each context root synthesize its nearest child context roots into child-context concerns, writes or removes the generated child-summary document at that level, and then rebuilds the local registry.

### guidance-command [r3]

ctx guidance should now tell operators and agents that plain ctx assemble reads only the current level, that --scope subtree is the explicit way to assemble nested corpora, that ctx sync --cascade maintains parent child-context summaries, and that both .contextignore and .contextrc affect what ctx will read or traverse.
