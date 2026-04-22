use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use glob::Pattern;
use serde::{Deserialize, Serialize};

use crate::{
    git::{commit_count_since_in, current_commit_short_in, last_modified_in, repo_files},
    registry::{Registry, load_or_sync_from},
};

pub const INDEX_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IndexedDocument {
    pub file: String,
    pub active_concerns: Vec<String>,
    pub components: Vec<String>,
    pub scope_paths: Vec<String>,
    pub matched_repo_paths: Vec<String>,
    pub missing_scope_paths: Vec<String>,
    pub last_updated: Option<DateTime<Utc>>,
    pub commits_in_scope_since_update: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CodeIndex {
    pub schema_version: u32,
    pub generated_at: DateTime<Utc>,
    pub generated_from_commit: Option<String>,
    pub repo_files: Vec<String>,
    pub documents: BTreeMap<String, IndexedDocument>,
}

impl CodeIndex {
    pub fn build(base: &Path, registry: &Registry) -> CodeIndex {
        let repo_files = repo_files(base);
        let mut documents = BTreeMap::new();

        for (id, entry) in &registry.documents {
            let (matched_repo_paths, missing_scope_paths) =
                match_scope_paths(&entry.scope.paths, &repo_files);
            let last_updated = last_modified_in(base, Path::new(&entry.file));
            let commits_in_scope_since_update = last_updated
                .as_ref()
                .and_then(|stamp| commit_count_since_in(base, stamp, &entry.scope.paths));

            documents.insert(
                id.clone(),
                IndexedDocument {
                    file: entry.file.clone(),
                    active_concerns: entry.active_concerns.clone(),
                    components: entry.scope.components.clone(),
                    scope_paths: entry.scope.paths.clone(),
                    matched_repo_paths,
                    missing_scope_paths,
                    last_updated,
                    commits_in_scope_since_update,
                },
            );
        }

        CodeIndex {
            schema_version: INDEX_SCHEMA_VERSION,
            generated_at: Utc::now(),
            generated_from_commit: current_commit_short_in(base),
            repo_files,
            documents,
        }
    }

    pub fn load(path: &Path) -> Result<CodeIndex> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read index at {}", path.display()))?;
        let index = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse index at {}", path.display()))?;
        Ok(index)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }

        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)
            .with_context(|| format!("failed to write index to {}", path.display()))?;
        Ok(())
    }
}

pub fn index_path() -> PathBuf {
    index_path_from(Path::new("."))
}

pub fn index_path_from(base: &Path) -> PathBuf {
    base.join(".context").join(".index.json")
}

pub fn build_index_from(base: &Path) -> Result<CodeIndex> {
    let registry = load_or_sync_from(base)?;
    Ok(CodeIndex::build(base, &registry))
}

pub fn refresh_index_from(base: &Path) -> Result<CodeIndex> {
    let index = build_index_from(base)?;
    index.save(&index_path_from(base))?;
    Ok(index)
}

pub fn load_or_build_index_from(base: &Path) -> Result<CodeIndex> {
    let path = index_path_from(base);
    if path.exists() {
        let index = CodeIndex::load(&path)?;
        if index.generated_from_commit == current_commit_short_in(base) {
            return Ok(index);
        }
    }

    build_index_from(base)
}

fn match_scope_paths(scope_paths: &[String], repo_files: &[String]) -> (Vec<String>, Vec<String>) {
    let mut matched_repo_paths = Vec::new();
    let mut missing_scope_paths = Vec::new();

    for scope_path in scope_paths {
        let pattern = match Pattern::new(scope_path) {
            Ok(pattern) => pattern,
            Err(_) => {
                missing_scope_paths.push(scope_path.clone());
                continue;
            }
        };

        let mut matched_for_pattern = repo_files
            .iter()
            .filter(|repo_path| pattern.matches(repo_path))
            .cloned()
            .collect::<Vec<_>>();

        if matched_for_pattern.is_empty() {
            missing_scope_paths.push(scope_path.clone());
        } else {
            matched_repo_paths.append(&mut matched_for_pattern);
        }
    }

    matched_repo_paths.sort();
    matched_repo_paths.dedup();
    missing_scope_paths.sort();
    missing_scope_paths.dedup();
    (matched_repo_paths, missing_scope_paths)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{INDEX_SCHEMA_VERSION, build_index_from};

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("ctx-phase8-index-{nanos}"))
    }

    #[test]
    fn builds_code_index_with_matched_and_missing_paths() {
        let base = unique_temp_dir();
        fs::create_dir_all(base.join(".context")).unwrap();
        fs::create_dir_all(base.join("src")).unwrap();
        fs::write(base.join("src/lib.rs"), "pub fn value() {}\n").unwrap();
        fs::write(
            base.join(".context/note.md"),
            "---\nid: ctx-1\ncreated: 2025-10-15T14:23:00Z\nstatus: current\nconcerns:\n- billing\nscope:\n  paths:\n  - src/*.rs\n  - src/missing/**\n  components:\n  - ctx-cli\nsuperseded_by: []\n---\nBody\n",
        )
        .unwrap();

        run_git(&base, &["init"]);
        run_git(&base, &["config", "user.email", "ctx@example.com"]);
        run_git(&base, &["config", "user.name", "Ctx Test"]);
        run_git(&base, &["add", "."]);
        run_git(&base, &["commit", "-m", "initial"]);

        let index = build_index_from(&base).unwrap();
        let doc = &index.documents["ctx-1"];

        assert_eq!(index.schema_version, INDEX_SCHEMA_VERSION);
        assert!(doc.matched_repo_paths.contains(&"src/lib.rs".to_string()));
        assert_eq!(doc.missing_scope_paths, vec!["src/missing/**".to_string()]);

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
