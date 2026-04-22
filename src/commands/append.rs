use anyhow::Result;

use crate::{cli::AppendArgs, output::OutputMode};

pub fn run(_args: AppendArgs, _output_mode: OutputMode) -> Result<()> {
    println!("ctx append is not implemented yet");
    Ok(())
}
