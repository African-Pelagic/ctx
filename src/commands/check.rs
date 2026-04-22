use anyhow::Result;

use crate::{cli::CheckArgs, output::OutputMode};

pub fn run(_args: CheckArgs, _output_mode: OutputMode) -> Result<()> {
    println!("ctx check is not implemented yet");
    Ok(())
}
