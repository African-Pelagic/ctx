---
id: ctx-2f2ead
created: 2026-10-28T17:01:46.003835Z
status: current
concerns:
- reference-sources
scope:
  paths:
  - src/bm25.rs
  - src/commands/source.rs
  - src/source.rs
  components: []
superseded_by: []
---
### reference-sources [r4]

ctx can import large authoritative texts as "sources" and retrieve them by chunk. `ctx source add <file> [--name] [--domain]` stores the text and a meta.json (sha256, origin, domain) under `.context/sources/<name>/`; `ctx source query --query ... [--source] [--domain] [--limit]` and the MCP tool `ctx_source_query` return BM25-ranked chunks with source, heading path and line range for citation.

Why: the operator wants agents guided by authoritative bodies of text (first domains: private equity knowledge, software engineering principles). A concept graph was considered and rejected for now; retrieval over chunks answers the stated need with far less machinery, and the operator said a graph is not required.

Design constraints:
- Sources are derived retrieval data, not claims. They never own concerns and never take part in supersession. A new edition of a source is a new source name.
- Retrieval is deterministic and dependency-free: chunks and BM25 scores are recomputed from stored text on each query (src/bm25.rs, src/source.rs), so there is no index to go stale. Same corpus and query give the same ranking; ties break on chunk order.
- Chunking splits on markdown (#) and org (*) headings, ignores headings inside code fences, packs paragraphs up to about 300 words, and windows oversized paragraphs. The heading path is searchable text.
- Tokenisation is lexical only (lowercase, stopwords, crude plural strip). No synonym or semantic matching: "leverage" and "leveraged" are different terms.
- The domain tag (pe, se, ...) scopes queries and is the intended defence against homonyms ("leverage" in PE vs software) instead of automatic entity resolution.
- `.context/sources/` is outside the registry's `.context/*.md` glob, so sources do not appear in list, assemble, search or check.

Excludes / not built: term or concept extraction, LLM-assisted extraction, embeddings, a concept graph, PDF or URL ingestion (UTF-8 text only), a removal command, and a persisted index. The intended extension point is extra derived files beside source.txt and meta.json (for example terms.json), generated at ingest time so query time stays deterministic. LLM output, if added, must be stored and reviewable, never computed at read time.

Open conflict: this widens scope beyond /Users/harryjohnson/code/ctx/non-goals.org and /Users/harryjohnson/code/ctx/workflow-context-scope.org, which call ctx a workflow-context tool and not a knowledge base. The operator chose to build inside ctx; those concerns have not yet been superseded or amended.

Open question: large source bodies in git. Whether to commit source.txt or gitignore it and commit only meta.json is undecided.

SUPERSEDE WHEN: sources gain extracted terms or a graph, retrieval stops being recomputed per query, sources are stored outside `.context/sources/`, or the workflow-scope concerns are amended to cover reference material.
