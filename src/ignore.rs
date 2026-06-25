use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::{Context, Result};
use glob::Pattern;

#[derive(Clone, Debug, Default)]
pub struct ContextIgnore {
    patterns: Vec<Pattern>,
}

#[derive(Clone, Debug, Default)]
pub struct ContextTraversalIgnore {
    patterns: Vec<Pattern>,
}

impl ContextIgnore {
    pub fn load_from(base: &Path) -> Result<Self> {
        Ok(Self {
            patterns: load_patterns(&contextignore_path_from(base), ".contextignore")?,
        })
    }

    pub fn matches(&self, path: &str) -> bool {
        let normalized = normalize(path);
        self.patterns
            .iter()
            .any(|pattern| pattern.matches(&normalized))
    }
}

impl ContextTraversalIgnore {
    pub fn load_from(base: &Path) -> Result<Self> {
        Ok(Self {
            patterns: load_patterns(&contextrc_path_from(base), ".contextrc")?,
        })
    }

    pub fn matches_dir(&self, path: &str) -> bool {
        let normalized = normalize(path);
        self.patterns.iter().any(|pattern| {
            pattern.matches(&normalized)
                || pattern.matches(&format!("{normalized}/"))
                || pattern.matches(&format!("{normalized}/**"))
        })
    }
}

pub fn contextignore_path_from(base: &Path) -> PathBuf {
    base.join(".contextignore")
}

pub fn contextrc_path_from(base: &Path) -> PathBuf {
    base.join(".contextrc")
}

pub fn requires_refresh(base: &Path, derived_file: &Path) -> bool {
    let ignore_path = contextignore_path_from(base);
    let ignore_modified = modified(&ignore_path);
    let derived_modified = modified(derived_file);

    match (ignore_modified, derived_modified) {
        (Some(ignore_time), Some(derived_time)) => ignore_time > derived_time,
        _ => false,
    }
}

fn normalize(path: &str) -> String {
    path.trim().trim_start_matches("./").replace('\\', "/")
}

fn load_patterns(path: &Path, label: &str) -> Result<Vec<Pattern>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut patterns = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let pattern =
            Pattern::new(trimmed).with_context(|| format!("invalid {label} pattern {trimmed}"))?;
        patterns.push(pattern);
    }

    Ok(patterns)
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{ContextIgnore, ContextTraversalIgnore};

    fn unique_temp_dir() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("ctx-ignore-{nanos}"))
    }

    #[test]
    fn loads_and_matches_patterns() {
        let base = unique_temp_dir();
        fs::create_dir_all(&base).unwrap();
        fs::write(
            base.join(".contextignore"),
            "# comment\n.context/private.md\nsecrets/**\n",
        )
        .unwrap();

        let ignore = ContextIgnore::load_from(&base).unwrap();
        assert!(ignore.matches(".context/private.md"));
        assert!(ignore.matches("secrets/prod.env"));
        assert!(!ignore.matches("src/main.rs"));

        fs::remove_file(base.join(".contextignore")).unwrap();
    }

    #[test]
    fn loads_and_matches_traversal_patterns() {
        let base = unique_temp_dir();
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join(".contextrc"), "# comment\nvendor/**\ntmp\n").unwrap();

        let ignore = ContextTraversalIgnore::load_from(&base).unwrap();
        assert!(ignore.matches_dir("vendor"));
        assert!(ignore.matches_dir("vendor/lib"));
        assert!(ignore.matches_dir("tmp"));
        assert!(!ignore.matches_dir("src"));

        fs::remove_file(base.join(".contextrc")).unwrap();
    }
}
