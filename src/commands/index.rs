use anyhow::Result;
use serde::Serialize;

use crate::{
    index::{index_path, refresh_index_from},
    output::OutputMode,
};

#[derive(Debug, Serialize)]
struct IndexSummary {
    file: String,
    documents: usize,
    repo_files: usize,
}

pub fn run(output_mode: OutputMode) -> Result<()> {
    let index = refresh_index_from(std::path::Path::new("."))?;
    let summary = IndexSummary {
        file: index_path().to_string_lossy().into_owned(),
        documents: index.documents.len(),
        repo_files: index.repo_files.len(),
    };

    match output_mode {
        OutputMode::Human => {
            println!(
                "Indexed {} documents across {} repo files into {}",
                summary.documents, summary.repo_files, summary.file
            );
        }
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        OutputMode::Porcelain => {
            println!(
                "{}\t{}\t{}",
                summary.file, summary.documents, summary.repo_files
            );
        }
    }

    Ok(())
}
