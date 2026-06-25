use std::{env, fs, path::Path};

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::json;

use crate::{
    cli::SyncArgs,
    document::{
        Frontmatter, Scope, Status, extract_concern_section, format_ranked_concern_block,
        parse_document, write_document,
    },
    id::generate_id,
    output::OutputMode,
    registry::{Registry, load_or_sync_from, sync_corpus, sync_corpus_from},
    subtree::{
        ContextRoot, child_context_roots, context_dir_exists, rebase_scope_path,
        synthesized_child_context_path,
    },
};

const SYNTHESIS_RANK: u8 = 3;

#[derive(Debug, Default, Serialize)]
struct CascadeStats {
    contexts_synced: usize,
    synthesized_documents: usize,
    synthesized_concerns: usize,
}

#[derive(Clone, Debug)]
struct ChildSummary {
    concern: String,
    relative_root: String,
    active_document_count: usize,
    active_concerns: Vec<String>,
    components: Vec<String>,
    scope_paths: Vec<String>,
    concern_summaries: Vec<ConcernSummary>,
}

#[derive(Clone, Debug)]
struct ConcernSummary {
    concern: String,
    summary: String,
}

pub fn run(args: SyncArgs, output_mode: OutputMode) -> Result<()> {
    if args.cascade {
        let base = env::current_dir().context("failed to determine current directory")?;
        let stats = cascade_sync_from(&base)?;
        emit_cascade_result(&stats, output_mode)?;
    } else {
        let registry = sync_corpus()?;
        emit_sync_result(&registry, output_mode)?;
    }

    Ok(())
}

fn emit_sync_result(registry: &Registry, output_mode: OutputMode) -> Result<()> {
    match output_mode {
        OutputMode::Human => {
            println!(
                "Synced {} context document{}",
                registry.documents.len(),
                if registry.documents.len() == 1 {
                    ""
                } else {
                    "s"
                }
            );
        }
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "documents": registry.documents.len(),
                    "registry": ".context/.registry.json"
                }))?
            );
        }
        OutputMode::Porcelain => {
            println!("{}", registry.documents.len());
        }
    }

    Ok(())
}

fn emit_cascade_result(stats: &CascadeStats, output_mode: OutputMode) -> Result<()> {
    match output_mode {
        OutputMode::Human => {
            println!(
                "Cascaded {} context level{}, wrote {} synthesized document{}, and refreshed {} child concern{}",
                stats.contexts_synced,
                if stats.contexts_synced == 1 { "" } else { "s" },
                stats.synthesized_documents,
                if stats.synthesized_documents == 1 {
                    ""
                } else {
                    "s"
                },
                stats.synthesized_concerns,
                if stats.synthesized_concerns == 1 {
                    ""
                } else {
                    "s"
                }
            );
        }
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(stats)?);
        }
        OutputMode::Porcelain => {
            println!(
                "{}\t{}\t{}",
                stats.contexts_synced, stats.synthesized_documents, stats.synthesized_concerns
            );
        }
    }

    Ok(())
}

fn cascade_sync_from(base: &Path) -> Result<CascadeStats> {
    let mut stats = CascadeStats::default();
    cascade_sync_inner(base, &mut stats)?;
    Ok(stats)
}

fn cascade_sync_inner(base: &Path, stats: &mut CascadeStats) -> Result<()> {
    let children = child_context_roots(base)?;
    for child in &children {
        cascade_sync_inner(&child.base, stats)?;
    }

    if !context_dir_exists(base) {
        return Ok(());
    }

    let summary_count = synthesize_child_contexts(base, &children)?;
    sync_corpus_from(base)?;
    stats.contexts_synced += 1;
    if summary_count > 0 {
        stats.synthesized_documents += 1;
    }
    stats.synthesized_concerns += summary_count;

    Ok(())
}

fn synthesize_child_contexts(base: &Path, children: &[ContextRoot]) -> Result<usize> {
    let mut summaries = Vec::new();
    for child in children {
        summaries.push(build_child_summary(base, child)?);
    }
    summaries.sort_by(|a, b| a.relative_root.cmp(&b.relative_root));

    let target = synthesized_child_context_path(base);
    if summaries.is_empty() {
        if target.exists() {
            fs::remove_file(&target)
                .with_context(|| format!("failed to remove {}", target.display()))?;
        }
        return Ok(0);
    }

    let mut frontmatter = existing_synthesis_frontmatter(&target)?;
    frontmatter.status = Status::Current;
    frontmatter.concerns = summaries
        .iter()
        .map(|summary| summary.concern.clone())
        .collect();
    frontmatter.scope = Scope {
        paths: summaries
            .iter()
            .map(|summary| format!("{}/**", summary.relative_root))
            .collect(),
        components: Vec::new(),
    };
    frontmatter.superseded_by.clear();

    let body = summaries
        .iter()
        .map(render_child_summary)
        .collect::<Vec<_>>()
        .join("\n");
    let content = write_document(&frontmatter, &body)?;
    fs::write(&target, content).with_context(|| format!("failed to write {}", target.display()))?;

    Ok(summaries.len())
}

fn existing_synthesis_frontmatter(path: &Path) -> Result<Frontmatter> {
    if path.exists() {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let (frontmatter, _) = parse_document(&content)
            .with_context(|| format!("failed to parse frontmatter in {}", path.display()))?;
        return Ok(frontmatter);
    }

    let created = chrono::Utc::now();
    Ok(Frontmatter {
        id: generate_id(
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .as_ref(),
            &created,
        ),
        created,
        status: Status::Current,
        concerns: Vec::new(),
        scope: Scope::default(),
        superseded_by: Vec::new(),
    })
}

fn build_child_summary(parent: &Path, child: &ContextRoot) -> Result<ChildSummary> {
    let registry = load_or_sync_from(&child.base)?;
    let relative_root = child.relative.to_string_lossy().replace('\\', "/");
    let concern = format!("child-context:{relative_root}");

    let mut active_entries = registry
        .documents
        .values()
        .filter(|entry| entry.status != Status::Superseded)
        .cloned()
        .collect::<Vec<_>>();
    active_entries.sort_by(|a, b| a.file.cmp(&b.file));

    let mut active_concerns = Vec::new();
    let mut components = Vec::new();
    let mut scope_paths = Vec::new();
    let mut concern_summaries = Vec::new();

    for entry in &active_entries {
        active_concerns.extend(entry.active_concerns.iter().cloned());
        components.extend(entry.scope.components.iter().cloned());
        scope_paths.extend(
            entry
                .scope
                .paths
                .iter()
                .map(|scope_path| rebase_scope_path(&child.relative, scope_path)),
        );

        let content = fs::read_to_string(&entry.file)
            .with_context(|| format!("failed to read {}", entry.file))?;
        let (_, body) = parse_document(&content)
            .with_context(|| format!("failed to parse frontmatter in {}", entry.file))?;

        for concern_name in &entry.active_concerns {
            if concern_summaries
                .iter()
                .any(|summary: &ConcernSummary| summary.concern == *concern_name)
            {
                continue;
            }

            if let Some(section) = extract_concern_section(&body, concern_name) {
                if let Some(summary) = summarize_section(&section) {
                    concern_summaries.push(ConcernSummary {
                        concern: concern_name.clone(),
                        summary,
                    });
                }
            }
        }
    }

    active_concerns.sort();
    active_concerns.dedup();
    components.sort();
    components.dedup();
    scope_paths.sort();
    scope_paths.dedup();
    concern_summaries.sort_by(|a, b| a.concern.cmp(&b.concern));

    let _ = parent;
    Ok(ChildSummary {
        concern,
        relative_root,
        active_document_count: active_entries.len(),
        active_concerns,
        components,
        scope_paths,
        concern_summaries,
    })
}

fn render_child_summary(summary: &ChildSummary) -> String {
    let mut lines = vec![
        format!("- Source: {}/.context/", summary.relative_root),
        format!("- Active documents: {}", summary.active_document_count),
    ];

    if summary.active_concerns.is_empty() {
        lines.push("- Active concerns: none".to_string());
    } else {
        lines.push(format!(
            "- Active concerns: {}",
            summary.active_concerns.join(", ")
        ));
    }

    if !summary.components.is_empty() {
        lines.push(format!("- Components: {}", summary.components.join(", ")));
    }

    if !summary.scope_paths.is_empty() {
        lines.push(format!(
            "- Scoped paths: {}",
            summary.scope_paths.join(", ")
        ));
    }

    if summary.concern_summaries.is_empty() {
        lines.push("- Summary: no ranked concern sections found".to_string());
    } else {
        lines.push("- Summary:".to_string());
        for concern in &summary.concern_summaries {
            lines.push(format!("  - {}: {}", concern.concern, concern.summary));
        }
    }

    format_ranked_concern_block(&summary.concern, &lines.join("\n"), SYNTHESIS_RANK)
}

fn summarize_section(section: &str) -> Option<String> {
    let body = section
        .lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    if body.is_empty() {
        return None;
    }

    let paragraph = body
        .split("\n\n")
        .map(str::trim)
        .find(|value| !value.is_empty())?;
    let condensed = paragraph.split_whitespace().collect::<Vec<_>>().join(" ");
    Some(truncate(&condensed, 220))
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    for _ in 0..max_chars {
        if let Some(ch) = chars.next() {
            out.push(ch);
        } else {
            return out;
        }
    }

    if chars.next().is_some() {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::Utc;

    use super::{build_child_summary, cascade_sync_from, summarize_section};
    use crate::{
        document::{Frontmatter, Scope, Status, write_document},
        subtree::SYNTHESIZED_CHILD_CONTEXT_FILE,
    };

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("ctx-sync-{nanos}"))
    }

    fn write_doc(path: &Path, frontmatter: &Frontmatter, body: &str) {
        fs::write(path, write_document(frontmatter, body).unwrap()).unwrap();
    }

    #[test]
    fn summarizes_first_paragraph_of_concern_section() {
        let section = "### auth [r4]\n\nTokens expire after 15 minutes.\n\nMore detail.\n";
        assert_eq!(
            summarize_section(section),
            Some("Tokens expire after 15 minutes.".to_string())
        );
    }

    #[test]
    fn builds_child_summary_from_active_context() {
        let base = unique_temp_dir();
        let child = base.join("apps/api");
        fs::create_dir_all(child.join(".context")).unwrap();

        write_doc(
            &child.join(".context/notes.md"),
            &Frontmatter {
                id: "ctx-child".into(),
                created: Utc::now(),
                status: Status::Current,
                concerns: vec!["auth".into()],
                scope: Scope {
                    paths: vec!["src/**".into()],
                    components: vec!["api".into()],
                },
                superseded_by: vec![],
            },
            "### auth [r4]\n\nTokens expire after 15 minutes.\n",
        );

        let summary = build_child_summary(
            &base,
            &crate::subtree::ContextRoot {
                base: child.clone(),
                relative: PathBuf::from("apps/api"),
            },
        )
        .unwrap();
        assert_eq!(summary.concern, "child-context:apps/api");
        assert_eq!(summary.active_concerns, vec!["auth".to_string()]);
        assert_eq!(summary.scope_paths, vec!["apps/api/src/**".to_string()]);

        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn cascades_child_context_summaries_bottom_up() {
        let base = unique_temp_dir();
        fs::create_dir_all(base.join(".context")).unwrap();
        fs::create_dir_all(base.join("apps/api/.context")).unwrap();
        fs::create_dir_all(base.join("apps/api/services/auth/.context")).unwrap();

        write_doc(
            &base.join("apps/api/services/auth/.context/auth.md"),
            &Frontmatter {
                id: "ctx-auth".into(),
                created: Utc::now(),
                status: Status::Current,
                concerns: vec!["token-expiry".into()],
                scope: Scope {
                    paths: vec!["src/auth.rs".into()],
                    components: vec!["auth-service".into()],
                },
                superseded_by: vec![],
            },
            "### token-expiry [r4]\n\nTokens expire after 15 minutes.\n",
        );

        let stats = cascade_sync_from(&base).unwrap();
        assert_eq!(stats.contexts_synced, 3);
        assert_eq!(stats.synthesized_documents, 2);
        assert_eq!(stats.synthesized_concerns, 2);

        let child_summary = fs::read_to_string(base.join(format!(
            "apps/api/.context/{SYNTHESIZED_CHILD_CONTEXT_FILE}"
        )))
        .unwrap();
        assert!(child_summary.contains("child-context:services/auth"));
        assert!(child_summary.contains("services/auth/src/auth.rs"));

        let root_summary =
            fs::read_to_string(base.join(format!(".context/{SYNTHESIZED_CHILD_CONTEXT_FILE}")))
                .unwrap();
        assert!(root_summary.contains("child-context:apps/api"));
        assert!(root_summary.contains("token-expiry: Tokens expire after 15 minutes."));

        fs::remove_dir_all(base).unwrap();
    }
}
