use std::{
    fs,
    path::Path,
    process::{self, Command},
};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{
    cli::CheckArgs,
    document::{parse_document, Frontmatter},
    git::is_stale,
    output::OutputMode,
    registry::{context_dir_from, Registry},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Severity {
    Warning,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct Issue {
    severity: Severity,
    code: &'static str,
    file: String,
    message: String,
}

pub fn run(args: CheckArgs, output_mode: OutputMode) -> Result<()> {
    let issues = collect_issues(Path::new("."), args.strict)?;

    emit_issues(&issues, output_mode)?;

    let has_error = issues.iter().any(|issue| issue.severity == Severity::Error);
    let has_warning = issues.iter().any(|issue| issue.severity == Severity::Warning);

    if has_error {
        process::exit(1);
    }
    if has_warning {
        process::exit(2);
    }

    Ok(())
}

fn collect_issues(base: &Path, strict: bool) -> Result<Vec<Issue>> {
    let context_dir = context_dir_from(base);
    let (mut issues, docs) = scan_frontmatter(&context_dir)?;

    let registry = Registry::build(&docs);

    for concern in &registry.orphaned_concerns {
        issues.push(Issue {
            severity: as_severity(strict),
            code: "ORPHANED_CONCERN",
            file: ".context/.registry.json".to_string(),
            message: format!("concern {concern} has no active owner"),
        });
    }

    for (id, entry) in &registry.documents {
        if is_stale(Path::new(&entry.file), &entry.scope.paths) {
            issues.push(Issue {
                severity: as_severity(strict),
                code: "STALE_DOCUMENT",
                file: entry.file.clone(),
                message: format!("document {id} appears stale relative to its scoped paths"),
            });
        }
    }

    for concern in &registry.multi_owned_concerns {
        let owners = registry.concern_roster[concern].owners.join(", ");
        issues.push(Issue {
            severity: as_severity(strict),
            code: "MULTI_OWNED_CONCERN",
            file: ".context/.registry.json".to_string(),
            message: format!("concern {concern} has multiple active owners: {owners}"),
        });
    }

    issues.extend(staged_diff_issues(base)?);
    issues.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.code.cmp(b.code))
            .then(a.message.cmp(&b.message))
    });
    Ok(issues)
}

fn as_severity(strict: bool) -> Severity {
    if strict {
        Severity::Error
    } else {
        Severity::Warning
    }
}

fn scan_frontmatter(context_dir: &Path) -> Result<(Vec<Issue>, Vec<(std::path::PathBuf, Frontmatter)>)> {
    let mut issues = Vec::new();
    let mut docs = Vec::new();
    if !context_dir.exists() {
        return Ok((issues, docs));
    }

    let pattern = format!("{}/{}", context_dir.display(), "*.md");
    let mut paths = Vec::new();
    for entry in glob::glob(&pattern).with_context(|| format!("invalid glob pattern {pattern}"))? {
        paths.push(entry.with_context(|| format!("failed to enumerate files matching {pattern}"))?);
    }
    paths.sort();

    for path in paths {
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) => {
                issues.push(Issue {
                    severity: Severity::Error,
                    code: "FRONTMATTER_INVALID",
                    file: path.to_string_lossy().into_owned(),
                    message: format!("failed to read document: {err}"),
                });
                continue;
            }
        };

        match parse_document(&content) {
            Ok((frontmatter, _)) => docs.push((path, frontmatter)),
            Err(err) => {
                issues.push(Issue {
                    severity: Severity::Error,
                    code: "FRONTMATTER_INVALID",
                    file: path.to_string_lossy().into_owned(),
                    message: err.to_string(),
                });
            }
        }
    }

    Ok((issues, docs))
}

fn staged_diff_issues(base: &Path) -> Result<Vec<Issue>> {
    let files = staged_context_files(base)?;
    let mut issues = Vec::new();

    for file in files {
        let old_content = git_show(base, "HEAD", &file)?;
        let new_content = git_show(base, ":", &file)?;

        let Some(old_content) = old_content else {
            continue;
        };

        let deletions = removed_lines(&old_content, new_content.as_deref());
        let frontmatter = frontmatter_range(&old_content);

        for deletion in deletions {
            if !line_in_frontmatter(deletion.line_number, frontmatter) {
                issues.push(Issue {
                    severity: Severity::Error,
                    code: "APPEND_ONLY_VIOLATION",
                    file: file.clone(),
                    message: format!(
                        "staged deletion outside frontmatter at line {}",
                        deletion.line_number
                    ),
                });
            } else if is_managed_field(&deletion.content) {
                issues.push(Issue {
                    severity: Severity::Error,
                    code: "MANAGED_FIELD_TAMPERING",
                    file: file.clone(),
                    message: format!(
                        "managed field modified in frontmatter at line {}",
                        deletion.line_number
                    ),
                });
            }
        }
    }

    Ok(issues)
}

fn staged_context_files(base: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .current_dir(base)
        .args(["diff", "--cached", "--name-only", "--", ".context/"])
        .output()
        .context("failed to inspect staged .context diff")?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut files = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && line.ends_with(".md"))
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    files.sort();
    files.dedup();
    Ok(files)
}

fn git_show(base: &Path, revision_prefix: &str, file: &str) -> Result<Option<String>> {
    let spec = if revision_prefix == ":" {
        format!(":{file}")
    } else {
        format!("{revision_prefix}:{file}")
    };
    let output = Command::new("git")
        .current_dir(base)
        .args(["show", &spec])
        .output()
        .with_context(|| format!("failed to read git object {spec}"))?;

    if !output.status.success() {
        return Ok(None);
    }

    Ok(Some(String::from_utf8_lossy(&output.stdout).into_owned()))
}

#[derive(Debug, Eq, PartialEq)]
struct RemovedLine {
    line_number: usize,
    content: String,
}

fn removed_lines(old_content: &str, new_content: Option<&str>) -> Vec<RemovedLine> {
    let old_lines = old_content.lines().collect::<Vec<_>>();
    let new_lines = new_content
        .unwrap_or("")
        .lines()
        .collect::<Vec<_>>();

    let lcs = lcs_table(&old_lines, &new_lines);
    let mut removed = Vec::new();
    backtrack_removed(&old_lines, &new_lines, &lcs, old_lines.len(), new_lines.len(), &mut removed);
    removed.sort_by_key(|line| line.line_number);
    removed
}

fn lcs_table<'a>(old_lines: &[&'a str], new_lines: &[&'a str]) -> Vec<Vec<usize>> {
    let mut table = vec![vec![0; new_lines.len() + 1]; old_lines.len() + 1];

    for i in 0..old_lines.len() {
        for j in 0..new_lines.len() {
            table[i + 1][j + 1] = if old_lines[i] == new_lines[j] {
                table[i][j] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }

    table
}

fn backtrack_removed(
    old_lines: &[&str],
    new_lines: &[&str],
    table: &[Vec<usize>],
    i: usize,
    j: usize,
    removed: &mut Vec<RemovedLine>,
) {
    if i == 0 {
        return;
    }

    if j == 0 {
        backtrack_removed(old_lines, new_lines, table, i - 1, 0, removed);
        removed.push(RemovedLine {
            line_number: i,
            content: old_lines[i - 1].to_string(),
        });
        return;
    }

    if old_lines[i - 1] == new_lines[j - 1] {
        backtrack_removed(old_lines, new_lines, table, i - 1, j - 1, removed);
    } else if table[i - 1][j] >= table[i][j - 1] {
        backtrack_removed(old_lines, new_lines, table, i - 1, j, removed);
        removed.push(RemovedLine {
            line_number: i,
            content: old_lines[i - 1].to_string(),
        });
    } else {
        backtrack_removed(old_lines, new_lines, table, i, j - 1, removed);
    }
}

fn frontmatter_range(content: &str) -> Option<(usize, usize)> {
    let mut lines = content.lines().enumerate();
    let first = lines.next()?;
    if first.1 != "---" {
        return None;
    }

    for (idx, line) in lines {
        if line == "---" {
            return Some((1, idx + 1));
        }
    }

    None
}

fn line_in_frontmatter(line_number: usize, range: Option<(usize, usize)>) -> bool {
    match range {
        Some((start, end)) => (start..=end).contains(&line_number),
        None => false,
    }
}

fn is_managed_field(line: &str) -> bool {
    let trimmed = line.trim_start();
    ["id:", "created:", "status:", "superseded_by:"]
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
}

fn emit_issues(issues: &[Issue], output_mode: OutputMode) -> Result<()> {
    match output_mode {
        OutputMode::Human => {
            if issues.is_empty() {
                println!("Context corpus is clean.");
            } else {
                for issue in issues {
                    println!(
                        "{} {} {} {}",
                        severity_label(issue.severity),
                        issue.code,
                        issue.file,
                        issue.message
                    );
                }
            }
        }
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&issues)?);
        }
        OutputMode::Porcelain => {
            for issue in issues {
                println!(
                    "{} {} {} {}",
                    severity_label(issue.severity).to_lowercase(),
                    issue.code,
                    issue.file,
                    issue.message
                );
            }
        }
    }
    Ok(())
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Warning => "WARNING",
        Severity::Error => "ERROR",
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{
        collect_issues, frontmatter_range, is_managed_field, line_in_frontmatter, removed_lines,
    };

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("ctx-phase6-check-{nanos}"))
    }

    #[test]
    fn computes_frontmatter_range() {
        let content = "---\nid: ctx-1\nstatus: current\n---\nbody\n";
        assert_eq!(frontmatter_range(content), Some((1, 4)));
        assert!(line_in_frontmatter(2, frontmatter_range(content)));
        assert!(!line_in_frontmatter(5, frontmatter_range(content)));
    }

    #[test]
    fn detects_managed_fields() {
        assert!(is_managed_field("status: current"));
        assert!(is_managed_field("  id: ctx-1"));
        assert!(!is_managed_field("concerns:"));
    }

    #[test]
    fn computes_removed_lines() {
        let old = "a\nb\nc\n";
        let new = "a\nc\n";
        let removed = removed_lines(old, Some(new));
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].line_number, 2);
        assert_eq!(removed[0].content, "b");
    }

    #[test]
    fn reports_invalid_frontmatter() {
        let base = unique_temp_dir();
        fs::create_dir_all(base.join(".context")).unwrap();
        fs::write(base.join(".context/bad.md"), "---\nnot: [valid\n---\n").unwrap();

        let issues = collect_issues(&base, false).unwrap();
        assert!(issues.iter().any(|issue| issue.code == "FRONTMATTER_INVALID"));

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn reports_staged_append_only_violation() {
        let base = unique_temp_dir();
        fs::create_dir_all(base.join(".context")).unwrap();

        run_git(&base, &["init"]);
        run_git(&base, &["config", "user.email", "ctx@example.com"]);
        run_git(&base, &["config", "user.name", "Ctx Test"]);

        let file = base.join(".context/note.md");
        fs::write(
            &file,
            "---\nid: ctx-1\ncreated: 2025-10-15T14:23:00Z\nstatus: current\nconcerns:\n- billing\nscope:\n  paths: []\n  components: []\nsuperseded_by: []\n---\nBody line\n",
        )
        .unwrap();
        run_git(&base, &["add", "."]);
        run_git(&base, &["commit", "-m", "initial"]);

        fs::write(
            &file,
            "---\nid: ctx-1\ncreated: 2025-10-15T14:23:00Z\nstatus: current\nconcerns:\n- billing\nscope:\n  paths: []\n  components: []\nsuperseded_by: []\n---\n",
        )
        .unwrap();
        run_git(&base, &["add", ".context/note.md"]);

        let issues = collect_issues(&base, false).unwrap();
        assert!(issues.iter().any(|issue| issue.code == "APPEND_ONLY_VIOLATION"));

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn reports_staged_managed_field_tampering() {
        let base = unique_temp_dir();
        fs::create_dir_all(base.join(".context")).unwrap();

        run_git(&base, &["init"]);
        run_git(&base, &["config", "user.email", "ctx@example.com"]);
        run_git(&base, &["config", "user.name", "Ctx Test"]);

        let file = base.join(".context/note.md");
        fs::write(
            &file,
            "---\nid: ctx-1\ncreated: 2025-10-15T14:23:00Z\nstatus: current\nconcerns:\n- billing\nscope:\n  paths: []\n  components: []\nsuperseded_by: []\n---\nBody line\n",
        )
        .unwrap();
        run_git(&base, &["add", "."]);
        run_git(&base, &["commit", "-m", "initial"]);

        fs::write(
            &file,
            "---\nid: ctx-1\ncreated: 2025-10-15T14:23:00Z\nstatus: superseded\nconcerns:\n- billing\nscope:\n  paths: []\n  components: []\nsuperseded_by: []\n---\nBody line\n",
        )
        .unwrap();
        run_git(&base, &["add", ".context/note.md"]);

        let issues = collect_issues(&base, false).unwrap();
        assert!(issues.iter().any(|issue| issue.code == "MANAGED_FIELD_TAMPERING"));

        fs::remove_dir_all(base).unwrap();
    }

    fn run_git(base: &Path, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(base)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {:?} failed", args);
    }
}
