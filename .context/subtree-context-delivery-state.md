---
id: ctx-74fe09
created: 2026-06-27T12:39:42.083278135Z
status: current
concerns:
- local-install-state
- subtree-context-delivery-state
scope:
  paths:
  - README.md
  - src/cli.rs
  - src/commands/assemble.rs
  - src/commands/sync.rs
  - src/ignore.rs
  - src/registry.rs
  - src/subtree.rs
  components:
  - ctx-cli
superseded_by: []
---
### subtree-context-delivery-state [r4]

The cascading subtree-context feature is implemented in the current repo state. The key user-facing surface is ctx assemble --scope current|subtree plus ctx sync --cascade, with .contextrc globs excluding directories from subtree traversal and parent synthesis. Verification completed with cargo test passing and ctx check clean during the implementation session, and the implementation commit was recorded as cf42c11 and pushed to origin/master.

### local-install-state [r3]

A locally installed ctx binary is present at /home/harry/.cargo/bin/ctx, and its help surface currently exposes both ctx assemble --scope subtree and ctx sync --cascade. The installed binary therefore matches the pushed cascading subtree-context implementation rather than an older local build.
