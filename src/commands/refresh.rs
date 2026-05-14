use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use chrono::Utc;

use crate::{
    cli::RefreshArgs,
    document::{
        Frontmatter, Scope, Status, SupersededBy, active_concerns, extract_concern_section,
        parse_document, recompute_status, write_document,
    },
    id::generate_id,
    output::OutputMode,
    registry::{context_dir_from, load_or_sync_from, sync_corpus_from},
};

pub fn run(args: RefreshArgs, output_mode: OutputMode) -> Result<()> {
    let refreshed = refresh_concern(&args, Path::new("."))?;

    match output_mode {
        OutputMode::Human => {
            println!(
                "Refreshed concern {} from {} into {} ({})",
                args.concern, refreshed.source_id, refreshed.new_id, refreshed.file
            );
        }
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "concern": args.concern,
                    "source_id": refreshed.source_id,
                    "new_id": refreshed.new_id,
                    "file": refreshed.file,
                }))?
            );
        }
        OutputMode::Porcelain => {
            println!(
                "{}\t{}\t{}\t{}",
                args.concern, refreshed.source_id, refreshed.new_id, refreshed.file
            );
        }
    }

    Ok(())
}

struct RefreshResult {
    source_id: String,
    new_id: String,
    file: String,
}

fn refresh_concern(args: &RefreshArgs, base: &Path) -> Result<RefreshResult> {
    let concern = args.concern.trim();
    if concern.is_empty() {
        bail!("--concern is required");
    }

    let name = normalize_name(&args.name);
    let file_path = context_dir_from(base).join(format!("{name}.md"));
    if file_path.exists() {
        bail!("document already exists at {}", file_path.display());
    }

    let registry = load_or_sync_from(base)?;
    let owners = registry
        .concern_roster
        .get(concern)
        .with_context(|| format!("concern {concern} has no active owner"))?
        .owners
        .clone();

    let source_id = match (owners.as_slice(), args.from.as_deref()) {
        ([single], None) => single.clone(),
        (_, Some(from)) => {
            if owners.iter().any(|owner| owner == from) {
                from.to_string()
            } else {
                bail!("document {from} is not an active owner of concern {concern}");
            }
        }
        _ => bail!(
            "concern {concern} has multiple active owners: {}; use --from to disambiguate",
            owners.join(", ")
        ),
    };

    let source_entry = registry
        .documents
        .get(&source_id)
        .with_context(|| format!("document {source_id} not found"))?;
    let source_path = base.join(&source_entry.file);
    let source_content = fs::read_to_string(&source_path)
        .with_context(|| format!("failed to read {}", source_path.display()))?;
    let (mut source_frontmatter, source_body) = parse_document(&source_content)
        .with_context(|| format!("failed to parse frontmatter in {}", source_path.display()))?;

    let active = active_concerns(&source_frontmatter);
    if !active.iter().any(|item| item == concern) {
        bail!(
            "concern {concern} is not active in document {}",
            source_frontmatter.id
        );
    }

    fs::create_dir_all(context_dir_from(base))
        .with_context(|| format!("failed to create {}", context_dir_from(base).display()))?;

    let created = Utc::now();
    let new_frontmatter = Frontmatter {
        id: generate_id(&name, &created),
        created,
        status: Status::Current,
        concerns: vec![concern.to_string()],
        scope: Scope {
            paths: source_frontmatter.scope.paths.clone(),
            components: source_frontmatter.scope.components.clone(),
        },
        superseded_by: Vec::new(),
    };

    let draft_body = if args.draft_body {
        extract_concern_section(&source_body, concern).unwrap_or_else(|| source_body.clone())
    } else {
        String::new()
    };
    let new_content = write_document(&new_frontmatter, &draft_body)?;
    fs::write(&file_path, new_content)
        .with_context(|| format!("failed to write {}", file_path.display()))?;

    if let Some(existing) = source_frontmatter
        .superseded_by
        .iter_mut()
        .find(|entry| entry.id == new_frontmatter.id)
    {
        if !existing.concerns.iter().any(|item| item == concern) {
            existing.concerns.push(concern.to_string());
            existing.concerns.sort();
            existing.concerns.dedup();
        }
    } else {
        source_frontmatter.superseded_by.push(SupersededBy {
            id: new_frontmatter.id.clone(),
            concerns: vec![concern.to_string()],
        });
        source_frontmatter
            .superseded_by
            .sort_by(|a, b| a.id.cmp(&b.id));
    }

    recompute_status(&mut source_frontmatter);
    let updated_source = write_document(&source_frontmatter, &source_body)?;
    fs::write(&source_path, updated_source)
        .with_context(|| format!("failed to write {}", source_path.display()))?;

    sync_corpus_from(base)?;

    Ok(RefreshResult {
        source_id,
        new_id: new_frontmatter.id,
        file: file_path.to_string_lossy().into_owned(),
    })
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

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::{TimeZone, Utc};

    use super::refresh_concern;
    use crate::{
        cli::RefreshArgs,
        document::{Frontmatter, Scope, Status, write_document},
    };

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("ctx-refresh-{nanos}"))
    }

    fn seed_doc(base: &Path) {
        let ctx_dir = base.join(".context");
        fs::create_dir_all(&ctx_dir).unwrap();
        let frontmatter = Frontmatter {
            id: "ctx-source".into(),
            created: Utc.with_ymd_and_hms(2025, 10, 15, 14, 23, 0).unwrap(),
            status: Status::Current,
            concerns: vec!["token-expiry".into(), "session-management".into()],
            scope: Scope {
                paths: vec!["src/auth/**".into()],
                components: vec!["auth-service".into()],
            },
            superseded_by: vec![],
        };
        fs::write(
            ctx_dir.join("source.md"),
            write_document(
                &frontmatter,
                "### token-expiry\n\nOld draft\n\n### session-management\n\nKeep\n",
            )
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn refreshes_single_owner_concern() {
        let base = unique_temp_dir();
        seed_doc(&base);

        let args = RefreshArgs {
            concern: "token-expiry".into(),
            from: None,
            name: "token-expiry-refresh".into(),
            draft_body: true,
        };

        let result = refresh_concern(&args, &base).unwrap();
        let new_doc = fs::read_to_string(base.join(".context/token-expiry-refresh.md")).unwrap();
        assert!(new_doc.contains("concerns:\n- token-expiry"));
        assert!(new_doc.contains("### token-expiry"));

        let source_doc = fs::read_to_string(base.join(".context/source.md")).unwrap();
        assert!(source_doc.contains("status: partially-superseded"));
        assert!(source_doc.contains(&result.new_id));

        fs::remove_dir_all(base).unwrap();
    }
}
