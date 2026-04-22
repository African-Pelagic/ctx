use anyhow::Result;
use serde::Serialize;

use crate::{document::Status, output::OutputMode, registry::load_or_sync};

#[derive(Debug, Serialize)]
struct GcEntry {
    id: String,
    file: String,
}

pub fn run(output_mode: OutputMode) -> Result<()> {
    let registry = load_or_sync()?;
    let mut entries = registry
        .documents
        .iter()
        .filter(|(_, entry)| entry.status == Status::Superseded)
        .map(|(id, entry)| GcEntry {
            id: id.clone(),
            file: entry.file.clone(),
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| a.file.cmp(&b.file));

    match output_mode {
        OutputMode::Human => {
            if entries.is_empty() {
                println!("No fully superseded documents.");
            } else {
                println!("ID\tFile");
                for entry in &entries {
                    println!("{}\t{}", entry.id, entry.file);
                }
            }
        }
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&entries)?);
        }
        OutputMode::Porcelain => {
            for entry in &entries {
                println!("{}\t{}", entry.id, entry.file);
            }
        }
    }

    Ok(())
}
