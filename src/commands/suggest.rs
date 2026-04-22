use anyhow::{Result, bail};
use glob::Pattern;
use serde::Serialize;

use crate::{
    cli::SuggestArgs,
    index::{IndexedDocument, load_or_build_index_from},
    output::OutputMode,
};

#[derive(Debug, Serialize)]
struct Suggestion {
    id: String,
    file: String,
    active_concerns: Vec<String>,
    matched_paths: Vec<String>,
    reasons: Vec<String>,
}

pub fn run(args: SuggestArgs, output_mode: OutputMode) -> Result<()> {
    let path = args
        .path
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("--path is required"))?;
    let index = load_or_build_index_from(std::path::Path::new("."))?;
    let suggestions = suggest_for_path(path, &index.documents)?;

    match output_mode {
        OutputMode::Human => {
            if suggestions.is_empty() {
                println!("No suggested context documents.");
            } else {
                for suggestion in &suggestions {
                    println!("# {} - {}", suggestion.id, suggestion.file);
                    println!("Active concerns: {}", suggestion.active_concerns.join(", "));
                    println!("Reasons: {}", suggestion.reasons.join(", "));
                    if !suggestion.matched_paths.is_empty() {
                        println!("Matched paths: {}", suggestion.matched_paths.join(", "));
                    }
                    println!();
                }
            }
        }
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&suggestions)?);
        }
        OutputMode::Porcelain => {
            for suggestion in &suggestions {
                println!(
                    "{}\t{}\t{}\t{}",
                    suggestion.id,
                    suggestion.file,
                    suggestion.active_concerns.join(","),
                    suggestion.reasons.join(",")
                );
            }
        }
    }

    Ok(())
}

fn suggest_for_path(
    query: &str,
    documents: &std::collections::BTreeMap<String, IndexedDocument>,
) -> Result<Vec<Suggestion>> {
    if query.trim().is_empty() {
        bail!("--path must not be empty");
    }

    let mut suggestions = Vec::new();
    let query = normalize_path(query);
    let query_prefix = format!("{query}/");

    for (id, document) in documents {
        let mut reasons = Vec::new();
        let mut matched_paths = document
            .matched_repo_paths
            .iter()
            .filter(|path| *path == &query || path.starts_with(&query_prefix))
            .cloned()
            .collect::<Vec<_>>();

        if !matched_paths.is_empty() {
            reasons.push("indexed-path-overlap".to_string());
        }

        let mut matched_scope_patterns = Vec::new();
        for scope_path in &document.scope_paths {
            let pattern = Pattern::new(scope_path)
                .map_err(|_| anyhow::anyhow!("invalid scope pattern in index: {scope_path}"))?;
            if pattern.matches(&query) {
                matched_scope_patterns.push(scope_path.clone());
            }
        }

        if !matched_scope_patterns.is_empty() {
            reasons.push("scope-pattern-match".to_string());
        }

        matched_paths.extend(matched_scope_patterns);
        matched_paths.sort();
        matched_paths.dedup();

        if reasons.is_empty() {
            continue;
        }

        suggestions.push(Suggestion {
            id: id.clone(),
            file: document.file.clone(),
            active_concerns: document.active_concerns.clone(),
            matched_paths,
            reasons,
        });
    }

    suggestions.sort_by(|a, b| {
        b.reasons
            .len()
            .cmp(&a.reasons.len())
            .then(a.file.cmp(&b.file))
    });
    Ok(suggestions)
}

fn normalize_path(path: &str) -> String {
    path.trim().trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::suggest_for_path;
    use crate::index::IndexedDocument;

    #[test]
    fn suggests_documents_for_indexed_path_overlap() {
        let documents = [(
            "ctx-1".to_string(),
            IndexedDocument {
                file: ".context/note.md".into(),
                active_concerns: vec!["billing".into()],
                components: vec!["ctx-cli".into()],
                scope_paths: vec!["src/**".into()],
                matched_repo_paths: vec!["src/main.rs".into(), "src/cli.rs".into()],
                missing_scope_paths: vec![],
                last_updated: None,
                commits_in_scope_since_update: None,
            },
        )]
        .into_iter()
        .collect::<BTreeMap<_, _>>();

        let suggestions = suggest_for_path("src", &documents).unwrap();
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].id, "ctx-1");
    }
}
