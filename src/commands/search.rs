use std::fs;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::{cli::SearchArgs, document::Status, output::OutputMode, registry::load_or_sync};

#[derive(Debug, Serialize)]
pub(crate) struct SearchMatch {
    pub line_number: usize,
    pub line: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct SearchResult {
    pub id: String,
    pub file: String,
    pub active_concerns: Vec<String>,
    pub matches: Vec<SearchMatch>,
}

pub(crate) fn collect(args: &SearchArgs) -> Result<Vec<SearchResult>> {
    search_documents(args)
}

pub fn run(args: SearchArgs, output_mode: OutputMode) -> Result<()> {
    let results = collect(&args)?;

    match output_mode {
        OutputMode::Human => {
            if results.is_empty() {
                println!("No matching context documents.");
            } else {
                for (index, result) in results.iter().enumerate() {
                    if index > 0 {
                        println!();
                    }
                    println!("# {} - {}", result.id, result.file);
                    println!("Active concerns: {}", result.active_concerns.join(", "));
                    for hit in &result.matches {
                        println!("{}: {}", hit.line_number, hit.line);
                    }
                }
            }
        }
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        OutputMode::Porcelain => {
            for result in &results {
                for hit in &result.matches {
                    println!(
                        "{}\t{}\t{}\t{}\t{}",
                        result.id,
                        result.file,
                        result.active_concerns.join(","),
                        hit.line_number,
                        hit.line
                    );
                }
            }
        }
    }

    Ok(())
}

fn search_documents(args: &SearchArgs) -> Result<Vec<SearchResult>> {
    if args.query.trim().is_empty() {
        bail!("--query must not be empty");
    }

    let registry = load_or_sync()?;
    let mut results = Vec::new();

    for (id, entry) in &registry.documents {
        if !args.include_superseded && entry.status == Status::Superseded {
            continue;
        }

        let content = fs::read_to_string(&entry.file)
            .with_context(|| format!("failed to read {}", entry.file))?;
        let body = strip_frontmatter(&content);
        let matches = find_matches(&body, &args.query);

        if matches.is_empty() {
            continue;
        }

        results.push(SearchResult {
            id: id.clone(),
            file: entry.file.clone(),
            active_concerns: entry.active_concerns.clone(),
            matches,
        });
    }

    results.sort_by(|a, b| a.file.cmp(&b.file).then(a.id.cmp(&b.id)));
    Ok(results)
}

fn find_matches(body: &str, query: &str) -> Vec<SearchMatch> {
    body.lines()
        .enumerate()
        .filter(|(_, line)| line.contains(query))
        .map(|(index, line)| SearchMatch {
            line_number: index + 1,
            line: line.to_string(),
        })
        .collect()
}

fn strip_frontmatter(content: &str) -> String {
    if let Some(rest) = content.strip_prefix("---\n") {
        if let Some((_, body)) = rest.split_once("\n---\n") {
            return body.to_string();
        }
    }
    content.to_string()
}

#[cfg(test)]
mod tests {
    use super::{find_matches, search_documents};
    use crate::cli::SearchArgs;

    #[test]
    fn rejects_empty_query() {
        let args = SearchArgs {
            query: " ".into(),
            include_superseded: false,
        };
        assert!(search_documents(&args).is_err());
    }

    #[test]
    fn finds_matching_lines() {
        let matches = find_matches("alpha\nbeta alpha\n", "alpha");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line_number, 1);
        assert_eq!(matches[1].line_number, 2);
    }
}
