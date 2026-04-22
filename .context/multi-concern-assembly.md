---
id: ctx-2d9d07
created: 2026-04-22T20:35:30.066685801Z
status: current
concerns:
- multi-concern-assembly
scope:
  paths:
  - README.md
  - src/cli.rs
  - src/commands/assemble.rs
  components:
  - ctx-cli
superseded_by: []
---
### multi-concern-assembly

ctx assemble now accepts multiple concern predicates using repeated --concern flags or comma-separated values. Concern matching uses OR semantics: a document is included if it matches any requested concern, but it appears only once in the result set. The command also reports matched_concerns so humans and agents can see which requested concerns caused a document to be assembled.
