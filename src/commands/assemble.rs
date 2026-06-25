use std::{env, fs, path::Path};

use anyhow::{Context, Result};
use glob::Pattern;
use serde::Serialize;

use crate::{
    cli::{AssembleArgs, AssembleScope},
    document::{Status, active_concerns, parse_document, sort_concern_sections_by_rank},
    output::OutputMode,
    registry::{CollectOptions, Registry, collect_documents_from_with_options, load_or_sync},
    subtree::{rebase_scope_path, subtree_context_roots},
};

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

#[derive(Clone, Debug)]
struct AssemblyCandidate {
    id: String,
    file: String,
    status: Status,
    active_concerns: Vec<String>,
    scope_paths: Vec<String>,
    components: Vec<String>,
    content: String,
}

pub fn run(args: AssembleArgs, output_mode: OutputMode) -> Result<()> {
    let docs = match args.scope {
        AssembleScope::Current => {
            let registry = load_or_sync()?;
            select_documents(&registry, &args)?
        }
        AssembleScope::Subtree => {
            let origin = env::current_dir().context("failed to determine current directory")?;
            let candidates = collect_subtree_candidates(&origin)?;
            select_candidates(&candidates, &args)?
        }
    };

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

fn select_documents(registry: &Registry, args: &AssembleArgs) -> Result<Vec<AssembledDocument>> {
    let mut candidates = Vec::new();
    for (id, entry) in &registry.documents {
        let content = fs::read_to_string(&entry.file)
            .with_context(|| format!("failed to read {}", entry.file))?;
        candidates.push(AssemblyCandidate {
            id: id.clone(),
            file: entry.file.clone(),
            status: entry.status.clone(),
            active_concerns: entry.active_concerns.clone(),
            scope_paths: entry.scope.paths.clone(),
            components: entry.scope.components.clone(),
            content: sort_concern_sections_by_rank(&strip_frontmatter(&content)),
        });
    }

    select_candidates(&candidates, args)
}

fn select_candidates(
    candidates: &[AssemblyCandidate],
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
    for candidate in candidates {
        if candidate.status == Status::Superseded {
            continue;
        }

        let mut reasons = Vec::new();

        for (requested_path, pattern) in args.path.iter().zip(compiled_paths.iter()) {
            for scope_path in &candidate.scope_paths {
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
            for item in &candidate.components {
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
            .filter(|concern| {
                candidate
                    .active_concerns
                    .iter()
                    .any(|item| item == *concern)
            })
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

        docs.push(AssembledDocument {
            id: candidate.id.clone(),
            file: candidate.file.clone(),
            active_concerns: candidate.active_concerns.clone(),
            matched_concerns,
            reasons,
            content: candidate.content.clone(),
        });
    }

    docs.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(docs)
}

fn collect_subtree_candidates(origin: &Path) -> Result<Vec<AssemblyCandidate>> {
    let mut candidates = Vec::new();
    for root in subtree_context_roots(origin)? {
        let docs = collect_documents_from_with_options(
            &root.base,
            CollectOptions {
                include_synthesized: false,
            },
        )?;

        for (path, frontmatter) in docs {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let (_, body) = parse_document(&content)
                .with_context(|| format!("failed to parse frontmatter in {}", path.display()))?;
            let relative_file = path
                .strip_prefix(origin)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            let mut scope_paths = frontmatter
                .scope
                .paths
                .iter()
                .map(|scope_path| rebase_scope_path(&root.relative, scope_path))
                .collect::<Vec<_>>();
            scope_paths.sort();
            scope_paths.dedup();
            let status = frontmatter.status.clone();
            let active = active_concerns(&frontmatter);
            let components = frontmatter.scope.components.clone();

            candidates.push(AssemblyCandidate {
                id: frontmatter.id,
                file: relative_file,
                status,
                active_concerns: active,
                scope_paths,
                components,
                content: sort_concern_sections_by_rank(&body),
            });
        }
    }

    candidates.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(candidates)
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
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::Utc;

    use super::{
        AssemblyCandidate, InclusionReason, collect_subtree_candidates, format_reasons,
        select_candidates, select_documents, strip_frontmatter,
    };
    use crate::{
        cli::{AssembleArgs, AssembleScope},
        document::{Frontmatter, Scope, Status, write_document},
        registry::{DocumentEntry, Registry},
        subtree::SYNTHESIZED_CHILD_CONTEXT_FILE,
    };

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("ctx-assemble-{nanos}"))
    }

    fn write_doc(path: &Path, frontmatter: &Frontmatter, body: &str) {
        fs::write(path, write_document(frontmatter, body).unwrap()).unwrap();
    }

    fn base_args() -> AssembleArgs {
        AssembleArgs {
            path: vec![],
            component: None,
            concern: vec![],
            paths_only: false,
            explain: false,
            scope: AssembleScope::Current,
        }
    }

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

        let mut args = base_args();
        args.component = Some("billing-service".into());

        let docs = select_documents(&registry, &args).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, "ctx-a");
        assert!(docs[0].matched_concerns.is_empty());
        assert_eq!(docs[0].reasons.len(), 1);
    }

    #[test]
    fn selects_documents_matching_any_requested_concern() {
        let candidates = vec![
            AssemblyCandidate {
                id: "ctx-a".into(),
                file: "Cargo.toml".into(),
                status: Status::Current,
                active_concerns: vec!["billing".into()],
                scope_paths: vec![],
                components: vec![],
                content: String::new(),
            },
            AssemblyCandidate {
                id: "ctx-b".into(),
                file: "README.md".into(),
                status: Status::Current,
                active_concerns: vec!["auth".into(), "sessions".into()],
                scope_paths: vec![],
                components: vec![],
                content: String::new(),
            },
        ];

        let mut args = base_args();
        args.concern = vec!["billing".into(), "sessions".into()];

        let docs = select_candidates(&candidates, &args).unwrap();
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
        let candidates = vec![
            AssemblyCandidate {
                id: "ctx-a".into(),
                file: "Cargo.toml".into(),
                status: Status::Current,
                active_concerns: vec!["billing".into()],
                scope_paths: vec![],
                components: vec![],
                content: String::new(),
            },
            AssemblyCandidate {
                id: "ctx-b".into(),
                file: "README.md".into(),
                status: Status::Superseded,
                active_concerns: vec![],
                scope_paths: vec![],
                components: vec![],
                content: String::new(),
            },
        ];

        let mut args = base_args();
        args.explain = true;

        let docs = select_candidates(&candidates, &args).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, "ctx-a");
        assert_eq!(docs[0].reasons.len(), 1);
        assert_eq!(docs[0].reasons[0].kind, "default-active");
    }

    #[test]
    fn selects_documents_matching_any_requested_path() {
        let candidates = vec![
            AssemblyCandidate {
                id: "ctx-a".into(),
                file: "Cargo.toml".into(),
                status: Status::Current,
                active_concerns: vec!["billing".into()],
                scope_paths: vec!["src/billing/**".into()],
                components: vec![],
                content: String::new(),
            },
            AssemblyCandidate {
                id: "ctx-b".into(),
                file: "README.md".into(),
                status: Status::Current,
                active_concerns: vec!["auth".into()],
                scope_paths: vec!["src/auth/**".into()],
                components: vec![],
                content: String::new(),
            },
        ];

        let mut args = base_args();
        args.path = vec!["src/billing/**".into(), "src/auth/**".into()];

        let docs = select_candidates(&candidates, &args).unwrap();
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn assembles_document_content_with_sections_sorted_by_rank() {
        let base = unique_temp_dir();
        fs::create_dir_all(&base).unwrap();

        let file = base.join("notes.md");
        let frontmatter = Frontmatter {
            id: "ctx-a".into(),
            created: Utc::now(),
            status: Status::Current,
            concerns: vec!["billing".into(), "auth".into()],
            scope: Scope::default(),
            superseded_by: vec![],
        };
        write_doc(
            &file,
            &frontmatter,
            "### billing [r2]\n\nLower\n\n### auth [r5]\n\nHigher\n",
        );

        let registry = Registry {
            schema_version: 1,
            generated_at: Utc::now(),
            generated_from_commit: None,
            documents: [(
                "ctx-a".to_string(),
                DocumentEntry {
                    file: file.to_string_lossy().into_owned(),
                    created: frontmatter.created,
                    status: Status::Current,
                    concerns: frontmatter.concerns.clone(),
                    active_concerns: frontmatter.concerns.clone(),
                    scope: Scope::default(),
                    superseded_by: vec![],
                },
            )]
            .into_iter()
            .collect(),
            concern_roster: Default::default(),
            orphaned_concerns: vec![],
            multi_owned_concerns: vec![],
        };

        let docs = select_documents(&registry, &base_args()).unwrap();
        assert_eq!(
            docs[0].content,
            "### auth [r5]\n\nHigher\n\n### billing [r2]\n\nLower\n"
        );

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn subtree_collects_raw_descendant_context_and_rebases_paths() {
        let base = unique_temp_dir();
        fs::create_dir_all(base.join(".context")).unwrap();
        fs::create_dir_all(base.join("apps/api/.context")).unwrap();

        let root_doc = Frontmatter {
            id: "ctx-root".into(),
            created: Utc::now(),
            status: Status::Current,
            concerns: vec!["root".into()],
            scope: Scope {
                paths: vec!["src/root.rs".into()],
                components: vec!["root-cli".into()],
            },
            superseded_by: vec![],
        };
        write_doc(
            &base.join(".context/root.md"),
            &root_doc,
            "### root [r4]\n\nRoot note\n",
        );

        let child_doc = Frontmatter {
            id: "ctx-child".into(),
            created: Utc::now(),
            status: Status::Current,
            concerns: vec!["child".into()],
            scope: Scope {
                paths: vec!["src/lib.rs".into()],
                components: vec!["api".into()],
            },
            superseded_by: vec![],
        };
        write_doc(
            &base.join("apps/api/.context/child.md"),
            &child_doc,
            "### child [r4]\n\nChild note\n",
        );
        write_doc(
            &base.join(format!(
                "apps/api/.context/{SYNTHESIZED_CHILD_CONTEXT_FILE}"
            )),
            &Frontmatter {
                id: "ctx-synth".into(),
                created: Utc::now(),
                status: Status::Current,
                concerns: vec!["child-context:apps/api/services".into()],
                scope: Scope::default(),
                superseded_by: vec![],
            },
            "### child-context:apps/api/services [r3]\n\nGenerated summary\n",
        );

        let docs = collect_subtree_candidates(&base).unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].file, ".context/root.md");
        assert_eq!(docs[1].file, "apps/api/.context/child.md");
        assert_eq!(docs[1].scope_paths, vec!["apps/api/src/lib.rs".to_string()]);

        fs::remove_dir_all(base).unwrap();
    }
}
