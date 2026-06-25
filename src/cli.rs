use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};

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
    #[command(about = "Explain how agents and humans should use ctx in this repo")]
    Guidance(GuidanceArgs),
    #[command(about = "Search context document bodies with active-only defaults")]
    Search(SearchArgs),
    #[command(about = "Suggest likely relevant context from the derived code index")]
    Suggest(SuggestArgs),
    #[command(about = "Append body text to an existing document under an active concern")]
    Append(AppendArgs),
    #[command(about = "Backfill rank metadata into existing context headings")]
    BackfillRanks(BackfillRanksArgs),
    #[command(about = "Assemble active context, optionally narrowed by explicit predicates")]
    Assemble(AssembleArgs),
    #[command(about = "Record concern-level supersession from one document to another")]
    Supersede(SupersedeArgs),
    #[command(about = "Create a successor document for a stale concern")]
    Refresh(RefreshArgs),
    #[command(about = "Rebuild registries and optionally synthesize child context up the tree")]
    Sync(SyncArgs),
    #[command(about = "Validate the context corpus and staged context changes")]
    Check(CheckArgs),
    #[command(about = "List fully superseded documents as cleanup candidates")]
    Gc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum AssembleScope {
    Current,
    Subtree,
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

    #[arg(long, help = "Initial text to write into the new document body")]
    pub text: Option<String>,

    #[arg(
        long,
        value_parser = clap::value_parser!(u8).range(1..=5),
        help = "Context rank for newly added text, from 1 to 5"
    )]
    pub rank: Option<u8>,
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

    #[arg(
        long,
        value_parser = clap::value_parser!(u8).range(1..=5),
        help = "Context rank for this appended text, from 1 to 5"
    )]
    pub rank: u8,
}

#[derive(Debug, Args)]
pub struct BackfillRanksArgs {
    #[arg(
        long,
        value_parser = clap::value_parser!(u8).range(1..=5),
        help = "Default rank to assign to headings that do not already have one"
    )]
    pub default_rank: u8,
}

#[derive(Debug, Args)]
pub struct AssembleArgs {
    #[arg(
        long,
        action = clap::ArgAction::Append,
        help = "Match documents whose scope.paths overlap this path pattern; repeat to include multiple paths"
    )]
    pub path: Vec<String>,

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

    #[arg(long, help = "Explain why each assembled document was included")]
    pub explain: bool,

    #[arg(
        long,
        value_enum,
        default_value_t = AssembleScope::Current,
        help = "Assemble only this level's corpus or the full descendant subtree"
    )]
    pub scope: AssembleScope,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    #[arg(
        long,
        help = "Search for this literal string in context document bodies"
    )]
    pub query: String,

    #[arg(long, help = "Include fully superseded documents in the search corpus")]
    pub include_superseded: bool,
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
pub struct GuidanceArgs {
    #[arg(
        long,
        help = "Update any AGENTS.md files in the repo with ctx usage instructions"
    )]
    pub add: bool,
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
pub struct RefreshArgs {
    #[arg(long, help = "Concern to refresh")]
    pub concern: String,

    #[arg(
        long,
        help = "Source document ID when the concern has multiple active owners"
    )]
    pub from: Option<String>,

    #[arg(long, help = "New document name; .md is optional and will be stripped")]
    pub name: String,

    #[arg(
        long,
        help = "Seed the new document body with the old concern section as a draft"
    )]
    pub draft_body: bool,
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    #[arg(long, help = "Escalate warning-class issues to errors")]
    pub strict: bool,
}

#[derive(Debug, Args)]
pub struct SyncArgs {
    #[arg(
        long,
        help = "Recursively synthesize child .context corpora into parent concern entries before syncing registries"
    )]
    pub cascade: bool,
}

pub fn render_help_dump() -> String {
    let mut sections = Vec::new();
    sections.push(render_help_for(Cli::command()));

    for subcommand in [
        "init",
        "new",
        "index",
        "list",
        "guidance",
        "search",
        "suggest",
        "append",
        "backfill-ranks",
        "assemble",
        "supersede",
        "refresh",
        "sync",
        "check",
        "gc",
    ] {
        let mut command = Cli::command();
        let sub = command
            .find_subcommand_mut(subcommand)
            .expect("declared subcommand must exist")
            .clone();
        sections.push(render_help_for(sub));
    }

    sections.join("\n\n")
}

fn render_help_for(mut command: clap::Command) -> String {
    let title = if command.get_name() == "ctx" {
        "# ctx --help".to_string()
    } else {
        format!("# ctx {} --help", command.get_name())
    };

    let mut bytes = Vec::new();
    command
        .write_long_help(&mut bytes)
        .expect("writing help into memory should not fail");
    let mut help = String::from_utf8(bytes).expect("clap help should be valid UTF-8");
    while help.ends_with('\n') {
        help.pop();
    }

    format!("{title}\n\n```text\n{help}\n```")
}
