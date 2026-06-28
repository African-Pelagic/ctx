use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;

use crate::{
    cli::PublishArgs,
    document::{extract_concern_section, parse_document},
    output::OutputMode,
    registry::{Registry, load_or_sync},
};

// ── Public entry points ────────────────────────────────────────────────────────

pub fn run(args: PublishArgs, output_mode: OutputMode) -> Result<()> {
    let registry = load_or_sync()?;
    let results = publish(&registry, args.concern.as_deref())?;
    emit(&results, output_mode)
}

// ── Core logic ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PublishResult {
    pub concern: String,
    pub file: String,
    pub source_doc: String,
}

pub fn publish(registry: &Registry, concern_filter: Option<&str>) -> Result<Vec<PublishResult>> {
    let concerns: Vec<&str> = if let Some(name) = concern_filter {
        if !registry.concern_roster.contains_key(name) {
            bail!("concern '{name}' not found in corpus");
        }
        vec![name]
    } else {
        let mut all: Vec<&str> = registry.concern_roster.keys().map(String::as_str).collect();
        all.sort();
        all
    };

    let mut results = Vec::new();

    for concern in concerns {
        let owners = &registry.concern_roster[concern].owners;
        // prefer the most recently created owner (last in sorted order = newest id prefix by time)
        let owner_id = owners.last().context("concern has no owners")?;
        let entry = registry
            .documents
            .get(owner_id)
            .context("owner document not found in registry")?;

        let content = fs::read_to_string(&entry.file)
            .with_context(|| format!("failed to read {}", entry.file))?;
        let (_, body) = parse_document(&content)
            .with_context(|| format!("failed to parse frontmatter in {}", entry.file))?;

        let section = extract_concern_section(&body, concern)
            .with_context(|| format!("concern '{concern}' section not found in {}", entry.file))?;

        let org = render_org(concern, owner_id, &strip_heading(&section));
        let out_path = org_path(concern);
        fs::write(&out_path, &org)
            .with_context(|| format!("failed to write {}", out_path.display()))?;

        results.push(PublishResult {
            concern: concern.to_string(),
            file: out_path.to_string_lossy().into_owned(),
            source_doc: owner_id.clone(),
        });
    }

    Ok(results)
}

// ── Org rendering ──────────────────────────────────────────────────────────────

fn render_org(concern: &str, source_doc: &str, body: &str) -> String {
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let mut out = String::new();

    out.push_str(&format!("#+TITLE: {concern}\n"));
    out.push_str(&format!("#+PROPERTY: PUBLISHED {now}\n"));
    out.push_str(&format!("#+PROPERTY: SOURCE_DOC {source_doc}\n"));
    out.push('\n');
    out.push_str(&markdown_to_org(body));

    out
}

/// Strip the `### concern-name [rN]` heading line from an extracted section body.
fn strip_heading(section: &str) -> String {
    let mut lines = section.lines();
    // skip the heading line (### ...)
    lines.next();
    // skip any immediately following blank line
    let rest: Vec<&str> = lines.collect();
    let trimmed = rest.join("\n");
    trimmed.trim_start_matches('\n').to_string()
}

/// Convert a markdown body (as used in CTX concern sections) to Org prose.
///
/// Conversions applied:
///   ### heading  → * heading
///   ## heading   → * heading
///   # heading    → * heading
///   **text**     → *text*      (bold)
///   *text*       → /text/      (italic, only single asterisk not part of bold)
///   `code`       → =code=
///   ```block```  → #+BEGIN_SRC / #+END_SRC
///   - list item  → - list item  (unchanged, Org uses same syntax)
///   [text](url)  → [[url][text]]
fn markdown_to_org(src: &str) -> String {
    let mut out = String::new();
    let mut in_code_block = false;

    for line in src.lines() {
        // fenced code blocks
        if line.trim_start().starts_with("```") {
            if in_code_block {
                out.push_str("#+END_SRC\n");
                in_code_block = false;
            } else {
                let lang = line.trim_start().trim_start_matches('`').trim();
                if lang.is_empty() {
                    out.push_str("#+BEGIN_SRC\n");
                } else {
                    out.push_str(&format!("#+BEGIN_SRC {lang}\n"));
                }
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            out.push_str(line);
            out.push('\n');
            continue;
        }

        // ATX headings: ###, ##, # → Org * headings
        if let Some(rest) = line.strip_prefix("### ") {
            out.push_str(&format!("*** {}\n", inline_markdown_to_org(rest)));
            continue;
        }
        if let Some(rest) = line.strip_prefix("## ") {
            out.push_str(&format!("** {}\n", inline_markdown_to_org(rest)));
            continue;
        }
        if let Some(rest) = line.strip_prefix("# ") {
            out.push_str(&format!("* {}\n", inline_markdown_to_org(rest)));
            continue;
        }

        // Everything else: apply inline conversions
        out.push_str(&inline_markdown_to_org(line));
        out.push('\n');
    }

    if in_code_block {
        out.push_str("#+END_SRC\n");
    }

    out
}

/// Apply inline markdown → Org conversions to a single line of text.
fn inline_markdown_to_org(src: &str) -> String {
    let s = convert_links(src);
    let s = convert_bold(&s);
    let s = convert_code(&s);
    s
}

/// `[text](url)` → `[[url][text]]`
fn convert_links(src: &str) -> String {
    let mut out = String::new();
    let mut rest = src;

    while let Some(bracket_open) = rest.find('[') {
        // push everything before this '['
        out.push_str(&rest[..bracket_open]);
        rest = &rest[bracket_open..];

        // try to match [text](url)
        if let Some(bracket_close) = rest.find("](") {
            let text = &rest[1..bracket_close];
            let after_bracket = &rest[bracket_close + 2..];
            if let Some(paren_close) = after_bracket.find(')') {
                let url = &after_bracket[..paren_close];
                out.push_str(&format!("[[{url}][{text}]]"));
                rest = &after_bracket[paren_close + 1..];
                continue;
            }
        }

        // not a link — push the '[' and move on
        out.push('[');
        rest = &rest[1..];
    }

    out.push_str(rest);
    out
}

/// `**text**` → `*text*`
fn convert_bold(src: &str) -> String {
    let mut out = String::new();
    let mut rest = src;

    while let Some(pos) = rest.find("**") {
        out.push_str(&rest[..pos]);
        rest = &rest[pos + 2..];
        if let Some(end) = rest.find("**") {
            let inner = &rest[..end];
            out.push('*');
            out.push_str(inner);
            out.push('*');
            rest = &rest[end + 2..];
        } else {
            // unmatched — put back the ** and stop
            out.push_str("**");
            out.push_str(rest);
            return out;
        }
    }

    out.push_str(rest);
    out
}

/// `` `code` `` → `=code=`
fn convert_code(src: &str) -> String {
    let mut out = String::new();
    let mut rest = src;

    while let Some(pos) = rest.find('`') {
        out.push_str(&rest[..pos]);
        rest = &rest[pos + 1..];
        if let Some(end) = rest.find('`') {
            let inner = &rest[..end];
            out.push('=');
            out.push_str(inner);
            out.push('=');
            rest = &rest[end + 1..];
        } else {
            // unmatched backtick
            out.push('`');
            out.push_str(rest);
            return out;
        }
    }

    out.push_str(rest);
    out
}

// ── Path helpers ───────────────────────────────────────────────────────────────

pub fn org_path(concern: &str) -> PathBuf {
    // Sanitise concern name for use as filename
    let filename = concern.replace(['/', '\\', ':', ' '], "-");
    Path::new(".").join(format!("{filename}.org"))
}

/// List all .org files in the corpus root (cwd) that were likely written by ctx publish.
pub fn existing_org_files() -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let pattern = "./*.org";
    for entry in glob::glob(pattern).context("invalid org glob pattern")? {
        files.push(entry.context("failed to enumerate org files")?);
    }
    files.sort();
    Ok(files)
}

pub fn concern_name_from_org_path(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.replace('-', "-"))
}

// ── Emit ───────────────────────────────────────────────────────────────────────

fn emit(results: &[PublishResult], output_mode: OutputMode) -> Result<()> {
    match output_mode {
        OutputMode::Human => {
            if results.is_empty() {
                println!("No concerns to publish");
            } else {
                for r in results {
                    println!("Published {} → {}", r.concern, r.file);
                }
            }
        }
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(results)?);
        }
        OutputMode::Porcelain => {
            for r in results {
                println!("{}\t{}\t{}", r.concern, r.file, r.source_doc);
            }
        }
    }
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_bold() {
        assert_eq!(convert_bold("**hello** world"), "*hello* world");
        assert_eq!(convert_bold("no bold here"), "no bold here");
        assert_eq!(convert_bold("**a** and **b**"), "*a* and *b*");
    }

    #[test]
    fn converts_inline_code() {
        assert_eq!(convert_code("`ctx publish`"), "=ctx publish=");
        assert_eq!(convert_code("run `foo` and `bar`"), "run =foo= and =bar=");
    }

    #[test]
    fn converts_markdown_links() {
        assert_eq!(
            convert_links("[ctx](https://example.com)"),
            "[[https://example.com][ctx]]"
        );
        assert_eq!(convert_links("no links here"), "no links here");
    }

    #[test]
    fn converts_headings() {
        let md = "### sub\n## section\n# top\n";
        let org = markdown_to_org(md);
        assert!(org.contains("*** sub\n"));
        assert!(org.contains("** section\n"));
        assert!(org.contains("* top\n"));
    }

    #[test]
    fn converts_fenced_code_block() {
        let md = "```rust\nfn main() {}\n```\n";
        let org = markdown_to_org(md);
        assert!(org.contains("#+BEGIN_SRC rust\n"));
        assert!(org.contains("fn main() {}\n"));
        assert!(org.contains("#+END_SRC\n"));
    }

    #[test]
    fn strips_heading_line() {
        let section = "### my-concern [r3]\n\nBody text here.\n";
        assert_eq!(strip_heading(section), "Body text here.");
    }

    #[test]
    fn render_org_has_correct_headers() {
        let org = render_org("my-concern", "ctx-abc123", "Some body text.");
        assert!(org.starts_with("#+TITLE: my-concern\n"));
        assert!(org.contains("#+PROPERTY: SOURCE_DOC ctx-abc123\n"));
        assert!(org.contains("Some body text."));
    }
}
