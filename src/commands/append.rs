use std::{fs, path::Path};

use anyhow::{bail, Context, Result};

use crate::{
    cli::AppendArgs,
    document::{active_concerns, parse_document, write_document},
    output::OutputMode,
    registry::{load_or_sync_from, sync_corpus_from},
};

pub fn run(args: AppendArgs, output_mode: OutputMode) -> Result<()> {
    append_to_document(&args, Path::new("."))?;

    match output_mode {
        OutputMode::Human => {
            println!("Appended note to {} for concern {}", args.id, args.concern);
        }
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "id": args.id,
                    "concern": args.concern,
                }))?
            );
        }
        OutputMode::Porcelain => {
            println!("{} {}", args.id, args.concern);
        }
    }

    Ok(())
}

fn append_to_document(args: &AppendArgs, base: &Path) -> Result<()> {
    let registry = load_or_sync_from(base)?;
    let entry = registry
        .documents
        .get(&args.id)
        .with_context(|| format!("document {} not found", args.id))?;
    let file_path = base.join(&entry.file);

    let content = fs::read_to_string(&file_path)
        .with_context(|| format!("failed to read {}", file_path.display()))?;
    let (frontmatter, body) = parse_document(&content)
        .with_context(|| format!("failed to parse frontmatter in {}", file_path.display()))?;

    let active = active_concerns(&frontmatter);
    if !active.contains(&args.concern) {
        bail!(
            "concern {} is not active in document {}",
            args.concern,
            frontmatter.id
        );
    }

    let updated_body = append_block(&body, &args.concern, &args.text);
    let updated = write_document(&frontmatter, &updated_body)?;
    fs::write(&file_path, updated)
        .with_context(|| format!("failed to write {}", file_path.display()))?;

    sync_corpus_from(base)?;
    Ok(())
}

fn append_block(body: &str, concern: &str, text: &str) -> String {
    let mut out = body.trim_end_matches('\n').to_string();
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str("### ");
    out.push_str(concern);
    out.push_str("\n\n");
    out.push_str(text.trim());
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::{TimeZone, Utc};

    use super::{append_block, append_to_document};
    use crate::{
        cli::AppendArgs,
        document::{write_document, Frontmatter, Scope, Status, SupersededBy},
    };

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("ctx-phase3-append-{nanos}"))
    }

    fn write_doc(base: &Path) {
        let ctx_dir = base.join(".context");
        fs::create_dir_all(&ctx_dir).unwrap();
        let frontmatter = Frontmatter {
            id: "ctx-7f3a9b".into(),
            created: Utc.with_ymd_and_hms(2025, 10, 15, 14, 23, 0).unwrap(),
            status: Status::Current,
            concerns: vec!["billing".into(), "token-expiry".into()],
            scope: Scope::default(),
            superseded_by: vec![SupersededBy {
                id: "ctx-2a81fc".into(),
                concerns: vec!["token-expiry".into()],
            }],
        };
        fs::write(
            ctx_dir.join("billing.md"),
            write_document(&frontmatter, "Initial body.\n").unwrap(),
        )
        .unwrap();
        fs::write(
            ctx_dir.join(".registry.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": 1,
                "generated_at": "2025-10-15T14:23:00Z",
                "generated_from_commit": null,
                "documents": {
                    "ctx-7f3a9b": {
                        "file": ".context/billing.md",
                        "created": "2025-10-15T14:23:00Z",
                        "status": "current",
                        "concerns": ["billing", "token-expiry"],
                        "active_concerns": ["billing"],
                        "scope": {"paths": [], "components": []},
                        "superseded_by": [{"id": "ctx-2a81fc", "concerns": ["token-expiry"]}]
                    }
                },
                "concern_roster": {"billing": {"owners": ["ctx-7f3a9b"]}},
                "orphaned_concerns": [],
                "multi_owned_concerns": []
            }))
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn appends_blocks_consistently() {
        let body = append_block("Existing text.\n", "billing", "New note");
        assert_eq!(body, "Existing text.\n\n### billing\n\nNew note\n");
    }

    #[test]
    fn appends_to_active_concern() {
        let base = unique_temp_dir();
        write_doc(&base);

        let args = AppendArgs {
            id: "ctx-7f3a9b".into(),
            concern: "billing".into(),
            text: "Investigated renewal edge case.".into(),
        };

        append_to_document(&args, &base).unwrap();

        let written = fs::read_to_string(base.join(".context/billing.md")).unwrap();
        assert!(written.contains("### billing"));
        assert!(written.contains("Investigated renewal edge case."));

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn rejects_append_to_inactive_concern() {
        let base = unique_temp_dir();
        write_doc(&base);

        let args = AppendArgs {
            id: "ctx-7f3a9b".into(),
            concern: "token-expiry".into(),
            text: "This should fail".into(),
        };

        let err = append_to_document(&args, &base).unwrap_err();
        assert!(err.to_string().contains("is not active"));

        fs::remove_dir_all(base).unwrap();
    }
}
