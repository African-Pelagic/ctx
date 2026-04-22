use std::{
    path::{Path, PathBuf},
    process::Command,
};

use chrono::{DateTime, Utc};

pub fn last_modified_in(base: &Path, path: &Path) -> Option<DateTime<Utc>> {
    let output = Command::new("git")
        .current_dir(base)
        .args(["log", "-1", "--format=%aI", "--"])
        .arg(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8(output.stdout).ok()?;
    let stamp = text.trim();
    if stamp.is_empty() {
        return None;
    }

    chrono::DateTime::parse_from_rfc3339(stamp)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

pub fn commit_count_since_in(
    base: &Path,
    since: &DateTime<Utc>,
    paths: &[String],
) -> Option<usize> {
    if paths.is_empty() {
        return Some(0);
    }

    let mut cmd = Command::new("git");
    cmd.current_dir(base);
    cmd.args([
        "log",
        "--oneline",
        &format!("{}..HEAD", since.to_rfc3339()),
        "--",
    ]);
    for path in paths {
        cmd.arg(path);
    }

    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8(output.stdout).ok()?;
    Some(text.lines().filter(|line| !line.trim().is_empty()).count())
}

pub fn is_stale(doc_file: &Path, scope_paths: &[String]) -> bool {
    is_stale_in(Path::new("."), doc_file, scope_paths)
}

pub fn is_stale_in(base: &Path, doc_file: &Path, scope_paths: &[String]) -> bool {
    let last_modified = match last_modified_in(base, doc_file) {
        Some(value) => value,
        None => return false,
    };

    let age = Utc::now() - last_modified;
    if age.num_days() < 30 {
        return false;
    }

    match commit_count_since_in(base, &last_modified, scope_paths) {
        Some(count) => count >= 10,
        None => false,
    }
}

pub fn current_commit_short() -> Option<String> {
    current_commit_short_in(Path::new("."))
}

pub fn current_commit_short_in(base: &Path) -> Option<String> {
    let output = Command::new("git")
        .current_dir(base)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8(output.stdout).ok()?;
    let short = text.trim();
    if short.is_empty() {
        None
    } else {
        Some(short.to_string())
    }
}

pub fn repo_files(base: &Path) -> Vec<String> {
    let output = match Command::new("git")
        .current_dir(base)
        .args(["ls-files", "--cached", "--others", "--exclude-standard"])
        .output()
    {
        Ok(output) => output,
        Err(_) => return Vec::new(),
    };

    if !output.status.success() {
        return Vec::new();
    }

    let mut files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    files.sort();
    files.dedup();
    files
}

pub fn doc_path(path: &str) -> PathBuf {
    PathBuf::from(path)
}
