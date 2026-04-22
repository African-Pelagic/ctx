use anyhow::Result;

use crate::{cli::SupersedeArgs, output::OutputMode};

pub fn run(_args: SupersedeArgs, _output_mode: OutputMode) -> Result<()> {
    println!("ctx supersede is not implemented yet");
    Ok(())
}
