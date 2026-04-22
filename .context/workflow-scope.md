---
id: ctx-f9c224
created: 2026-04-22T17:55:15.118079764Z
status: current
concerns:
- deterministic-assembly
- non-goals
- workflow-context-scope
scope:
  paths:
  - context-document-management.md
  - context-management-solution.md
  - src/**
  - workflow-context-scope.md
  components:
  - ctx-cli
superseded_by: []
---
### workflow-context-scope

This repo treats ctx as a workflow-context tool rather than a general knowledge base. In-scope material is short-to-medium-lived engineering context that helps an agent or human modify code correctly now. The key fit test is whether the claim is local enough to implementation work and likely to need explicit supersession later.

### deterministic-assembly

Assembly is intentionally deterministic for now. Relevant context is selected from YAML frontmatter using declared concerns, scoped paths, scoped components, and supersession state; semantic or LLM-based relevance ranking is explicitly deferred so the system remains inspectable and predictable.

### non-goals

Out of scope are durable reference artifacts and work-organization objects: company mission, product vision, roadmap narratives, tickets, epics, feature flags as coordination primitives, broad architectural styles, code conventions, and general onboarding material. Those may be linked from workflow context, but ctx is not meant to own them.
