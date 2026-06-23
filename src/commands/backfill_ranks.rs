use std::{fs, path::Path};

use anyhow::{Context, Result};

use crate::{
    cli::BackfillRanksArgs,
    document::{parse_document, validate_rank, write_document},
    ignore::ContextIgnore,
    output::OutputMode,
    registry::context_dir_from,
};

pub fn run(args: BackfillRanksArgs, output_mode: OutputMode) -> Result<()> {
    validate_rank(args.default_rank)?;
    let updated = backfill_ranks(&args, Path::new("."))?;

    match output_mode {
        OutputMode::Human => {
            println!("Backfilled rank metadata in {updated} document(s)");
        }
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "updated_documents": updated,
                    "default_rank": args.default_rank,
                }))?
            );
        }
        OutputMode::Porcelain => {
            println!("{updated}\t{}", args.default_rank);
        }
    }

    Ok(())
}

fn backfill_ranks(args: &BackfillRanksArgs, base: &Path) -> Result<usize> {
    let context_dir = context_dir_from(base);
    if !context_dir.exists() {
        return Ok(0);
    }

    let ignore = ContextIgnore::load_from(base)?;
    let pattern = format!("{}/{}", context_dir.display(), "*.md");
    let mut paths = Vec::new();
    for entry in glob::glob(&pattern).with_context(|| format!("invalid glob pattern {pattern}"))? {
        paths.push(entry.with_context(|| format!("failed to enumerate files matching {pattern}"))?);
    }
    paths.sort();

    let mut updated = 0;
    for path in paths {
        let relative = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        if ignore.matches(&relative) {
            continue;
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let (frontmatter, body) = parse_document(&content)
            .with_context(|| format!("failed to parse frontmatter in {}", path.display()))?;
        let rewritten_body = rewrite_rank_headings(&body, args.default_rank);
        if rewritten_body == body {
            continue;
        }

        let updated_content = write_document(&frontmatter, &rewritten_body)?;
        fs::write(&path, updated_content)
            .with_context(|| format!("failed to write {}", path.display()))?;
        updated += 1;
    }

    Ok(updated)
}

fn rewrite_rank_headings(body: &str, default_rank: u8) -> String {
    let mut out = Vec::new();
    let lines = body.lines().collect::<Vec<_>>();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        if !line.starts_with("### ") {
            out.push(line.to_string());
            index += 1;
            continue;
        }

        let (heading, rank_line_consumed) = rewrite_heading(
            line,
            lines.get(index + 1),
            lines.get(index + 2),
            default_rank,
        );
        out.push(heading);

        if rank_line_consumed {
            index += 1;
            if index < lines.len() && lines[index].trim().is_empty() {
                index += 1;
            }
            if index < lines.len() && parse_legacy_rank_line(lines[index]).is_some() {
                index += 1;
            }
            if index < lines.len() && lines[index].trim().is_empty() {
                index += 1;
                if out.last().is_some_and(|last| !last.is_empty()) {
                    out.push(String::new());
                }
            }
            continue;
        }

        index += 1;
    }

    let mut text = out.join("\n");
    if body.ends_with('\n') {
        text.push('\n');
    }
    text
}

fn rewrite_heading(
    line: &str,
    next_line: Option<&&str>,
    second_next_line: Option<&&str>,
    default_rank: u8,
) -> (String, bool) {
    if heading_has_rank(line) {
        return (line.to_string(), false);
    }

    let rank = match (next_line.copied(), second_next_line.copied()) {
        (Some(blank), Some(rank_line)) if blank.trim().is_empty() => {
            parse_legacy_rank_line(rank_line).unwrap_or(default_rank)
        }
        (Some(rank_line), _) => parse_legacy_rank_line(rank_line).unwrap_or(default_rank),
        _ => default_rank,
    };

    (
        format!("{line} [r{rank}]"),
        legacy_rank_follows(next_line.copied(), second_next_line.copied()),
    )
}

fn legacy_rank_follows(next_line: Option<&str>, second_next_line: Option<&str>) -> bool {
    match (next_line, second_next_line) {
        (Some(blank), Some(rank_line)) if blank.trim().is_empty() => {
            parse_legacy_rank_line(rank_line).is_some()
        }
        (Some(rank_line), _) => parse_legacy_rank_line(rank_line).is_some(),
        _ => false,
    }
}

fn heading_has_rank(line: &str) -> bool {
    line.split_whitespace().any(|token| {
        token.starts_with("[r")
            && token.ends_with(']')
            && token[2..token.len() - 1].parse::<u8>().is_ok()
    })
}

fn parse_legacy_rank_line(line: &str) -> Option<u8> {
    let value = line.trim().strip_prefix("Rank: ")?;
    let rank = value.parse::<u8>().ok()?;
    (1..=5).contains(&rank).then_some(rank)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::{TimeZone, Utc};

    use super::{backfill_ranks, rewrite_rank_headings};
    use crate::{
        cli::BackfillRanksArgs,
        document::{Frontmatter, Scope, Status, write_document},
    };

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("ctx-backfill-ranks-{nanos}"))
    }

    #[test]
    fn rewrites_legacy_rank_line_into_heading() {
        let body = "### billing\n\nRank: 4\n\nBody\n";
        assert_eq!(rewrite_rank_headings(body, 2), "### billing [r4]\n\nBody\n");
    }

    #[test]
    fn adds_default_rank_to_unranked_heading() {
        let body = "### billing\n\nBody\n";
        assert_eq!(rewrite_rank_headings(body, 2), "### billing [r2]\n\nBody\n");
    }

    #[test]
    fn leaves_ranked_heading_unchanged() {
        let body = "### billing [n7] [r5]\n\nBody\n";
        assert_eq!(rewrite_rank_headings(body, 2), body);
    }

    #[test]
    fn backfills_context_documents() {
        let base = unique_temp_dir();
        let ctx_dir = base.join(".context");
        fs::create_dir_all(&ctx_dir).unwrap();

        let frontmatter = Frontmatter {
            id: "ctx-1".into(),
            created: Utc.with_ymd_and_hms(2025, 10, 15, 14, 23, 0).unwrap(),
            status: Status::Current,
            concerns: vec!["billing".into()],
            scope: Scope::default(),
            superseded_by: vec![],
        };

        fs::write(
            ctx_dir.join("note.md"),
            write_document(&frontmatter, "### billing\n\nRank: 3\n\nBody\n").unwrap(),
        )
        .unwrap();

        let updated = backfill_ranks(&BackfillRanksArgs { default_rank: 2 }, &base).unwrap();
        assert_eq!(updated, 1);

        let content = fs::read_to_string(ctx_dir.join("note.md")).unwrap();
        assert!(content.contains("### billing [r3]"));
        assert!(!content.contains("Rank: 3"));

        fs::remove_dir_all(base).unwrap();
    }
}
