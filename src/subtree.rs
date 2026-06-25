use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::ignore::ContextTraversalIgnore;

pub const SYNTHESIZED_CHILD_CONTEXT_FILE: &str = "ctx-child-context.md";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextRoot {
    pub base: PathBuf,
    pub relative: PathBuf,
}

pub fn child_context_roots(base: &Path) -> Result<Vec<ContextRoot>> {
    let exclude = ContextTraversalIgnore::load_from(base)?;
    let mut roots = Vec::new();
    visit_child_context_roots(base, base, &exclude, &mut roots)?;
    roots.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok(roots)
}

pub fn subtree_context_roots(base: &Path) -> Result<Vec<ContextRoot>> {
    let mut roots = Vec::new();
    collect_subtree_context_roots(base, PathBuf::new(), &mut roots)?;
    roots.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok(roots)
}

pub fn context_dir_exists(base: &Path) -> bool {
    base.join(".context").is_dir()
}

pub fn synthesized_child_context_path(base: &Path) -> PathBuf {
    base.join(".context").join(SYNTHESIZED_CHILD_CONTEXT_FILE)
}

pub fn is_synthesized_child_context_path(path: &Path) -> bool {
    path.file_name() == Some(OsStr::new(SYNTHESIZED_CHILD_CONTEXT_FILE))
}

pub fn rebase_scope_path(relative: &Path, scope_path: &str) -> String {
    let normalized_scope = normalize(scope_path);
    if relative.as_os_str().is_empty() {
        return normalized_scope;
    }

    let prefix = normalize(&relative.to_string_lossy());
    if normalized_scope.is_empty() {
        prefix
    } else {
        format!("{prefix}/{normalized_scope}")
    }
}

fn collect_subtree_context_roots(
    base: &Path,
    relative: PathBuf,
    roots: &mut Vec<ContextRoot>,
) -> Result<()> {
    if context_dir_exists(base) {
        roots.push(ContextRoot {
            base: base.to_path_buf(),
            relative: relative.clone(),
        });
    }

    for child in child_context_roots(base)? {
        let child_relative = relative.join(&child.relative);
        collect_subtree_context_roots(&child.base, child_relative, roots)?;
    }

    Ok(())
}

fn visit_child_context_roots(
    origin: &Path,
    dir: &Path,
    exclude: &ContextTraversalIgnore,
    roots: &mut Vec<ContextRoot>,
) -> Result<()> {
    let mut entries = fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("failed to read entries in {}", dir.display()))?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to read file type for {}", path.display()))?;
        if !file_type.is_dir() {
            continue;
        }

        if entry.file_name() == OsStr::new(".context") {
            continue;
        }

        let relative = path
            .strip_prefix(origin)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        if exclude.matches_dir(&relative) {
            continue;
        }

        if context_dir_exists(&path) {
            roots.push(ContextRoot {
                base: path,
                relative: PathBuf::from(relative),
            });
            continue;
        }

        visit_child_context_roots(origin, &path, exclude, roots)?;
    }

    Ok(())
}

fn normalize(path: &str) -> String {
    path.trim().trim_start_matches("./").replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{
        ContextRoot, SYNTHESIZED_CHILD_CONTEXT_FILE, child_context_roots, rebase_scope_path,
        subtree_context_roots, synthesized_child_context_path,
    };

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("ctx-subtree-{nanos}"))
    }

    #[test]
    fn finds_nearest_child_context_roots_only() {
        let base = unique_temp_dir();
        fs::create_dir_all(base.join(".context")).unwrap();
        fs::create_dir_all(base.join("apps/api/.context")).unwrap();
        fs::create_dir_all(base.join("apps/api/services/auth/.context")).unwrap();
        fs::create_dir_all(base.join("packages/ui/.context")).unwrap();

        let roots = child_context_roots(&base).unwrap();
        assert_eq!(
            roots,
            vec![
                ContextRoot {
                    base: base.join("apps/api"),
                    relative: PathBuf::from("apps/api"),
                },
                ContextRoot {
                    base: base.join("packages/ui"),
                    relative: PathBuf::from("packages/ui"),
                },
            ]
        );

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn respects_contextrc_when_finding_child_roots() {
        let base = unique_temp_dir();
        fs::create_dir_all(base.join(".context")).unwrap();
        fs::create_dir_all(base.join("vendor/sdk/.context")).unwrap();
        fs::create_dir_all(base.join("apps/api/.context")).unwrap();
        fs::write(base.join(".contextrc"), "vendor/**\n").unwrap();

        let roots = child_context_roots(&base).unwrap();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].relative, PathBuf::from("apps/api"));

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn finds_all_context_roots_in_subtree() {
        let base = unique_temp_dir();
        fs::create_dir_all(base.join(".context")).unwrap();
        fs::create_dir_all(base.join("apps/api/.context")).unwrap();
        fs::create_dir_all(base.join("apps/api/services/auth/.context")).unwrap();
        fs::create_dir_all(base.join("packages/ui/.context")).unwrap();

        let roots = subtree_context_roots(&base).unwrap();
        let relatives = roots
            .iter()
            .map(|root| root.relative.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            relatives,
            vec![
                PathBuf::new(),
                PathBuf::from("apps/api"),
                PathBuf::from("apps/api/services/auth"),
                PathBuf::from("packages/ui"),
            ]
        );

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn rebases_scope_paths_for_descendant_context() {
        assert_eq!(
            rebase_scope_path(Path::new("apps/api"), "src/**"),
            "apps/api/src/**"
        );
        assert_eq!(rebase_scope_path(Path::new(""), "src/**"), "src/**");
    }

    #[test]
    fn returns_synthesized_child_context_file_path() {
        let base = unique_temp_dir();
        assert_eq!(
            synthesized_child_context_path(&base),
            base.join(".context").join(SYNTHESIZED_CHILD_CONTEXT_FILE)
        );
    }
}
