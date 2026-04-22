use anyhow::{anyhow, Result};
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
    Ok(format!("---\n{}---\n{}", yaml, body))
}

pub fn active_concerns(frontmatter: &Frontmatter) -> Vec<String> {
    let mut active = frontmatter.concerns.clone();

    for replacement in &frontmatter.superseded_by {
        active.retain(|concern| !replacement.concerns.contains(concern));
    }

    active
}

#[cfg(test)]
mod tests {
    use super::{active_concerns, parse_document, write_document, Frontmatter, Scope, Status, SupersededBy};
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
            vec!["session-management".to_string(), "refresh-tokens".to_string()]
        );
    }
}
