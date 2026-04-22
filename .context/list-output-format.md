---
id: ctx-7b99b7
created: 2026-04-22T20:13:31.231049805Z
status: current
concerns:
- list-output-format
scope:
  paths:
  - src/commands/list.rs
  components:
  - ctx-cli
superseded_by: []
---
### list-output-format

ctx list should present the active concern roster as a readable table for humans, with explicit columns for concern name, owning document ids, source filenames, and notes. The human output should be aligned rather than tab-separated, while JSON and porcelain output should also carry the filenames so agents and scripts can map concerns back to concrete documents without a second lookup.
