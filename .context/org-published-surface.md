---
id: ctx-734f2b
created: 2026-06-27T17:33:55.627826765Z
status: superseded
concerns:
- org-published-surface
scope:
  paths: []
  components: []
superseded_by:
- id: ctx-30c448
  concerns:
  - org-published-surface
---
### org-published-surface [r3]

CTX corpora exist within a three-tier knowledge architecture. Capture happens via voice recordings processed by an agent into a conversation, then synthesised into CTX — this is ephemeral and low-friction, with the agent interaction as the capture interface. Working knowledge lives in CTX documents in `.context/` — fluid, evolving, agent-native, semantically structured via concerns, scope, and supersession, and always the source of truth. Published knowledge lives in Org files in `.context/published/` — curated, stable, human-readable exports derived from CTX, intended for humans and sharing contexts where CTX tooling is not present.

The flow is strictly: Voice → Agent → CTX → Org. CTX is always canonical. Org files are derived artifacts and are never hand-edited.

**The published/ directory.** Every CTX corpus may have a `.context/published/` directory. Each file in it is an Org-format rendering of a single concern assembled from the corpus. The filename matches the concern name (e.g., `concern-primary-architecture.org`). The Org file structure includes `#+TITLE:` set to the concern name, a property drawer with corpus metadata (scope paths, components, rank, last-published date), the assembled concern body as Org headings and prose, and `#+FILETAGS:` reflecting concern status (e.g., `:published:`, `:deprecated:`). Only concerns in `published` lifecycle state receive an Org file by default. Active and draft concerns remain CTX-internal. A `--all` flag exports active concerns for local use.

**The ctx publish command.** Intended surface:
- `ctx publish` — regenerate all published/ org files for published-state concerns
- `ctx publish --concern <name>` — publish a single concern
- `ctx publish --all` — include active concerns (local use only)

`ctx check` should warn when a concern is in `published` state but has no corresponding Org file in `published/`, or when an Org file exists for a concern that has since been deprecated or is no longer active. The `published/` directory may be committed to git (making it a human-readable companion to the corpus) or gitignored (keeping it local-only); the corpus owner decides.

**Granularity: one Org file per concern, not per CTX document.** A single CTX document may own multiple concerns; a concern may have history across multiple superseded documents. The concern is the semantic unit, the document is storage. The assembled body is the current concern body only — not supersession history. Git history on the `.md` source files is the audit trail.

**Ingestion of existing Org files.** Pre-existing Org notes may be ingested into CTX via `ctx new` or `ctx append` using the Org content as seed text. This is a one-time or periodic migration, not an ongoing input pipe. The `published/` directory is strictly output-only. Ingestion of arbitrary Org files into CTX is a separate concern from publication.

**What this excludes.** Org files in `published/` are not hand-edited — to change content, update the CTX concern and republish. CTX does not treat Org as a capture format for ongoing input; voice → agent → CTX is the capture path. The published surface is not a replacement for CTX assembly; it is a human-friendly projection of it.
