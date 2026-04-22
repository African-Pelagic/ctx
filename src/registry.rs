use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    document::{Frontmatter, Scope, Status, SupersededBy, active_concerns, parse_document},
    git::current_commit_short,
    ignore::{ContextIgnore, requires_refresh},
};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DocumentEntry {
    pub file: String,
    pub created: DateTime<Utc>,
    pub status: Status,
    pub concerns: Vec<String>,
    pub active_concerns: Vec<String>,
    pub scope: Scope,
    pub superseded_by: Vec<SupersededBy>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConcernRosterEntry {
    pub owners: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Registry {
    pub schema_version: u32,
    pub generated_at: DateTime<Utc>,
    pub generated_from_commit: Option<String>,
    pub documents: BTreeMap<String, DocumentEntry>,
    pub concern_roster: BTreeMap<String, ConcernRosterEntry>,
    pub orphaned_concerns: Vec<String>,
    pub multi_owned_concerns: Vec<String>,
}

impl Registry {
    pub fn build(docs: &[(PathBuf, Frontmatter)]) -> Registry {
        let mut documents = BTreeMap::new();
        let mut concern_to_owners: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut superseded_concerns = BTreeSet::new();

        for (path, frontmatter) in docs {
            let active = active_concerns(frontmatter);

            for concern in &active {
                concern_to_owners
                    .entry(concern.clone())
                    .or_default()
                    .push(frontmatter.id.clone());
            }

            for entry in &frontmatter.superseded_by {
                superseded_concerns.extend(entry.concerns.iter().cloned());
            }

            documents.insert(
                frontmatter.id.clone(),
                DocumentEntry {
                    file: path.to_string_lossy().into_owned(),
                    created: frontmatter.created,
                    status: frontmatter.status.clone(),
                    concerns: frontmatter.concerns.clone(),
                    active_concerns: active,
                    scope: frontmatter.scope.clone(),
                    superseded_by: frontmatter.superseded_by.clone(),
                },
            );
        }

        for owners in concern_to_owners.values_mut() {
            owners.sort();
            owners.dedup();
        }

        let concern_roster = concern_to_owners
            .iter()
            .map(|(concern, owners)| {
                (
                    concern.clone(),
                    ConcernRosterEntry {
                        owners: owners.clone(),
                    },
                )
            })
            .collect();

        let orphaned_concerns = superseded_concerns
            .into_iter()
            .filter(|concern| !concern_to_owners.contains_key(concern))
            .collect();

        let multi_owned_concerns = concern_to_owners
            .into_iter()
            .filter_map(|(concern, owners)| (owners.len() > 1).then_some(concern))
            .collect();

        Registry {
            schema_version: SCHEMA_VERSION,
            generated_at: Utc::now(),
            generated_from_commit: current_commit_short(),
            documents,
            concern_roster,
            orphaned_concerns,
            multi_owned_concerns,
        }
    }

    pub fn load(path: &Path) -> Result<Registry> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read registry at {}", path.display()))?;
        let registry = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse registry at {}", path.display()))?;
        Ok(registry)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }

        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)
            .with_context(|| format!("failed to write registry to {}", path.display()))?;
        Ok(())
    }
}

pub fn context_dir() -> PathBuf {
    context_dir_from(Path::new("."))
}

pub fn context_dir_from(base: &Path) -> PathBuf {
    base.join(".context")
}

pub fn registry_path() -> PathBuf {
    registry_path_from(Path::new("."))
}

pub fn registry_path_from(base: &Path) -> PathBuf {
    context_dir_from(base).join(".registry.json")
}

pub fn collect_documents_from(base: &Path) -> Result<Vec<(PathBuf, Frontmatter)>> {
    let pattern = format!("{}/{}", context_dir_from(base).display(), "*.md");
    let mut docs = Vec::new();
    let ignore = ContextIgnore::load_from(base)?;

    for entry in glob::glob(&pattern).with_context(|| format!("invalid glob pattern {pattern}"))? {
        let path =
            entry.with_context(|| format!("failed to enumerate files matching {pattern}"))?;
        let relative = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        if ignore.matches(&relative) {
            continue;
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read document {}", path.display()))?;
        let (frontmatter, _) = parse_document(&content)
            .with_context(|| format!("failed to parse frontmatter in {}", path.display()))?;
        docs.push((path, frontmatter));
    }

    docs.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(docs)
}

pub fn sync_corpus() -> Result<Registry> {
    sync_corpus_from(Path::new("."))
}

pub fn sync_corpus_from(base: &Path) -> Result<Registry> {
    let docs = collect_documents_from(base)?;
    let registry = Registry::build(&docs);
    registry.save(&registry_path_from(base))?;
    Ok(registry)
}

pub fn load_or_sync() -> Result<Registry> {
    load_or_sync_from(Path::new("."))
}

pub fn load_or_sync_from(base: &Path) -> Result<Registry> {
    let path = registry_path_from(base);
    if path.exists() && !requires_refresh(base, &path) {
        Registry::load(&path)
    } else {
        sync_corpus_from(base)
    }
}

#[cfg(test)]
mod tests {
    use super::{Registry, SCHEMA_VERSION, sync_corpus_from};
    use std::{
        env, fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::{TimeZone, Utc};

    use crate::document::{Frontmatter, Scope, Status, SupersededBy};

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("ctx-test-{nanos}"))
    }

    fn write_doc(path: &Path, frontmatter: &Frontmatter) {
        let yaml = serde_yaml::to_string(frontmatter).unwrap();
        fs::write(path, format!("---\n{}---\nbody\n", yaml)).unwrap();
    }

    #[test]
    fn builds_registry_with_derived_views() {
        let created = Utc.with_ymd_and_hms(2025, 10, 15, 14, 23, 0).unwrap();
        let docs = vec![
            (
                PathBuf::from(".context/auth.md"),
                Frontmatter {
                    id: "ctx-a".into(),
                    created,
                    status: Status::Current,
                    concerns: vec!["authentication".into(), "token-expiry".into()],
                    scope: Scope {
                        paths: vec!["src/auth/**".into()],
                        components: vec!["auth-service".into()],
                    },
                    superseded_by: vec![SupersededBy {
                        id: "ctx-b".into(),
                        concerns: vec!["token-expiry".into()],
                    }],
                },
            ),
            (
                PathBuf::from(".context/billing.md"),
                Frontmatter {
                    id: "ctx-b".into(),
                    created,
                    status: Status::Current,
                    concerns: vec!["token-expiry".into(), "billing-auth".into()],
                    scope: Scope {
                        paths: vec!["src/billing/**".into()],
                        components: vec!["billing-service".into()],
                    },
                    superseded_by: vec![],
                },
            ),
        ];

        let registry = Registry::build(&docs);

        assert_eq!(registry.schema_version, SCHEMA_VERSION);
        assert_eq!(
            registry.documents["ctx-a"].active_concerns,
            vec!["authentication".to_string()]
        );
        assert_eq!(
            registry.concern_roster["token-expiry"].owners,
            vec!["ctx-b".to_string()]
        );
        assert_eq!(registry.orphaned_concerns, Vec::<String>::new());
        assert_eq!(registry.multi_owned_concerns, Vec::<String>::new());
    }

    #[test]
    fn syncs_corpus_from_context_directory() {
        let tmp = unique_temp_dir();
        fs::create_dir_all(tmp.join(".context")).unwrap();

        let created = Utc.with_ymd_and_hms(2025, 10, 15, 14, 23, 0).unwrap();
        let frontmatter = Frontmatter {
            id: "ctx-7f3a9b".into(),
            created,
            status: Status::Current,
            concerns: vec!["session-management".into()],
            scope: Scope {
                paths: vec!["src/sessions/**".into()],
                components: vec!["session-service".into()],
            },
            superseded_by: vec![],
        };
        write_doc(&tmp.join(".context/session.md"), &frontmatter);

        let registry = sync_corpus_from(&tmp).unwrap();

        assert_eq!(registry.documents.len(), 1);
        assert!(tmp.join(".context/.registry.json").exists());

        fs::remove_dir_all(tmp).unwrap();
    }

    #[test]
    fn skips_ignored_context_documents() {
        let tmp = unique_temp_dir();
        fs::create_dir_all(tmp.join(".context")).unwrap();
        fs::write(tmp.join(".contextignore"), ".context/private.md\n").unwrap();

        let created = Utc.with_ymd_and_hms(2025, 10, 15, 14, 23, 0).unwrap();
        let frontmatter = Frontmatter {
            id: "ctx-1".into(),
            created,
            status: Status::Current,
            concerns: vec!["public".into()],
            scope: Scope::default(),
            superseded_by: vec![],
        };
        write_doc(&tmp.join(".context/private.md"), &frontmatter);

        let registry = sync_corpus_from(&tmp).unwrap();
        assert!(registry.documents.is_empty());

        fs::remove_dir_all(tmp).unwrap();
    }
}
