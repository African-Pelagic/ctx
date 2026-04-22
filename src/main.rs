mod cli;
mod commands;
mod document;
mod git;
mod id;
mod ignore;
mod index;
mod output;
mod registry;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();
    let output_mode = match output::OutputMode::from_flags(cli.json, cli.porcelain) {
        Ok(mode) => mode,
        Err(err) => output::print_error_and_exit(output::OutputMode::Human, &err),
    };

    if let Err(err) = commands::run(cli.command, output_mode) {
        output::print_error_and_exit(output_mode, &err);
    }
}
