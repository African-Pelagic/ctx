use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

pub fn generate_id(filename: &str, created: &DateTime<Utc>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(filename.as_bytes());
    hasher.update(created.to_rfc3339().as_bytes());

    let digest = hasher.finalize();
    let short = hex::encode(&digest[..3]);

    format!("ctx-{short}")
}

#[cfg(test)]
mod tests {
    use super::generate_id;
    use chrono::{TimeZone, Utc};

    #[test]
    fn generates_stable_prefixed_ids() {
        let created = Utc.with_ymd_and_hms(2025, 10, 15, 14, 23, 0).unwrap();
        let id = generate_id("session-handling", &created);

        assert!(id.starts_with("ctx-"));
        assert_eq!(id.len(), 10);
        assert_eq!(id, generate_id("session-handling", &created));
    }
}
