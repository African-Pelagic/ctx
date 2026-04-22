use anyhow::Result;

use crate::{cli::AssembleArgs, output::OutputMode};

pub fn run(_args: AssembleArgs, _output_mode: OutputMode) -> Result<()> {
    println!("ctx assemble is not implemented yet");
    Ok(())
}
