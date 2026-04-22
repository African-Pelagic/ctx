use std::{fs, path::Path};

use anyhow::{bail, Context, Result};

use crate::{
    cli::SupersedeArgs,
    document::{active_concerns, parse_document, recompute_status, write_document, SupersededBy},
    output::OutputMode,
    registry::{load_or_sync_from, sync_corpus_from},
};

pub fn run(args: SupersedeArgs, output_mode: OutputMode) -> Result<()> {
    supersede_document(&args, Path::new("."))?;

    match output_mode {
        OutputMode::Human => {
            println!(
                "Recorded supersession for {} by {}",
                args.id, args.by_id
            );
        }
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "id": args.id,
                    "by": args.by_id,
                    "concerns": args.concerns,
                }))?
            );
        }
        OutputMode::Porcelain => {
            println!("{} {}", args.id, args.by_id);
        }
    }

    Ok(())
}

fn supersede_document(args: &SupersedeArgs, base: &Path) -> Result<()> {
    let concerns = normalize_values(&args.concerns);
    if concerns.is_empty() {
        bail!("--concerns is required");
    }

    let registry = load_or_sync_from(base)?;
    let source_entry = registry
        .documents
        .get(&args.id)
        .with_context(|| format!("document {} not found", args.id))?;
    let _replacement_entry = registry
        .documents
        .get(&args.by_id)
        .with_context(|| format!("replacement document {} not found", args.by_id))?;

    let source_path = base.join(&source_entry.file);
    let content = fs::read_to_string(&source_path)
        .with_context(|| format!("failed to read {}", source_path.display()))?;
    let (mut frontmatter, body) = parse_document(&content)
        .with_context(|| format!("failed to parse frontmatter in {}", source_path.display()))?;

    let active = active_concerns(&frontmatter);
    for concern in &concerns {
        if !active.contains(concern) {
            bail!(
                "concern {} is not active in document {}",
                concern,
                frontmatter.id
            );
        }
    }

    if let Some(existing) = frontmatter
        .superseded_by
        .iter_mut()
        .find(|entry| entry.id == args.by_id)
    {
        existing.concerns.extend(concerns);
        existing.concerns.sort();
        existing.concerns.dedup();
    } else {
        frontmatter.superseded_by.push(SupersededBy {
            id: args.by_id.clone(),
            concerns,
        });
        frontmatter
            .superseded_by
            .sort_by(|a, b| a.id.cmp(&b.id));
    }

    recompute_status(&mut frontmatter);

    let updated = write_document(&frontmatter, &body)?;
    fs::write(&source_path, updated)
        .with_context(|| format!("failed to write {}", source_path.display()))?;

    sync_corpus_from(base)?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::{TimeZone, Utc};

    use super::{normalize_values, supersede_document};
    use crate::{
        cli::SupersedeArgs,
        document::{write_document, Frontmatter, Scope, Status, SupersededBy},
    };

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("ctx-phase4-supersede-{nanos}"))
    }

    fn write_doc(path: &Path, frontmatter: &Frontmatter, body: &str) {
        fs::write(path, write_document(frontmatter, body).unwrap()).unwrap();
    }

    fn write_registry(base: &Path) {
        fs::write(
            base.join(".context/.registry.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 1,
                "generated_at": "2025-10-15T14:23:00Z",
                "generated_from_commit": null,
                "documents": {
                    "ctx-source": {
                        "file": ".context/source.md",
                        "created": "2025-10-15T14:23:00Z",
                        "status": "current",
                        "concerns": ["session-management", "token-expiry"],
                        "active_concerns": ["session-management", "token-expiry"],
                        "scope": {"paths": [], "components": []},
                        "superseded_by": []
                    },
                    "ctx-replacement": {
                        "file": ".context/replacement.md",
                        "created": "2025-10-15T14:24:00Z",
                        "status": "current",
                        "concerns": ["token-expiry"],
                        "active_concerns": ["token-expiry"],
                        "scope": {"paths": [], "components": []},
                        "superseded_by": []
                    }
                },
                "concern_roster": {
                    "session-management": {"owners": ["ctx-source"]},
                    "token-expiry": {"owners": ["ctx-source", "ctx-replacement"]}
                },
                "orphaned_concerns": [],
                "multi_owned_concerns": ["token-expiry"]
            }))
            .unwrap(),
        )
        .unwrap();
    }

    fn seed_corpus(base: &Path) {
        let ctx_dir = base.join(".context");
        fs::create_dir_all(&ctx_dir).unwrap();

        let source = Frontmatter {
            id: "ctx-source".into(),
            created: Utc.with_ymd_and_hms(2025, 10, 15, 14, 23, 0).unwrap(),
            status: Status::Current,
            concerns: vec!["session-management".into(), "token-expiry".into()],
            scope: Scope::default(),
            superseded_by: vec![],
        };
        let replacement = Frontmatter {
            id: "ctx-replacement".into(),
            created: Utc.with_ymd_and_hms(2025, 10, 15, 14, 24, 0).unwrap(),
            status: Status::Current,
            concerns: vec!["token-expiry".into()],
            scope: Scope::default(),
            superseded_by: vec![],
        };

        write_doc(&ctx_dir.join("source.md"), &source, "source body\n");
        write_doc(&ctx_dir.join("replacement.md"), &replacement, "replacement body\n");
        write_registry(base);
    }

    #[test]
    fn normalizes_and_deduplicates_concerns() {
        assert_eq!(
            normalize_values(&[" token-expiry ".into(), "session-management, token-expiry".into()]),
            vec!["session-management".to_string(), "token-expiry".to_string()]
        );
    }

    #[test]
    fn records_partial_supersession() {
        let base = unique_temp_dir();
        seed_corpus(&base);

        let args = SupersedeArgs {
            id: "ctx-source".into(),
            concerns: vec!["token-expiry".into()],
            by_id: "ctx-replacement".into(),
        };

        supersede_document(&args, &base).unwrap();

        let written = fs::read_to_string(base.join(".context/source.md")).unwrap();
        assert!(written.contains("id: ctx-replacement"));
        assert!(written.contains("- token-expiry"));
        assert!(written.contains("status: partially-superseded"));

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn records_full_supersession() {
        let base = unique_temp_dir();
        seed_corpus(&base);

        let args = SupersedeArgs {
            id: "ctx-source".into(),
            concerns: vec!["session-management".into(), "token-expiry".into()],
            by_id: "ctx-replacement".into(),
        };

        supersede_document(&args, &base).unwrap();

        let written = fs::read_to_string(base.join(".context/source.md")).unwrap();
        assert!(written.contains("status: superseded"));

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn rejects_non_active_concerns() {
        let base = unique_temp_dir();
        seed_corpus(&base);

        let existing = fs::read_to_string(base.join(".context/source.md")).unwrap();
        let (mut frontmatter, body) = crate::document::parse_document(&existing).unwrap();
        frontmatter.superseded_by.push(SupersededBy {
            id: "ctx-old".into(),
            concerns: vec!["token-expiry".into()],
        });
        fs::write(
            base.join(".context/source.md"),
            write_document(&frontmatter, &body).unwrap(),
        )
        .unwrap();

        let args = SupersedeArgs {
            id: "ctx-source".into(),
            concerns: vec!["token-expiry".into()],
            by_id: "ctx-replacement".into(),
        };

        let err = supersede_document(&args, &base).unwrap_err();
        assert!(err.to_string().contains("is not active"));

        fs::remove_dir_all(base).unwrap();
    }
}
