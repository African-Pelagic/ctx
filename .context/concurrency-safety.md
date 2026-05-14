---
id: ctx-bafb28
created: 2026-04-30T13:08:22.656937627Z
status: current
concerns:
- agent-retry-behavior
- concurrency-safety
scope:
  paths:
  - src/cli.rs
  - src/commands/*.rs
  - src/index.rs
  - src/registry.rs
  components:
  - ctx-cli
superseded_by: []
---
### concurrency-safety

The planned concurrency model for ctx is a single repo-wide write lock around all mutating commands, with read commands remaining lock-free by default. The lock should cover ctx new, append, supersede, and future refresh-style commands. Inside the critical section, the command should reload fresh corpus state, validate again under the lock, write any managed markdown documents atomically, rebuild derived files such as .context/.registry.json and .context/.index.json atomically, and then release the lock.

This approach is preferred because ctx is a semantic coordination layer rather than a high-throughput datastore. Correctness and deterministic behavior matter more than maximizing parallel writes. The current codebase uses read/modify/write flows plus full sync steps, so a coarse writer lock fits the existing architecture with less risk than finer-grained locking.

Derived files should be treated as caches, not transactional truth. Commands must not assume the registry or index is authoritative without revalidation inside the locked section. Atomic write helpers should be used for all managed files to prevent torn writes and partial updates.

This model is intended to prevent lost updates, overlapping writers silently clobbering each other, and readers observing half-written derived state. It does not by itself resolve stale semantic decisions made from old reads, so it should be paired with optimistic concurrency checks such as document version tokens and a corpus revision token.

This concern would be superseded if ctx adopts a materially different write architecture, such as finer-grained locking by concern or document, a long-lived daemon that serializes mutations, or an append-only operation log with a separate applier.

### agent-retry-behavior

Agent-facing write behavior should distinguish transient lock contention from semantic conflicts. ctx should support short bounded waiting for the repo-wide write lock, using jittered backoff and a configurable lock timeout. If lock acquisition still fails after that bounded wait, the command should return a structured lock-busy error that tells the caller the operation is retryable.

Semantic conflicts should not be retried blindly. If a write fails because the expected document version, corpus revision, or concern ownership changed, the agent should re-read relevant context, recompute whether the intended mutation is still correct, and only then decide whether to try again. This preserves semantic safety instead of replaying stale intent against newer state.

The tool should expose machine-readable error kinds for non-interactive workflows, including lock_busy and stale or ownership-changed variants, and should emit version tokens in machine-readable read outputs so agents can carry explicit preconditions into later writes.

The intended retry policy is narrow: retry lock contention a small number of times, but do not automatically retry stale semantic conflicts. If contention persists, the agent should report that it is blocked on another writer rather than looping indefinitely.

This concern would be superseded if the agent interaction model changes substantially, for example if ctx moves from direct CLI writes to a daemon or queue-based mutation service with different admission and retry semantics.
