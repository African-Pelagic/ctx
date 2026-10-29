//! Reference sources: large authoritative texts stored under
//! `.context/sources/<name>/` and retrieved by chunk.
//!
//! A source is derived retrieval data, not a claim: it never takes part in
//! concern ownership or supersession. Chunks and rankings are computed from
//! the stored text on every query, so there is no index to go stale. Anything
//! richer (extracted terms, aliases) should be added as extra files beside
//! `source.txt` and `meta.json`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{bm25, registry::context_dir_from};

const TEXT_FILE: &str = "source.txt";
const META_FILE: &str = "meta.json";
const MAX_CHUNK_WORDS: usize = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMeta {
    pub name: String,
    pub domain: Option<String>,
    pub origin: String,
    pub sha256: String,
    pub bytes: usize,
    pub added: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Chunk {
    pub source: String,
    pub domain: Option<String>,
    pub heading: String,
    pub start_line: usize,
    pub end_line: usize,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct Hit {
    pub score: f64,
    #[serde(flatten)]
    pub chunk: Chunk,
}

#[derive(Debug, Default)]
pub struct Filter {
    pub source: Option<String>,
    pub domain: Option<String>,
}

fn sources_dir_from(base: &Path) -> PathBuf {
    context_dir_from(base).join("sources")
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

pub fn slugify(raw: &str) -> String {
    let slug: String = raw
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    slug.split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn add(base: &Path, file: &Path, name: Option<&str>, domain: Option<&str>) -> Result<SourceMeta> {
    let text = fs::read_to_string(file)
        .with_context(|| format!("failed to read {} as UTF-8 text", file.display()))?;
    if text.trim().is_empty() {
        bail!("{} is empty", file.display());
    }

    let name = match name {
        Some(name) => name.to_string(),
        None => slugify(
            &file
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
        ),
    };
    if !valid_name(&name) {
        bail!("source name {name:?} must be lowercase letters, digits and hyphens; pass --name");
    }
    let domain = match domain {
        Some(domain) if !valid_name(domain) => {
            bail!("domain {domain:?} must be lowercase letters, digits and hyphens")
        }
        other => other.map(str::to_string),
    };

    let dir = sources_dir_from(base).join(&name);
    if dir.exists() {
        bail!(
            "source {name:?} already exists at {}; choose another --name",
            dir.display()
        );
    }

    let meta = SourceMeta {
        name,
        domain,
        origin: file.display().to_string(),
        sha256: hex::encode(Sha256::digest(text.as_bytes())),
        bytes: text.len(),
        added: Utc::now().to_rfc3339(),
    };

    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    fs::write(dir.join(TEXT_FILE), &text)?;
    fs::write(dir.join(META_FILE), serde_json::to_string_pretty(&meta)? + "\n")?;
    Ok(meta)
}

pub fn list(base: &Path) -> Result<Vec<SourceMeta>> {
    let root = sources_dir_from(base);
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut sources = Vec::new();
    for entry in fs::read_dir(&root).with_context(|| format!("failed to read {}", root.display()))? {
        let meta_path = entry?.path().join(META_FILE);
        if !meta_path.is_file() {
            continue;
        }
        let raw = fs::read_to_string(&meta_path)?;
        sources.push(
            serde_json::from_str(&raw)
                .with_context(|| format!("failed to parse {}", meta_path.display()))?,
        );
    }
    sources.sort_by(|a: &SourceMeta, b| a.name.cmp(&b.name));
    Ok(sources)
}

/// BM25-ranked chunks across the selected sources, best first.
pub fn query(base: &Path, text: &str, filter: &Filter, limit: usize) -> Result<Vec<Hit>> {
    let query_tokens = bm25::tokenize(text);
    if query_tokens.is_empty() {
        bail!("query has no searchable terms");
    }

    let mut chunks = Vec::new();
    for meta in list(base)? {
        if filter.source.as_ref().is_some_and(|s| *s != meta.name)
            || filter.domain.is_some() && filter.domain != meta.domain
        {
            continue;
        }
        let path = sources_dir_from(base).join(&meta.name).join(TEXT_FILE);
        let body = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        chunks.extend(chunk_text(&meta.name, meta.domain.as_deref(), &body));
    }

    // The heading path is part of the searchable text so a query for a section
    // title finds its content.
    let corpus: Vec<Vec<String>> = chunks
        .iter()
        .map(|c| bm25::tokenize(&format!("{} {}", c.heading, c.text)))
        .collect();

    Ok(bm25::rank(&corpus, &query_tokens)
        .into_iter()
        .take(limit)
        .map(|(index, score)| Hit {
            score,
            chunk: chunks[index].clone(),
        })
        .collect())
}

/// Splits text on markdown (`#`) and org (`*`) headings, then packs each
/// section's paragraphs into chunks of at most roughly `MAX_CHUNK_WORDS`.
/// Lines are 1-based so a hit can be opened at `source.txt:<line>`.
pub fn chunk_text(source: &str, domain: Option<&str>, text: &str) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut headings: Vec<(usize, String)> = Vec::new();
    let mut section: Vec<(usize, &str)> = Vec::new();
    let mut in_fence = false;

    let mut flush = |headings: &[(usize, String)], section: &mut Vec<(usize, &str)>| {
        let path = headings
            .iter()
            .map(|(_, h)| h.as_str())
            .collect::<Vec<_>>()
            .join(" > ");
        pack_section(source, domain, &path, section, &mut chunks);
        section.clear();
    };

    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        }
        match (in_fence, heading_of(line)) {
            (false, Some((level, title))) => {
                flush(&headings, &mut section);
                headings.retain(|(l, _)| *l < level);
                headings.push((level, title));
            }
            _ => section.push((line_number, line)),
        }
    }
    flush(&headings, &mut section);
    chunks
}

fn heading_of(line: &str) -> Option<(usize, String)> {
    let marker = line.chars().next().filter(|c| *c == '#' || *c == '*')?;
    let level = line.chars().take_while(|c| *c == marker).count();
    let title = line[level..].strip_prefix(' ')?.trim();
    (level <= 6 && !title.is_empty()).then(|| (level, title.to_string()))
}

fn pack_section(
    source: &str,
    domain: Option<&str>,
    heading: &str,
    lines: &[(usize, &str)],
    out: &mut Vec<Chunk>,
) {
    // Paragraph = run of non-blank lines, remembering its line span.
    let mut paragraphs: Vec<(usize, usize, String)> = Vec::new();
    for &(number, line) in lines {
        if line.trim().is_empty() {
            continue;
        }
        match paragraphs.last_mut() {
            Some((_, end, text)) if *end + 1 == number => {
                *end = number;
                text.push('\n');
                text.push_str(line);
            }
            _ => paragraphs.push((number, number, line.to_string())),
        }
    }

    let mut current: Option<(usize, usize, String, usize)> = None;
    let mut emit = |current: &mut Option<(usize, usize, String, usize)>| {
        if let Some((start, end, text, _)) = current.take() {
            out.push(Chunk {
                source: source.to_string(),
                domain: domain.map(str::to_string),
                heading: heading.to_string(),
                start_line: start,
                end_line: end,
                text,
            });
        }
    };

    for (start, end, text) in paragraphs {
        // A paragraph longer than the limit is cut into word windows that
        // share its line span (reflowed text may have no internal newlines).
        let word_count = text.split_whitespace().count();
        let pieces: Vec<(String, usize)> = if word_count > MAX_CHUNK_WORDS {
            text.split_whitespace()
                .collect::<Vec<_>>()
                .chunks(MAX_CHUNK_WORDS)
                .map(|w| (w.join(" "), w.len()))
                .collect()
        } else {
            vec![(text, word_count)]
        };

        for (piece, count) in pieces {
            match current.as_mut() {
                Some((_, cur_end, cur_text, cur_words))
                    if *cur_words + count <= MAX_CHUNK_WORDS =>
                {
                    *cur_end = end;
                    cur_text.push_str("\n\n");
                    cur_text.push_str(&piece);
                    *cur_words += count;
                }
                _ => {
                    emit(&mut current);
                    current = Some((start, end, piece, count));
                }
            }
        }
    }
    emit(&mut current);
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf};

    use super::{Filter, add, chunk_text, list, query, slugify};

    fn temp_base(label: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("ctx-source-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn slugifies_file_stems() {
        assert_eq!(slugify("Private Equity: 2nd Ed."), "private-equity-2nd-ed");
    }

    #[test]
    fn chunks_by_heading_with_path_and_line_numbers() {
        let text = "intro line\n\n# Fees\n\nmanagement fee text\n\n## Carry\n\ncarry text\n";
        let chunks = chunk_text("pe", None, text);
        let shape: Vec<_> = chunks
            .iter()
            .map(|c| (c.heading.as_str(), c.start_line, c.end_line))
            .collect();
        assert_eq!(
            shape,
            vec![("", 1, 1), ("Fees", 5, 5), ("Fees > Carry", 9, 9)]
        );
    }

    #[test]
    fn org_headings_split_and_fenced_hashes_do_not() {
        let text = "* Top\nbody\n```\n# not a heading\n```\n";
        let chunks = chunk_text("s", None, text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].heading, "Top");
        assert!(chunks[0].text.contains("# not a heading"));
    }

    #[test]
    fn oversized_paragraphs_are_windowed() {
        let text = vec!["word"; 700].join(" ");
        let chunks = chunk_text("s", None, &text);
        assert_eq!(chunks.len(), 3);
        assert!(chunks.iter().all(|c| c.start_line == 1 && c.end_line == 1));
    }

    #[test]
    fn add_list_and_query_round_trip() {
        let base = temp_base("roundtrip");
        let file = base.join("pe.md");
        fs::write(
            &file,
            "# Fees\n\nThe hurdle rate gates carried interest.\n\n# Exits\n\nSecondary sales return capital.\n",
        )
        .unwrap();

        let meta = add(&base, &file, Some("pe-primer"), Some("pe")).unwrap();
        assert_eq!(meta.bytes, fs::read_to_string(&file).unwrap().len());
        assert!(add(&base, &file, Some("pe-primer"), None).is_err());
        assert_eq!(list(&base).unwrap().len(), 1);

        let hits = query(&base, "hurdle rate", &Filter::default(), 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].chunk.heading, "Fees");

        let other = Filter {
            domain: Some("se".into()),
            ..Filter::default()
        };
        assert!(query(&base, "hurdle", &other, 5).unwrap().is_empty());
        let _ = fs::remove_dir_all(&base);
    }
}
