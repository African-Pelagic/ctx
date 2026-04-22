mod append;
mod assemble;
mod check;
mod gc;
mod index;
mod init;
mod list;
mod new;
mod suggest;
mod supersede;
mod sync;

use anyhow::Result;

use crate::{cli::Command, output::OutputMode};

pub fn run(command: Command, output_mode: OutputMode) -> Result<()> {
    match command {
        Command::Init => init::run(output_mode),
        Command::New(args) => new::run(args, output_mode),
        Command::Index => index::run(output_mode),
        Command::List => list::run(output_mode),
        Command::Suggest(args) => suggest::run(args, output_mode),
        Command::Append(args) => append::run(args, output_mode),
        Command::Assemble(args) => assemble::run(args, output_mode),
        Command::Supersede(args) => supersede::run(args, output_mode),
        Command::Sync => sync::run(output_mode),
        Command::Check(args) => check::run(args, output_mode),
        Command::Gc => gc::run(output_mode),
    }
}
