use std::{
    error::Error,
    fs,
    fmt,
    path::{Path, PathBuf},
    process,
};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::Serialize;

use crate::{
    cli::NewArgs,
    document::{write_document, Frontmatter, Scope, Status},
    id::generate_id,
    output::OutputMode,
    registry::{context_dir_from, load_or_sync_from, sync_corpus_from},
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct Conflict {
    concern: String,
    owners: Vec<String>,
}

#[derive(Debug)]
struct ConflictError(Vec<Conflict>);

impl fmt::Display for ConflictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "concern overlap")
    }
}

impl Error for ConflictError {}

pub fn run(args: NewArgs, output_mode: OutputMode) -> Result<()> {
    match create_document(&args, Path::new(".")) {
        Ok(()) => {}
        Err(NewCommandError::Conflicts(conflicts)) => {
            emit_conflicts(&conflicts, output_mode)?;
            process::exit(3);
        }
        Err(NewCommandError::Fatal(err)) => return Err(err),
    }

    match output_mode {
        OutputMode::Human => {
            println!("Created {}", output_path(&args.name).display());
        }
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "file": output_path(&args.name),
                }))?
            );
        }
        OutputMode::Porcelain => {
            println!("{}", output_path(&args.name).display());
        }
    }

    Ok(())
}

enum NewCommandError {
    Fatal(anyhow::Error),
    Conflicts(Vec<Conflict>),
}

fn create_document(args: &NewArgs, base: &Path) -> std::result::Result<(), NewCommandError> {
    create_document_inner(args, base).map_err(|err| match err.downcast::<ConflictError>() {
        Ok(conflicts) => NewCommandError::Conflicts(conflicts.0),
        Err(other) => NewCommandError::Fatal(other),
    })
}

fn create_document_inner(args: &NewArgs, base: &Path) -> Result<()> {
    if !args.non_interactive {
        bail!("interactive mode is not implemented yet; use --non-interactive");
    }

    let concerns = normalize_values(&args.concerns);
    if concerns.is_empty() {
        bail!("--concerns is required in --non-interactive mode");
    }

    let name = normalize_name(&args.name);
    let file_path = context_dir_from(base).join(format!("{name}.md"));
    if file_path.exists() {
        bail!("document already exists at {}", file_path.display());
    }

    let registry = load_or_sync_from(base)?;
    let conflicts = detect_conflicts(&registry, &concerns);
    if !conflicts.is_empty() && !args.append {
        return Err(ConflictError(conflicts).into());
    }

    fs::create_dir_all(context_dir_from(base))
        .with_context(|| format!("failed to create {}", context_dir_from(base).display()))?;

    let created = Utc::now();
    let frontmatter = Frontmatter {
        id: generate_id(&name, &created),
        created,
        status: Status::Current,
        concerns,
        scope: Scope {
            paths: normalize_values(&args.paths),
            components: normalize_values(&args.components),
        },
        superseded_by: Vec::new(),
    };

    let content = write_document(&frontmatter, "")?;
    fs::write(&file_path, content)
        .with_context(|| format!("failed to write {}", file_path.display()))?;

    sync_corpus_from(base)?;
    Ok(())
}

fn output_path(name: &str) -> PathBuf {
    PathBuf::from(".context").join(format!("{}.md", normalize_name(name)))
}

fn normalize_name(name: &str) -> String {
    let trimmed = name.trim();
    match trimmed.rsplit_once('.') {
        Some((base, ext)) if !base.is_empty() && !ext.contains('/') && !ext.contains('\\') => {
            base.to_string()
        }
        _ => trimmed.to_string(),
    }
}

fn normalize_values(values: &[String]) -> Vec<String> {
    let mut normalized = values
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn detect_conflicts(registry: &crate::registry::Registry, concerns: &[String]) -> Vec<Conflict> {
    let mut conflicts = concerns
        .iter()
        .filter_map(|concern| {
            registry.concern_roster.get(concern).map(|entry| Conflict {
                concern: concern.clone(),
                owners: entry.owners.clone(),
            })
        })
        .collect::<Vec<_>>();
    conflicts.sort_by(|a, b| a.concern.cmp(&b.concern));
    conflicts
}

fn emit_conflicts(conflicts: &[Conflict], output_mode: OutputMode) -> Result<()> {
    match output_mode {
        OutputMode::Human => {
            eprintln!("concern overlap requires explicit resolution:");
            for conflict in conflicts {
                eprintln!(
                    "  {} already owned by {}",
                    conflict.concern,
                    conflict.owners.join(", ")
                );
            }
            eprintln!("re-run with --append to declare additive multi-ownership");
        }
        OutputMode::Json => {
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "error": "concern overlap",
                    "code": "overlap",
                    "conflicts": conflicts,
                }))?
            );
        }
        OutputMode::Porcelain => {
            for conflict in conflicts {
                eprintln!("overlap {} {}", conflict.concern, conflict.owners.join(","));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::{TimeZone, Utc};

    use super::{create_document_inner, detect_conflicts, normalize_name, normalize_values};
    use crate::{
        cli::NewArgs,
        document::{write_document, Frontmatter, Scope, Status},
        registry::Registry,
    };

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("ctx-phase3-new-{nanos}"))
    }

    fn write_existing_doc(base: &Path) {
        let ctx_dir = base.join(".context");
        fs::create_dir_all(&ctx_dir).unwrap();
        let frontmatter = Frontmatter {
            id: "ctx-existing".into(),
            created: Utc.with_ymd_and_hms(2025, 10, 15, 14, 23, 0).unwrap(),
            status: Status::Current,
            concerns: vec!["billing".into()],
            scope: Scope::default(),
            superseded_by: vec![],
        };
        fs::write(
            ctx_dir.join("existing.md"),
            write_document(&frontmatter, "body\n").unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn strips_extensions_from_names() {
        assert_eq!(normalize_name("billing-notes.md"), "billing-notes");
        assert_eq!(normalize_name("billing-notes"), "billing-notes");
    }

    #[test]
    fn normalizes_and_deduplicates_values() {
        assert_eq!(
            normalize_values(&[" billing ".into(), "billing, auth".into(), "".into()]),
            vec!["auth".to_string(), "billing".to_string()]
        );
    }

    #[test]
    fn detects_conflicts_from_roster() {
        let registry = Registry {
            schema_version: 1,
            generated_at: Utc::now(),
            generated_from_commit: None,
            documents: Default::default(),
            concern_roster: [(
                "billing".to_string(),
                crate::registry::ConcernRosterEntry {
                    owners: vec!["ctx-existing".to_string()],
                },
            )]
            .into_iter()
            .collect(),
            orphaned_concerns: vec![],
            multi_owned_concerns: vec![],
        };

        let conflicts = detect_conflicts(&registry, &["billing".into(), "auth".into()]);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].concern, "billing");
    }

    #[test]
    fn creates_document_and_registry_entry() {
        let base = unique_temp_dir();
        fs::create_dir_all(base.join(".context")).unwrap();

        let args = NewArgs {
            name: "billing-notes.md".into(),
            non_interactive: true,
            append: false,
            concerns: vec!["billing".into()],
            paths: vec!["src/billing/**".into()],
            components: vec!["billing-service".into()],
        };

        create_document_inner(&args, &base).unwrap();

        let created_file = base.join(".context/billing-notes.md");
        assert!(created_file.exists());
        let content = fs::read_to_string(created_file).unwrap();
        assert!(content.contains("concerns:"));
        assert!(base.join(".context/.registry.json").exists());

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn rejects_overlap_without_append() {
        let base = unique_temp_dir();
        write_existing_doc(&base);

        let args = NewArgs {
            name: "billing-notes".into(),
            non_interactive: true,
            append: false,
            concerns: vec!["billing".into()],
            paths: vec![],
            components: vec![],
        };

        let err = create_document_inner(&args, &base).unwrap_err();
        assert!(err.downcast_ref::<super::ConflictError>().is_some());

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn allows_overlap_with_append() {
        let base = unique_temp_dir();
        write_existing_doc(&base);

        let args = NewArgs {
            name: "billing-notes".into(),
            non_interactive: true,
            append: true,
            concerns: vec!["billing".into()],
            paths: vec![],
            components: vec![],
        };

        create_document_inner(&args, &base).unwrap();

        let registry = fs::read_to_string(base.join(".context/.registry.json")).unwrap();
        assert!(registry.contains("multi_owned_concerns"));
        assert!(registry.contains("billing"));

        fs::remove_dir_all(base).unwrap();
    }
}
