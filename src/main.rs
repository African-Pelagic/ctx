mod cli;
mod commands;
mod document;
mod git;
mod id;
mod output;
mod registry;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    let output_mode = output::OutputMode::from_flags(cli.json, cli.porcelain)?;

    commands::run(cli.command, output_mode)
}
