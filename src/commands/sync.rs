use anyhow::Result;
use serde_json::json;

use crate::{
    cli::SyncArgs,
    output::OutputMode,
    registry::{Registry, sync_corpus},
};

pub fn run(_args: SyncArgs, output_mode: OutputMode) -> Result<()> {
    let registry = sync_corpus()?;
    emit_sync_result(&registry, output_mode)
}

fn emit_sync_result(registry: &Registry, output_mode: OutputMode) -> Result<()> {
    match output_mode {
        OutputMode::Human => {
            println!(
                "Synced {} context document{}",
                registry.documents.len(),
                if registry.documents.len() == 1 { "" } else { "s" }
            );
        }
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "documents": registry.documents.len(),
                    "registry": ".context/.registry.json"
                }))?
            );
        }
        OutputMode::Porcelain => {
            println!("{}", registry.documents.len());
        }
    }

    Ok(())
}
