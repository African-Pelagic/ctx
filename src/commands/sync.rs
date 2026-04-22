use anyhow::Result;
use serde_json::json;

use crate::{output::OutputMode, registry::sync_corpus};

pub fn run(output_mode: OutputMode) -> Result<()> {
    let registry = sync_corpus()?;

    match output_mode {
        OutputMode::Human => {
            println!(
                "Synced {} context document{}",
                registry.documents.len(),
                if registry.documents.len() == 1 {
                    ""
                } else {
                    "s"
                }
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
