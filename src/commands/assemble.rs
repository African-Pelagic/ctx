use std::fs;

use anyhow::{Context, Result};
use glob::Pattern;
use serde::Serialize;

use crate::{cli::AssembleArgs, document::Status, output::OutputMode, registry::load_or_sync};

#[derive(Debug, Serialize)]
struct InclusionReason {
    kind: &'static str,
    requested: String,
    matched: String,
}

#[derive(Debug, Serialize)]
struct AssembledDocument {
    id: String,
    file: String,
    active_concerns: Vec<String>,
    matched_concerns: Vec<String>,
    reasons: Vec<InclusionReason>,
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
                    if args.explain && !doc.reasons.is_empty() {
                        println!("Included because: {}", format_reasons(&doc.reasons));
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
                    if args.explain {
                        println!(
                            "{}\t{}\t{}\t{}\t{}",
                            doc.id,
                            doc.file,
                            doc.active_concerns.join(","),
                            doc.matched_concerns.join(","),
                            serde_json::to_string(&doc.reasons)?
                        );
                    } else {
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
    }

    Ok(())
}

fn select_documents(
    registry: &crate::registry::Registry,
    args: &AssembleArgs,
) -> Result<Vec<AssembledDocument>> {
    let compiled_paths = args
        .path
        .iter()
        .map(|path| Pattern::new(path).with_context(|| format!("invalid path pattern {path}")))
        .collect::<Result<Vec<_>>>()?;
    let has_predicate =
        !compiled_paths.is_empty() || args.component.is_some() || !args.concern.is_empty();

    let mut docs = Vec::new();
    for (id, entry) in &registry.documents {
        if entry.status == Status::Superseded {
            continue;
        }

        let mut reasons = Vec::new();

        for (requested_path, pattern) in args.path.iter().zip(compiled_paths.iter()) {
            for scope_path in &entry.scope.paths {
                if pattern.matches(scope_path) {
                    reasons.push(InclusionReason {
                        kind: "path-match",
                        requested: requested_path.clone(),
                        matched: scope_path.clone(),
                    });
                }
            }
        }

        if let Some(component) = &args.component {
            for item in &entry.scope.components {
                if item == component {
                    reasons.push(InclusionReason {
                        kind: "component-match",
                        requested: component.clone(),
                        matched: item.clone(),
                    });
                }
            }
        }

        let mut matched_concerns = args
            .concern
            .iter()
            .filter(|concern| entry.active_concerns.iter().any(|item| item == *concern))
            .cloned()
            .collect::<Vec<_>>();
        matched_concerns.sort();
        matched_concerns.dedup();
        for concern in &matched_concerns {
            reasons.push(InclusionReason {
                kind: "concern-match",
                requested: concern.clone(),
                matched: concern.clone(),
            });
        }

        if !has_predicate {
            reasons.push(InclusionReason {
                kind: "default-active",
                requested: "active-corpus".to_string(),
                matched: "active-corpus".to_string(),
            });
        }

        if reasons.is_empty() {
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
            reasons,
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

fn format_reasons(reasons: &[InclusionReason]) -> String {
    reasons
        .iter()
        .map(|reason| match reason.kind {
            "concern-match" => format!("concern {}", reason.matched),
            "component-match" => format!("component {}", reason.matched),
            "path-match" => format!("path {}", reason.matched),
            "default-active" => "default active corpus".to_string(),
            _ => format!("{} {}", reason.kind, reason.matched),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::{InclusionReason, format_reasons, select_documents, strip_frontmatter};
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
            path: vec![],
            component: Some("billing-service".into()),
            concern: vec![],
            paths_only: false,
            explain: false,
        };

        let docs = select_documents(&registry, &args).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, "ctx-a");
        assert!(docs[0].matched_concerns.is_empty());
        assert_eq!(docs[0].reasons.len(), 1);
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
            path: vec![],
            component: None,
            concern: vec!["billing".into(), "sessions".into()],
            paths_only: false,
            explain: false,
        };

        let docs = select_documents(&registry, &args).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].id, "ctx-a");
        assert_eq!(docs[0].matched_concerns, vec!["billing".to_string()]);
        assert_eq!(docs[1].id, "ctx-b");
        assert_eq!(docs[1].matched_concerns, vec!["sessions".to_string()]);
    }

    #[test]
    fn formats_explain_reasons() {
        let reasons = vec![
            InclusionReason {
                kind: "concern-match",
                requested: "billing".into(),
                matched: "billing".into(),
            },
            InclusionReason {
                kind: "path-match",
                requested: "src/**".into(),
                matched: "src/billing/**".into(),
            },
        ];

        assert_eq!(
            format_reasons(&reasons),
            "concern billing, path src/billing/**"
        );
    }

    #[test]
    fn selects_all_active_documents_when_no_predicates_are_given() {
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
                        status: Status::Superseded,
                        concerns: vec!["auth".into()],
                        active_concerns: vec![],
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
            path: vec![],
            component: None,
            concern: vec![],
            paths_only: false,
            explain: true,
        };

        let docs = select_documents(&registry, &args).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, "ctx-a");
        assert_eq!(docs[0].reasons.len(), 1);
        assert_eq!(docs[0].reasons[0].kind, "default-active");
    }

    #[test]
    fn selects_documents_matching_any_requested_path() {
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
                        scope: crate::document::Scope {
                            paths: vec!["src/billing/**".into()],
                            components: vec![],
                        },
                        superseded_by: vec![],
                    },
                ),
                (
                    "ctx-b".to_string(),
                    DocumentEntry {
                        file: "README.md".into(),
                        created: Utc::now(),
                        status: Status::Current,
                        concerns: vec!["auth".into()],
                        active_concerns: vec!["auth".into()],
                        scope: crate::document::Scope {
                            paths: vec!["src/auth/**".into()],
                            components: vec![],
                        },
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
            path: vec!["src/billing/**".into(), "src/auth/**".into()],
            component: None,
            concern: vec![],
            paths_only: false,
            explain: false,
        };

        let docs = select_documents(&registry, &args).unwrap();
        assert_eq!(docs.len(), 2);
    }
}
