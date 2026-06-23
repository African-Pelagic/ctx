use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Current,
    PartiallySuperseded,
    Superseded,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Scope {
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub components: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SupersededBy {
    pub id: String,
    #[serde(default)]
    pub concerns: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Frontmatter {
    pub id: String,
    pub created: DateTime<Utc>,
    pub status: Status,
    #[serde(default)]
    pub concerns: Vec<String>,
    pub scope: Scope,
    #[serde(default)]
    pub superseded_by: Vec<SupersededBy>,
}

pub fn parse_document(content: &str) -> Result<(Frontmatter, String)> {
    let rest = content
        .strip_prefix("---\n")
        .ok_or_else(|| anyhow!("document must start with frontmatter delimiter"))?;
    let (yaml, body) = rest
        .split_once("\n---\n")
        .ok_or_else(|| anyhow!("document must contain a closing frontmatter delimiter"))?;

    let frontmatter = serde_yaml::from_str::<Frontmatter>(yaml)?;
    Ok((frontmatter, body.to_string()))
}

pub fn write_document(frontmatter: &Frontmatter, body: &str) -> Result<String> {
    let yaml = serde_yaml::to_string(frontmatter)?;
    Ok(format!("---\n{yaml}---\n{body}"))
}

pub fn active_concerns(frontmatter: &Frontmatter) -> Vec<String> {
    let mut active = frontmatter.concerns.clone();

    for replacement in &frontmatter.superseded_by {
        active.retain(|concern| !replacement.concerns.contains(concern));
    }

    active
}

pub fn recompute_status(frontmatter: &mut Frontmatter) {
    if active_concerns(frontmatter).is_empty() {
        frontmatter.status = Status::Superseded;
    } else if frontmatter.superseded_by.is_empty() {
        frontmatter.status = Status::Current;
    } else {
        frontmatter.status = Status::PartiallySuperseded;
    }
}

pub fn concern_headings(body: &str) -> Vec<String> {
    body.lines().filter_map(parse_concern_heading).collect()
}

pub fn validate_rank(rank: u8) -> Result<()> {
    if (1..=5).contains(&rank) {
        Ok(())
    } else {
        bail!("rank must be between 1 and 5 inclusive");
    }
}

pub fn format_ranked_concern_block(concern: &str, text: &str, rank: u8) -> String {
    let mut out = String::new();
    out.push_str("### ");
    out.push_str(concern);
    out.push_str(" [r");
    out.push_str(&rank.to_string());
    out.push(']');
    out.push_str("\n\n");
    out.push_str(text.trim());
    out.push('\n');
    out
}

pub fn parse_concern_heading(line: &str) -> Option<String> {
    let heading = line.strip_prefix("### ")?;
    let heading = heading.trim();
    if heading.is_empty() {
        return None;
    }

    let mut concern = heading;
    while let Some((prefix, suffix)) = concern.rsplit_once(" [") {
        if suffix.ends_with(']') {
            concern = prefix.trim_end();
        } else {
            break;
        }
    }

    let concern = concern.trim();
    (!concern.is_empty()).then(|| concern.to_string())
}

pub fn extract_concern_section(body: &str, concern: &str) -> Option<String> {
    let mut section_lines = Vec::new();
    let mut in_section = false;

    for line in body.lines() {
        if parse_concern_heading(line).as_deref() == Some(concern) {
            in_section = true;
            section_lines.push(line);
            continue;
        }

        if in_section && line.starts_with("### ") {
            break;
        }

        if in_section {
            section_lines.push(line);
        }
    }

    if !in_section {
        return None;
    }

    Some(section_lines.join("\n").trim().to_string() + "\n")
}

pub fn sort_concern_sections_by_rank(body: &str) -> String {
    #[derive(Clone)]
    struct Section {
        rank: Option<u8>,
        start: usize,
        text: String,
    }

    let lines = body.lines().collect::<Vec<_>>();
    let mut preamble = Vec::new();
    let mut sections = Vec::new();
    let mut current_start = None;

    for (index, line) in lines.iter().enumerate() {
        if line.starts_with("### ") {
            if let Some(start) = current_start.replace(index) {
                sections.push(Section {
                    rank: parse_concern_rank(lines[start]),
                    start,
                    text: lines[start..index].join("\n"),
                });
            } else {
                preamble = lines[..index].to_vec();
            }
        }
    }

    if let Some(start) = current_start {
        sections.push(Section {
            rank: parse_concern_rank(lines[start]),
            start,
            text: lines[start..].join("\n"),
        });
    } else {
        return body.to_string();
    }

    sections.sort_by(|a, b| {
        b.rank
            .unwrap_or(0)
            .cmp(&a.rank.unwrap_or(0))
            .then(a.start.cmp(&b.start))
    });

    let mut parts = Vec::new();
    if !preamble.is_empty() {
        parts.push(preamble.join("\n").trim_end_matches('\n').to_string());
    }
    parts.extend(
        sections
            .iter()
            .map(|section| section.text.trim_end_matches('\n').to_string()),
    );

    let mut out = parts.join("\n\n");
    out.push('\n');
    out
}

fn parse_concern_rank(line: &str) -> Option<u8> {
    let heading = line.strip_prefix("### ")?.trim();
    for suffix in heading.split(" [").skip(1) {
        if let Some(metadata) = suffix.strip_suffix(']') {
            if let Some(value) = metadata.strip_prefix('r') {
                let rank = value.parse::<u8>().ok()?;
                if (1..=5).contains(&rank) {
                    return Some(rank);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        Frontmatter, Scope, Status, SupersededBy, active_concerns, concern_headings,
        extract_concern_section, format_ranked_concern_block, parse_concern_heading,
        parse_document, recompute_status, sort_concern_sections_by_rank, validate_rank,
        write_document,
    };
    use chrono::{TimeZone, Utc};

    fn sample_frontmatter() -> Frontmatter {
        Frontmatter {
            id: "ctx-7f3a9b".to_string(),
            created: Utc.with_ymd_and_hms(2025, 10, 15, 14, 23, 0).unwrap(),
            status: Status::Current,
            concerns: vec![
                "session-management".to_string(),
                "token-expiry".to_string(),
                "refresh-tokens".to_string(),
            ],
            scope: Scope {
                paths: vec!["src/sessions/**".to_string()],
                components: vec!["session-service".to_string()],
            },
            superseded_by: vec![SupersededBy {
                id: "ctx-2a81fc".to_string(),
                concerns: vec!["token-expiry".to_string()],
            }],
        }
    }

    #[test]
    fn round_trips_frontmatter_and_body() {
        let expected = sample_frontmatter();
        let document = write_document(&expected, "body text\n").unwrap();
        let (actual, body) = parse_document(&document).unwrap();

        assert_eq!(actual, expected);
        assert_eq!(body, "body text\n");
    }

    #[test]
    fn filters_superseded_concerns() {
        let frontmatter = sample_frontmatter();

        assert_eq!(
            active_concerns(&frontmatter),
            vec![
                "session-management".to_string(),
                "refresh-tokens".to_string()
            ]
        );
    }

    #[test]
    fn recomputes_partially_superseded_status() {
        let mut frontmatter = sample_frontmatter();
        recompute_status(&mut frontmatter);
        assert_eq!(frontmatter.status, Status::PartiallySuperseded);
    }

    #[test]
    fn recomputes_fully_superseded_status() {
        let mut frontmatter = sample_frontmatter();
        frontmatter.superseded_by.push(SupersededBy {
            id: "ctx-3".to_string(),
            concerns: vec![
                "session-management".to_string(),
                "refresh-tokens".to_string(),
            ],
        });
        recompute_status(&mut frontmatter);
        assert_eq!(frontmatter.status, Status::Superseded);
    }

    #[test]
    fn extracts_concern_headings() {
        let body = "### billing [r2]\n\nnote\n\n### auth [n7] [r4]\n\nother\n";
        assert_eq!(
            concern_headings(body),
            vec!["billing".to_string(), "auth".to_string()]
        );
    }

    #[test]
    fn extracts_specific_concern_section() {
        let body = "### billing [r3]\n\nnote\n\n### auth [r2]\n\nother\n";
        assert_eq!(
            extract_concern_section(body, "billing"),
            Some("### billing [r3]\n\nnote\n".to_string())
        );
    }

    #[test]
    fn validates_rank_range() {
        assert!(validate_rank(1).is_ok());
        assert!(validate_rank(5).is_ok());
        assert!(validate_rank(0).is_err());
        assert!(validate_rank(6).is_err());
    }

    #[test]
    fn formats_ranked_concern_block() {
        assert_eq!(
            format_ranked_concern_block("billing", "New note", 4),
            "### billing [r4]\n\nNew note\n"
        );
    }

    #[test]
    fn parses_heading_metadata_suffixes() {
        assert_eq!(
            parse_concern_heading("### billing [r4]"),
            Some("billing".to_string())
        );
        assert_eq!(
            parse_concern_heading("### billing rules [n7] [r4]"),
            Some("billing rules".to_string())
        );
    }

    #[test]
    fn sorts_concern_sections_by_descending_rank() {
        let body = "Intro\n\n### billing [r2]\n\nLower\n\n### auth [r5]\n\nHigher\n\n### cache [r3]\n\nMiddle\n";
        assert_eq!(
            sort_concern_sections_by_rank(body),
            "Intro\n\n### auth [r5]\n\nHigher\n\n### cache [r3]\n\nMiddle\n\n### billing [r2]\n\nLower\n"
        );
    }

    #[test]
    fn leaves_unranked_sections_after_ranked_ones() {
        let body = "### legacy\n\nOld\n\n### auth [r4]\n\nCurrent\n";
        assert_eq!(
            sort_concern_sections_by_rank(body),
            "### auth [r4]\n\nCurrent\n\n### legacy\n\nOld\n"
        );
    }

    #[test]
    fn parses_rank_with_other_heading_metadata_present() {
        let body = "### billing [n7] [r4]\n\nBody\n### auth [r2]\n\nOther\n";
        assert_eq!(
            sort_concern_sections_by_rank(body),
            "### billing [n7] [r4]\n\nBody\n\n### auth [r2]\n\nOther\n"
        );
    }
}
