use std::fs;

use anyhow::{Context, Result, bail};
use glob::Pattern;
use serde::Serialize;

use crate::{cli::AssembleArgs, document::Status, output::OutputMode, registry::load_or_sync};

#[derive(Debug, Serialize)]
struct AssembledDocument {
    id: String,
    file: String,
    active_concerns: Vec<String>,
    matched_concerns: Vec<String>,
    content: String,
}

pub fn run(args: AssembleArgs, output_mode: OutputMode) -> Result<()> {
    let registry = load_or_sync()?;
    let docs = select_documents(&registry, &args)?;

    match output_mode {
        OutputMode::Human => {
            if args.paths_only {
                for doc in &docs {
                    println!("{}", doc.file);
                }
            } else {
                for (index, doc) in docs.iter().enumerate() {
                    if index > 0 {
                        println!();
                    }
                    println!("# {} - {}", doc.id, doc.file);
                    println!("Active concerns: {}", doc.active_concerns.join(", "));
                    if !doc.matched_concerns.is_empty() {
                        println!("Matched concerns: {}", doc.matched_concerns.join(", "));
                    }
                    if !doc.content.trim().is_empty() {
                        println!();
                        print!("{}", doc.content);
                        if !doc.content.ends_with('\n') {
                            println!();
                        }
                    }
                }
            }
        }
        OutputMode::Json => {
            if args.paths_only {
                let paths = docs.iter().map(|doc| doc.file.clone()).collect::<Vec<_>>();
                println!("{}", serde_json::to_string_pretty(&paths)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&docs)?);
            }
        }
        OutputMode::Porcelain => {
            if args.paths_only {
                for doc in &docs {
                    println!("{}", doc.file);
                }
            } else {
                for doc in &docs {
                    println!(
                        "{}\t{}\t{}\t{}",
                        doc.id,
                        doc.file,
                        doc.active_concerns.join(","),
                        doc.matched_concerns.join(",")
                    );
                }
            }
        }
    }

    Ok(())
}

fn select_documents(
    registry: &crate::registry::Registry,
    args: &AssembleArgs,
) -> Result<Vec<AssembledDocument>> {
    let has_predicate = args.path.is_some() || args.component.is_some() || !args.concern.is_empty();
    if !has_predicate {
        bail!("at least one of --path, --component, or --concern is required");
    }

    let compiled_path = match &args.path {
        Some(path) => {
            Some(Pattern::new(path).with_context(|| format!("invalid path pattern {path}"))?)
        }
        None => None,
    };

    let mut docs = Vec::new();
    for (id, entry) in &registry.documents {
        if entry.status == Status::Superseded {
            continue;
        }

        let path_match = compiled_path
            .as_ref()
            .map(|pattern| entry.scope.paths.iter().any(|scope| pattern.matches(scope)))
            .unwrap_or(false);

        let component_match = args
            .component
            .as_ref()
            .map(|component| entry.scope.components.iter().any(|item| item == component))
            .unwrap_or(false);

        let mut matched_concerns = args
            .concern
            .iter()
            .filter(|concern| entry.active_concerns.iter().any(|item| item == *concern))
            .cloned()
            .collect::<Vec<_>>();
        matched_concerns.sort();
        matched_concerns.dedup();
        let concern_match = !matched_concerns.is_empty();

        if !(path_match || component_match || concern_match) {
            continue;
        }

        let content = fs::read_to_string(&entry.file)
            .with_context(|| format!("failed to read {}", entry.file))?;
        let body = strip_frontmatter(&content);

        docs.push(AssembledDocument {
            id: id.clone(),
            file: entry.file.clone(),
            active_concerns: entry.active_concerns.clone(),
            matched_concerns,
            content: body,
        });
    }

    docs.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(docs)
}

fn strip_frontmatter(content: &str) -> String {
    if let Some(rest) = content.strip_prefix("---\n") {
        if let Some((_, body)) = rest.split_once("\n---\n") {
            return body.to_string();
        }
    }
    content.to_string()
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::{select_documents, strip_frontmatter};
    use crate::{
        cli::AssembleArgs,
        document::Status,
        registry::{DocumentEntry, Registry},
    };

    #[test]
    fn strips_frontmatter_from_document() {
        let content = "---\nid: ctx-1\n---\nbody\n";
        assert_eq!(strip_frontmatter(content), "body\n");
    }

    #[test]
    fn selects_matching_documents() {
        let registry = Registry {
            schema_version: 1,
            generated_at: Utc::now(),
            generated_from_commit: None,
            documents: [(
                "ctx-a".to_string(),
                DocumentEntry {
                    file: "Cargo.toml".into(),
                    created: Utc::now(),
                    status: Status::Current,
                    concerns: vec!["billing".into()],
                    active_concerns: vec!["billing".into()],
                    scope: crate::document::Scope {
                        paths: vec!["src/billing/**".into()],
                        components: vec!["billing-service".into()],
                    },
                    superseded_by: vec![],
                },
            )]
            .into_iter()
            .collect(),
            concern_roster: Default::default(),
            orphaned_concerns: vec![],
            multi_owned_concerns: vec![],
        };

        let args = AssembleArgs {
            path: None,
            component: Some("billing-service".into()),
            concern: vec![],
            paths_only: false,
        };

        let docs = select_documents(&registry, &args).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, "ctx-a");
        assert!(docs[0].matched_concerns.is_empty());
    }

    #[test]
    fn selects_documents_matching_any_requested_concern() {
        let registry = Registry {
            schema_version: 1,
            generated_at: Utc::now(),
            generated_from_commit: None,
            documents: [
                (
                    "ctx-a".to_string(),
                    DocumentEntry {
                        file: "Cargo.toml".into(),
                        created: Utc::now(),
                        status: Status::Current,
                        concerns: vec!["billing".into()],
                        active_concerns: vec!["billing".into()],
                        scope: crate::document::Scope::default(),
                        superseded_by: vec![],
                    },
                ),
                (
                    "ctx-b".to_string(),
                    DocumentEntry {
                        file: "README.md".into(),
                        created: Utc::now(),
                        status: Status::Current,
                        concerns: vec!["auth".into(), "sessions".into()],
                        active_concerns: vec!["auth".into(), "sessions".into()],
                        scope: crate::document::Scope::default(),
                        superseded_by: vec![],
                    },
                ),
            ]
            .into_iter()
            .collect(),
            concern_roster: Default::default(),
            orphaned_concerns: vec![],
            multi_owned_concerns: vec![],
        };

        let args = AssembleArgs {
            path: None,
            component: None,
            concern: vec!["billing".into(), "sessions".into()],
            paths_only: false,
        };

        let docs = select_documents(&registry, &args).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].id, "ctx-a");
        assert_eq!(docs[0].matched_concerns, vec!["billing".to_string()]);
        assert_eq!(docs[1].id, "ctx-b");
        assert_eq!(docs[1].matched_concerns, vec!["sessions".to_string()]);
    }
}
