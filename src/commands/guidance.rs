use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{cli::GuidanceArgs, output::OutputMode};

const START_MARKER: &str = "<!-- ctx-guidance:start -->";
const END_MARKER: &str = "<!-- ctx-guidance:end -->";

#[derive(Debug, Serialize)]
struct GuidancePayload<'a> {
    guidance: &'a str,
    updated_files: Vec<String>,
}

pub fn run(args: GuidanceArgs, output_mode: OutputMode) -> Result<()> {
    let guidance = guidance_text();
    let updated_files = if args.add {
        upsert_agents_files(Path::new("."), guidance)?
    } else {
        Vec::new()
    };

    match output_mode {
        OutputMode::Human => {
            print!("{guidance}");
            if !guidance.ends_with('\n') {
                println!();
            }
            if args.add {
                println!();
                if updated_files.is_empty() {
                    println!("No AGENTS.md files updated.");
                } else {
                    println!("Updated AGENTS.md files:");
                    for file in &updated_files {
                        println!("{file}");
                    }
                }
            }
        }
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&GuidancePayload {
                    guidance,
                    updated_files,
                })?
            );
        }
        OutputMode::Porcelain => {
            println!("{START_MARKER}");
            print!("{guidance}");
            if !guidance.ends_with('\n') {
                println!();
            }
            println!("{END_MARKER}");
            for file in &updated_files {
                println!("updated\t{file}");
            }
        }
    }

    Ok(())
}

fn guidance_text() -> &'static str {
    "ctx guidance

- .context/ is managed by ctx.
- Do not directly edit .context documents except for recovery or repair work.
- Use ctx assemble before relevant work.
- Use ctx new, ctx append, and ctx supersede for context updates.
- Capture enough detail that a later agent can act without another interview.
- Prefer semantic coverage over verbosity.
- For each concern, try to record: the current claim, why it is true, what it depends on, what it excludes, and what would cause it to be superseded.
- Include decisions, assumptions, constraints, tradeoffs, and concrete examples when they remove ambiguity.
- Do not overfit the context to incidental implementation details that will churn quickly.
- Run ctx check after context changes.
- Respect .contextignore when deciding what belongs in managed context.
"
}

fn upsert_agents_files(base: &Path, guidance: &str) -> Result<Vec<String>> {
    let mut paths = find_agents_files(base)?;
    if paths.is_empty() {
        paths.push(base.join("AGENTS.md"));
    }

    let block = guidance_block(guidance);
    let mut updated = Vec::new();
    for path in paths {
        upsert_guidance_block(&path, &block)?;
        let relative = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        updated.push(relative);
    }

    updated.sort();
    updated.dedup();
    Ok(updated)
}

fn find_agents_files(base: &Path) -> Result<Vec<PathBuf>> {
    let mut results = Vec::new();
    visit_dirs(base, &mut results)?;
    results.sort();
    results.dedup();
    Ok(results)
}

fn visit_dirs(dir: &Path, results: &mut Vec<PathBuf>) -> Result<()> {
    for entry in
        fs::read_dir(dir).with_context(|| format!("failed to read directory {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to read file type for {}", path.display()))?;

        if file_type.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name == ".git" || name == "target" {
                continue;
            }
            visit_dirs(&path, results)?;
            continue;
        }

        if file_type.is_file() && entry.file_name().to_string_lossy() == "AGENTS.md" {
            results.push(path);
        }
    }

    Ok(())
}

fn guidance_block(guidance: &str) -> String {
    format!("{START_MARKER}\n## ctx\n\n{guidance}{END_MARKER}\n")
}

fn upsert_guidance_block(path: &Path, block: &str) -> Result<()> {
    let content = if path.exists() {
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?
    } else {
        String::new()
    };

    let updated = if let Some(start) = content.find(START_MARKER) {
        let end = content.find(END_MARKER).with_context(|| {
            format!("missing closing ctx guidance marker in {}", path.display())
        })?;
        let end_index = end + END_MARKER.len();
        let mut merged = String::new();
        merged.push_str(&content[..start]);
        if !merged.is_empty() && !merged.ends_with('\n') {
            merged.push('\n');
        }
        merged.push_str(block);
        if end_index < content.len() {
            let suffix = content[end_index..].trim_start_matches('\n');
            if !suffix.is_empty() {
                merged.push('\n');
                merged.push_str(suffix);
                if !merged.ends_with('\n') {
                    merged.push('\n');
                }
            }
        }
        merged
    } else if content.trim().is_empty() {
        block.to_string()
    } else {
        let mut merged = content;
        if !merged.ends_with('\n') {
            merged.push('\n');
        }
        merged.push('\n');
        merged.push_str(block);
        merged
    };

    fs::write(path, updated).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{find_agents_files, guidance_block, guidance_text, upsert_agents_files};

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("ctx-guidance-{nanos}"))
    }

    #[test]
    fn creates_root_agents_file_when_missing() {
        let base = unique_temp_dir();
        fs::create_dir_all(&base).unwrap();

        let updated = upsert_agents_files(&base, guidance_text()).unwrap();

        assert_eq!(updated, vec!["AGENTS.md".to_string()]);
        let content = fs::read_to_string(base.join("AGENTS.md")).unwrap();
        assert!(content.contains("Do not directly edit .context documents"));
        assert!(content.contains("ctx assemble before relevant work"));
        assert!(content.contains("Capture enough detail that a later agent can act"));
        assert!(content.contains("Prefer semantic coverage over verbosity"));
        assert!(content.contains("what would cause it to be superseded"));

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn updates_existing_agents_files() {
        let base = unique_temp_dir();
        let nested = base.join("docs");
        fs::create_dir_all(&nested).unwrap();
        fs::write(
            nested.join("AGENTS.md"),
            "# Agents\n\nOld text\n\n<!-- ctx-guidance:start -->\nold\n<!-- ctx-guidance:end -->\n",
        )
        .unwrap();

        let updated = upsert_agents_files(&base, guidance_text()).unwrap();

        assert_eq!(updated, vec!["docs/AGENTS.md".to_string()]);
        let content = fs::read_to_string(nested.join("AGENTS.md")).unwrap();
        assert!(content.contains("Use ctx new, ctx append, and ctx supersede"));
        assert!(!content.contains("\nold\n"));

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn finds_agents_files_recursively() {
        let base = unique_temp_dir();
        fs::create_dir_all(base.join("a/b")).unwrap();
        fs::write(base.join("a/b/AGENTS.md"), guidance_block(guidance_text())).unwrap();

        let files = find_agents_files(&base).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("a/b/AGENTS.md"));

        fs::remove_dir_all(base).unwrap();
    }
}
