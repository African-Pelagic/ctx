use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "ctx")]
#[command(about = "Context management for workflow-aware engineering notes")]
#[command(
    long_about = "Manage workflow context as markdown documents with explicit concerns, scope, and supersession. ctx is designed for both humans and AI agents working on evolving engineering tasks."
)]
pub struct Cli {
    #[arg(long, global = true, help = "Emit structured JSON output")]
    pub json: bool,

    #[arg(
        long,
        global = true,
        help = "Emit stable plain-text output for scripts"
    )]
    pub porcelain: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Initialize a .context corpus and registry in the current repository")]
    Init,
    #[command(about = "Create a new context document")]
    New(NewArgs),
    #[command(about = "Build or refresh the derived code index")]
    Index,
    #[command(about = "List active concerns, owners, and roster notes")]
    List,
    #[command(about = "Suggest likely relevant context from the derived code index")]
    Suggest(SuggestArgs),
    #[command(about = "Append body text to an existing document under an active concern")]
    Append(AppendArgs),
    #[command(about = "Assemble the subset of context relevant to explicit predicates")]
    Assemble(AssembleArgs),
    #[command(about = "Record concern-level supersession from one document to another")]
    Supersede(SupersedeArgs),
    #[command(about = "Rebuild the registry from .context markdown documents")]
    Sync,
    #[command(about = "Validate the context corpus and staged context changes")]
    Check(CheckArgs),
    #[command(about = "List fully superseded documents as cleanup candidates")]
    Gc,
}

#[derive(Debug, Args)]
pub struct NewArgs {
    #[arg(help = "Document name; .md is optional and will be stripped")]
    pub name: String,

    #[arg(
        long,
        help = "Disable prompts and require all needed metadata as flags"
    )]
    pub non_interactive: bool,

    #[arg(
        long,
        help = "Allow deliberate additive overlap with existing concern owners"
    )]
    pub append: bool,

    #[arg(
        long,
        value_delimiter = ',',
        help = "Comma-separated concern names owned by this document"
    )]
    pub concerns: Vec<String>,

    #[arg(
        long,
        value_delimiter = ',',
        help = "Comma-separated path globs used for deterministic assembly"
    )]
    pub paths: Vec<String>,

    #[arg(
        long,
        value_delimiter = ',',
        help = "Comma-separated component labels used for deterministic assembly"
    )]
    pub components: Vec<String>,
}

#[derive(Debug, Args)]
pub struct AppendArgs {
    #[arg(help = "Document ID to update")]
    pub id: String,

    #[arg(
        long,
        help = "Active concern in the target document that this note belongs under"
    )]
    pub concern: String,

    #[arg(long, help = "Text to append to the document body")]
    pub text: String,
}

#[derive(Debug, Args)]
pub struct AssembleArgs {
    #[arg(
        long,
        help = "Match documents whose scope.paths overlap this path pattern"
    )]
    pub path: Option<String>,

    #[arg(long, help = "Match documents that declare this component")]
    pub component: Option<String>,

    #[arg(
        long,
        value_delimiter = ',',
        help = "Match documents that currently own any of these concerns"
    )]
    pub concern: Vec<String>,

    #[arg(long = "paths", help = "Emit only matching document paths")]
    pub paths_only: bool,
}

#[derive(Debug, Args)]
pub struct SuggestArgs {
    #[arg(
        long,
        help = "Return documents whose scoped paths are likely relevant to this repo path"
    )]
    pub path: Option<String>,
}

#[derive(Debug, Args)]
pub struct SupersedeArgs {
    #[arg(help = "Source document ID whose concern ownership is being replaced")]
    pub id: String,

    #[arg(
        long,
        value_delimiter = ',',
        help = "Comma-separated concerns to supersede on the source document"
    )]
    pub concerns: Vec<String>,

    #[arg(
        long = "by",
        help = "Replacement document ID that becomes the new owner"
    )]
    pub by_id: String,
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    #[arg(long, help = "Escalate warning-class issues to errors")]
    pub strict: bool,
}
