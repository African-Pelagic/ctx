use anyhow::Result;

use crate::{cli::NewArgs, output::OutputMode};

pub fn run(_args: NewArgs, _output_mode: OutputMode) -> Result<()> {
    println!("ctx new is not implemented yet");
    Ok(())
}
