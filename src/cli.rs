use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "ctx")]
#[command(about = "Context management for workflow-aware engineering notes")]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long, global = true)]
    pub porcelain: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init,
    New(NewArgs),
    List,
    Append(AppendArgs),
    Assemble(AssembleArgs),
    Supersede(SupersedeArgs),
    Sync,
    Check(CheckArgs),
    Gc,
}

#[derive(Debug, Args)]
pub struct NewArgs {
    pub name: String,

    #[arg(long)]
    pub non_interactive: bool,

    #[arg(long)]
    pub append: bool,

    #[arg(long, value_delimiter = ',')]
    pub concerns: Vec<String>,

    #[arg(long, value_delimiter = ',')]
    pub paths: Vec<String>,

    #[arg(long, value_delimiter = ',')]
    pub components: Vec<String>,
}

#[derive(Debug, Args)]
pub struct AppendArgs {
    pub id: String,

    #[arg(long)]
    pub concern: String,

    #[arg(long)]
    pub text: String,
}

#[derive(Debug, Args)]
pub struct AssembleArgs {
    #[arg(long)]
    pub path: Option<String>,

    #[arg(long)]
    pub component: Option<String>,

    #[arg(long)]
    pub concern: Option<String>,

    #[arg(long = "paths")]
    pub paths_only: bool,
}

#[derive(Debug, Args)]
pub struct SupersedeArgs {
    pub id: String,

    #[arg(long, value_delimiter = ',')]
    pub concerns: Vec<String>,

    #[arg(long = "by")]
    pub by_id: String,
}

#[derive(Debug, Args)]
pub struct CheckArgs {
    #[arg(long)]
    pub strict: bool,
}
