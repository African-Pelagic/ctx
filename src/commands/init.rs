use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde_json::json;

use crate::{output::OutputMode, registry::{context_dir, registry_path, Registry, SCHEMA_VERSION}};

pub fn run(output_mode: OutputMode) -> Result<()> {
    fs::create_dir_all(context_dir()).context("failed to create .context directory")?;

    let registry = Registry {
        schema_version: SCHEMA_VERSION,
        generated_at: chrono::Utc::now(),
        generated_from_commit: None,
        documents: Default::default(),
        concern_roster: Default::default(),
        orphaned_concerns: Default::default(),
        multi_owned_concerns: Default::default(),
    };
    registry.save(&registry_path())?;

    ensure_gitignore_entry(Path::new(".gitignore"), ".context/.registry.json")?;

    match output_mode {
        OutputMode::Human => {
            println!("Initialized context corpus in .context/");
        }
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "context_dir": ".context",
                    "registry": ".context/.registry.json"
                }))?
            );
        }
        OutputMode::Porcelain => {
            println!(".context");
            println!(".context/.registry.json");
        }
    }

    Ok(())
}

fn ensure_gitignore_entry(path: &Path, entry: &str) -> Result<()> {
    let mut content = if path.exists() {
        fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?
    } else {
        String::new()
    };

    if content.lines().any(|line| line.trim() == entry) {
        return Ok(());
    }

    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(entry);
    content.push('\n');

    fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{env, fs, time::{SystemTime, UNIX_EPOCH}};

    use super::ensure_gitignore_entry;

    fn unique_path(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("ctx-{name}-{nanos}"))
    }

    #[test]
    fn appends_gitignore_entry_once() {
        let path = unique_path("gitignore");
        fs::write(&path, "/target\n").unwrap();

        ensure_gitignore_entry(&path, ".context/.registry.json").unwrap();
        ensure_gitignore_entry(&path, ".context/.registry.json").unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "/target\n.context/.registry.json\n");

        fs::remove_file(path).unwrap();
    }
}
