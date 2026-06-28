use std::path::Path;

use anyhow::Result;
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::wrapper::Parameters,
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::Deserialize;

use crate::cli::{AppendArgs, AssembleArgs, AssembleScope, NewArgs, SearchArgs, SupersedeArgs, SuggestArgs};

// ── Parameter types ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct AssembleParams {
    /// File path glob to narrow assembly (optional)
    path: Option<String>,
    /// Concern name to narrow assembly (optional)
    concern: Option<String>,
    /// Component label to narrow assembly (optional)
    component: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct NewDocParams {
    /// Document name (slug, .md suffix is optional)
    name: String,
    /// Comma-separated concern names this document will own
    concerns: String,
    /// Initial body text for the document
    text: String,
    /// Context rank 1–5 (2 is a sensible default)
    rank: u8,
    /// Optional comma-separated path globs for deterministic assembly
    paths: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct AppendParams {
    /// Document ID to append to (e.g. ctx-9ea666)
    id: String,
    /// Active concern in the document to append under
    concern: String,
    /// Text to append
    text: String,
    /// Context rank 1–5
    rank: u8,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchParams {
    /// Literal string to search for across active context documents
    query: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SuggestParams {
    /// Repo-relative path to suggest relevant context documents for
    path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SupersedeParams {
    /// Source document ID whose concerns are being replaced
    source_id: String,
    /// Replacement document ID that becomes the new owner
    by_id: String,
    /// Comma-separated concerns to supersede (all active concerns if omitted)
    concerns: Option<String>,
}

// ── MCP server ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct CtxServer;

#[tool_router]
impl CtxServer {
    #[tool(description = "Assemble active context from the .context/ corpus. Optionally narrow by path glob, concern name, or component label. Run this before starting work on any area of the codebase.")]
    fn ctx_assemble(&self, Parameters(p): Parameters<AssembleParams>) -> Result<String, String> {
        let args = AssembleArgs {
            path: p.path.into_iter().collect(),
            component: p.component,
            concern: p.concern.into_iter().collect(),
            paths_only: false,
            explain: false,
            scope: AssembleScope::Current,
        };
        super::assemble::collect(&args)
            .and_then(|docs| Ok(serde_json::to_string_pretty(&docs)?))
            .map_err(|e| e.to_string())
    }

    #[tool(description = "Create a new context document owning one or more concerns. Use when recording a new decision, constraint, or architectural claim.")]
    fn ctx_new(&self, Parameters(p): Parameters<NewDocParams>) -> Result<String, String> {
        let args = NewArgs {
            name: p.name,
            concerns: split_csv(&p.concerns),
            paths: p.paths.as_deref().map(split_csv).unwrap_or_default(),
            components: vec![],
            text: Some(p.text),
            rank: Some(p.rank),
            non_interactive: true,
            append: false,
        };
        super::new::create(args).map_err(|e| e.to_string())
    }

    #[tool(description = "Append body text to an existing context document under an active concern. Use when the document still owns the concern and you are adding detail, not changing ownership.")]
    fn ctx_append(&self, Parameters(p): Parameters<AppendParams>) -> Result<String, String> {
        let args = AppendArgs {
            id: p.id,
            concern: p.concern,
            text: p.text,
            rank: p.rank,
        };
        super::append::append(args).map_err(|e| e.to_string())
    }

    #[tool(description = "List all active concerns, their owner documents, and any notes (stale, multi-owned).")]
    fn ctx_list(&self) -> Result<String, String> {
        super::list::collect(crate::output::OutputMode::Json)
            .and_then(|v| Ok(serde_json::to_string_pretty(&v)?))
            .map_err(|e| e.to_string())
    }

    #[tool(description = "Search active context document bodies for a literal string.")]
    fn ctx_search(&self, Parameters(p): Parameters<SearchParams>) -> Result<String, String> {
        let args = SearchArgs {
            query: p.query,
            include_superseded: false,
        };
        super::search::collect(&args)
            .and_then(|r| Ok(serde_json::to_string_pretty(&r)?))
            .map_err(|e| e.to_string())
    }

    #[tool(description = "Suggest relevant context documents for a given repo-relative path using the code index.")]
    fn ctx_suggest(&self, Parameters(p): Parameters<SuggestParams>) -> Result<String, String> {
        let args = SuggestArgs { path: Some(p.path) };
        super::suggest::collect(&args)
            .and_then(|s| Ok(serde_json::to_string_pretty(&s)?))
            .map_err(|e| e.to_string())
    }

    #[tool(description = "Validate the context corpus for errors, conflicts, and inconsistencies. Run after making context changes.")]
    fn ctx_check(&self) -> Result<String, String> {
        super::check::collect(Path::new("."), false)
            .and_then(|issues| Ok(serde_json::to_string_pretty(&issues)?))
            .map_err(|e| e.to_string())
    }

    #[tool(description = "Record that one context document supersedes another for specific concerns. Use when an older operational claim is no longer current.")]
    fn ctx_supersede(&self, Parameters(p): Parameters<SupersedeParams>) -> Result<String, String> {
        let args = SupersedeArgs {
            id: p.source_id,
            by_id: p.by_id,
            concerns: p.concerns.as_deref().map(split_csv).unwrap_or_default(),
        };
        super::supersede::supersede(args).map_err(|e| e.to_string())
    }
}

#[tool_handler(
    name = "ctx",
    instructions = "ctx manages workflow context as versioned markdown documents. \
        Use ctx_assemble before starting work, ctx_new or ctx_append to record decisions, \
        ctx_supersede when an older claim is no longer current, and ctx_check after changes."
)]
impl ServerHandler for CtxServer {}

// ── Entry point ────────────────────────────────────────────────────────────────

pub fn run() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let server = CtxServer.serve(stdio()).await?;
            server.waiting().await?;
            Ok(())
        })
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|p| p.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}
