---
id: ctx-7f4c27
created: 2026-06-27T14:02:00Z
status: current
concerns:
- s3-remote-design
- remote-visibility-model
- remote-cli-surface
- remote-storage-layout
- remote-version-control-semantics
scope:
  paths:
  - src/cli.rs
  - src/commands/mod.rs
  - .contextrc
  - .contextignore
  - src/git.rs
  - src/registry.rs
  - src/ignore.rs
  components:
  - ctx-cli
superseded_by: []
---
### s3-remote-design [r4]

ctx derives its value from git-coupling: every commit has its context, and `ctx
assemble` at any HEAD is meaningful because `.context/` lives beside the code.
Committing `.context/` to a shared or public repository breaks this when context
documents contain secrets, internal architecture decisions, confidentiality-sensitive
assumptions, or material the operator does not want public. The current escape hatch —
`.contextignore` — excludes documents from the managed corpus but still requires the
operator to prevent git from staging them, and it does nothing to preserve history for
excluded documents.

The design goal is a storage backend that: (1) preserves the git-coupling property
(context is queryable by commit SHA); (2) keeps sensitive context private and
access-controlled independently of the code repo; (3) does not require a second git
remote, a daemon, or a separate auth system beyond standard AWS credential chains; and
(4) works without any changes to the local `ctx` workflow for commands that stay
purely local (new, append, supersede, check, assemble, etc.).

#### Why S3 Rather Than a Shadow Git Repo

A shadow bare-git repo on S3 (via a custom git remote helper) would provide richer
history semantics, but it introduces significant complexity: the operator must manage
two git remotes with diverging auth, every push/pull involves git pack negotiation, and
the tooling requires either a compiled git remote helper or GIT_REMOTE_HELPER plumbing.
The benefit over commit-keyed S3 objects is marginal for ctx's use case, where the
version of record is always "the context at commit X" rather than "the diff between two
context versions."

S3 with per-object versioning is simpler, has well-understood access controls (IAM
bucket policies, S3 Object Lock, pre-signed URLs), and maps cleanly onto the existing
registry model.

#### Why Not Commit Everything and Rely on Access Controls

A private git remote with restricted access solves the public-repo problem but not the
monorepo-with-many-readers problem, the "context shouldn't be in git history at all"
problem (once committed, history is hard to purge), or the case where the code repo
lives in a third-party system (GitHub, GitLab) that the operator cannot fully control.
A separate credential scope for context also allows granting context access to agents
without granting code-push access, and vice versa.

#### Relation to Existing Git Workflow

`.context/` currently sits in the repo and is committed. The remote backend is opt-in.
Operators who currently commit all of `.context/` can continue to do so. Operators who
want to keep some or all context private add a remote, gitignore relevant documents,
and use `ctx push`/`ctx pull` instead of `git add .context/`. A hybrid model is
explicitly supported: some documents committed to git (public, architecture-level
notes), others excluded from git but pushed to the S3 remote (sensitive decisions,
secrets references, internal tradeoff notes). The split is controlled by `.contextignore`
and the per-document `visibility` frontmatter field.

The fundamental invariant that every code commit can have its context reconstructed is
preserved: `ctx pull --at <sha>` fetches the bundle pushed at that commit.

### remote-storage-layout [r4]

All objects for a single remote live under a user-configured prefix (the remote URL
path component). The layout within that prefix is:

```
{prefix}/
  snapshots/
    {commit-sha}/
      manifest.json        — snapshot metadata (timestamp, ctx version, document list)
      corpus.tar.gz        — all managed .context/*.md files in the snapshot
      registry.json        — .context/.registry.json at snapshot time
  refs/
    latest                 — plain-text file: SHA of the most recent push
    branches/
      {branch-name}        — plain-text file: SHA of the last push on that branch
```

`manifest.json` schema:

```json
{
  "schema_version": 1,
  "ctx_version": "0.1.0",
  "pushed_at": "2026-06-27T14:02:00Z",
  "commit_sha": "abc123def456...",
  "branch": "main",
  "document_count": 12,
  "documents": [
    { "id": "ctx-7f4c27", "file": ".context/s3-remote-backend.md", "visibility": "remote" }
  ]
}
```

`corpus.tar.gz` contains only documents whose effective visibility is `remote` or
`both`. Documents with `visibility: git` are excluded from the bundle because they are
already in the code repo. Documents with `visibility: local` are excluded from both
git and remote.

Object keys are deterministic: re-pushing the same corpus at the same commit SHA
overwrites the same prefix. This means `ctx push` in CI is safe to re-run on retries.
S3 bucket versioning provides the audit trail for each overwrite.

The `refs/` subtree is updated after a successful snapshot upload. `latest` always
points to the most recently pushed SHA regardless of branch. Branch refs allow `ctx
pull` to resolve context by branch name without knowing the current HEAD SHA — making
fresh-clone hydration trivial.

### remote-visibility-model [r4]

Each context document carries an optional `visibility` frontmatter field:

| Value    | Committed to git | Pushed to remote | Description |
|----------|-----------------|-----------------|-------------|
| `git`    | yes             | no              | Lives in the code repo as today (default) |
| `remote` | no              | yes             | Gitignored locally; stored only in S3 |
| `both`   | yes             | yes             | Redundant storage; useful during migration |
| `local`  | no              | no              | Machine-local only; never leaves the workstation |

Default visibility when the field is absent is `git`. This preserves backward
compatibility — all existing ctx documents continue to behave exactly as before.

`ctx push` includes only documents with `visibility: remote` or `visibility: both`.
`ctx pull` writes only the documents from the bundle; it never replaces documents
already in git. `.contextignore` takes precedence over visibility: a document matching
`.contextignore` is excluded from both push and pull regardless of its visibility
field.

`ctx push` warns if a `remote`-visibility document appears in `git ls-files`, because
that indicates the document is at risk of being committed despite its intent. `ctx
check` surfaces this as a deterministic drift warning.

### remote-cli-surface [r4]

Four new top-level commands extend the CLI. All follow the existing output-mode
pattern (`--json`, `--porcelain`) and the same error-handling conventions as existing
commands.

#### `ctx remote`

Manages named remote configurations in `.contextrc`.

```
ctx remote add <name> <url>      Register a new remote (e.g. s3://my-bucket/my-project)
ctx remote remove <name>         Deregister a remote
ctx remote list                  List configured remotes with their URLs
```

Remote names follow git conventions (`origin` is the conventional default). URLs use
the `s3://` scheme. The bucket and optional path prefix are parsed from the URL.

`.contextrc` gains an optional structured block alongside its existing glob lines:

```toml
[remote "origin"]
url = s3://my-private-ctx-bucket/my-project
credentials = env
```

`credentials` defaults to `env`, meaning standard AWS credential chain resolution
(`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`, `AWS_REGION`,
`~/.aws/credentials`, instance metadata). Explicit key configuration in `.contextrc`
is excluded to prevent secrets in config files. `ctx remote add` warns if bucket
versioning is not enabled (requires `s3:GetBucketVersioning`).

#### `ctx push [remote]`

Uploads a snapshot of the current corpus to the named remote (default: `origin`).

```
ctx push
ctx push origin
ctx push --dry-run         Show what would be uploaded without uploading
```

Execution sequence: (1) resolve HEAD SHA via `git rev-parse HEAD` — fail clearly if
not in a git repo; (2) resolve branch via `git symbolic-ref --short HEAD`; (3) collect
`remote`- and `both`-visibility documents; (4) warn for any `remote`-visibility
document in `git ls-files`; (5) build `manifest.json` and `corpus.tar.gz` in memory;
(6) upload `snapshots/{sha}/manifest.json`, then `corpus.tar.gz`, then
`registry.json`; (7) update `refs/latest` and `refs/branches/{branch}`.

Push is non-destructive: it never deletes existing snapshots. Re-running at the same
SHA is idempotent.

#### `ctx pull [remote] [--at <sha|branch>]`

Downloads context for a specific commit or the latest push.

```
ctx pull
ctx pull origin
ctx pull --at abc123def456
ctx pull --at main
ctx pull --dry-run          Show what would be written without writing
ctx pull --overwrite        Replace local documents that already exist (default: skip)
```

SHA resolution: `--at <sha>` does a prefix lookup in `snapshots/`; `--at <branch>`
reads `refs/branches/{branch}`; no `--at` reads `refs/latest`. If no exact SHA match
exists, `ctx pull` walks backward through `git log --oneline` to find the nearest
ancestor that has a snapshot (fallback ancestor scan). This makes fresh-clone
hydration work with just `git clone <repo> && ctx pull`.

Pull never overwrites `git`-visibility documents. Documents present in git are skipped
with a warning if the bundle contains a version of the same file. After extraction,
`ctx sync` is run automatically to rebuild the local registry.

#### `ctx log [remote]`

Lists available snapshots in the remote store.

```
ctx log
ctx log --limit 20         Default: 50
ctx log --branch main      Filter to pushes on a specific branch
```

Human output: table of short SHA, branch, pushed_at, document count. JSON: array of
manifest objects. Uses `ListObjectsV2` against `snapshots/`; results sorted by
`pushed_at` descending.

#### Implementation Phasing

Phase 1 — configuration and scaffolding: add `[remote "name"]` parsing to `.contextrc`,
add `ctx remote add/remove/list`, add `visibility` field to `Frontmatter` with `git`
default, extend `ctx check` to warn on `remote`-visibility documents tracked by git.
No new crate dependencies.

Phase 2 — push: add `aws-config` + `aws-sdk-s3` (official AWS SDK for Rust) and `tar`
+ `flate2` for bundle packing; implement `ctx push` with the upload sequence above;
implement `refs/` updates.

Phase 3 — pull and log: implement `ctx pull` with SHA resolution, fallback ancestor
scan, bundle extraction, and `ctx sync`; implement `ctx log` with pagination.

Phase 4 — UX polish: update `ctx guidance` and AGENTS.md output to describe the
push/pull workflow; update README with a remote quickstart; add `--dry-run` to both
push and pull.

#### Non-Goals

Multi-region replication, client-side encryption (operators use SSE-KMS via IAM),
non-S3 backends (the storage implementation should be isolated behind a `ContextRemote`
trait to allow future backends, but only S3 ships in this design), context merging on
pull conflicts (the supersession model handles semantic conflicts after pull), git hook
auto-installation (push is always explicit), and storing the full git object database
in S3.

### remote-version-control-semantics [r4]

The version control model is intentionally simpler than git's DAG. The primary query
is "what was the context at commit X?" — answered by a direct key lookup in
`snapshots/{sha}/` with no graph traversal. There is no delta format; every snapshot
is a complete copy of the remote-visibility corpus. Context documents are small
(typically < 100 KB total), so full snapshots are trivially reconstructable without a
base chain, and diffs would add complexity without meaningful storage savings.

Operators push after significant context updates, not necessarily on every commit. The
commit SHA in the snapshot key is the *code* commit at push time, not a separate
context commit. A snapshot therefore represents "the context that was current when
commit X was made," which is the semantically useful claim.

S3 bucket versioning must be enabled on any bucket used as a ctx remote. This provides
recovery from accidental overwrites (re-push idempotency overwrites the same key prefix
but S3 retains previous object versions), an audit trail independent of snapshot
history, and Object Lock compatibility for regulated environments.

In multi-agent or multi-developer workflows, the S3 remote serves as a shared context
bus with eventual consistency semantics: Agent A pushes after updating context; Agent B
pulls before starting work. There is no distributed locking at the remote level.
Conflicts at the document level are resolved by the existing concern-level supersession
model after pull. Remote push and pull are outside the scope of the local write lock
(described in the concurrency-safety concern) because they involve network I/O and must
not block local corpus mutations.
