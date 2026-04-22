use anyhow::Result;
use serde::Serialize;

use crate::{
    git::{doc_path, is_stale},
    output::OutputMode,
    registry::load_or_sync,
};

#[derive(Debug, Serialize)]
struct ListRow {
    concern: String,
    owners: Vec<String>,
    files: Vec<String>,
    notes: Vec<String>,
}

pub fn run(output_mode: OutputMode) -> Result<()> {
    let registry = load_or_sync()?;
    let rows = build_rows(&registry);

    match output_mode {
        OutputMode::Human => print_human(&rows, &registry.orphaned_concerns),
        OutputMode::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "roster": rows,
                    "orphaned_concerns": registry.orphaned_concerns,
                    "multi_owned_concerns": registry.multi_owned_concerns,
                }))?
            );
        }
        OutputMode::Porcelain => {
            for row in &rows {
                println!(
                    "{}\t{}\t{}\t{}",
                    row.concern,
                    row.owners.join(","),
                    row.files.join(","),
                    row.notes.join(",")
                );
            }
            for concern in &registry.orphaned_concerns {
                println!("orphaned\t{concern}\t\t");
            }
        }
    }

    Ok(())
}

fn build_rows(registry: &crate::registry::Registry) -> Vec<ListRow> {
    let mut rows = Vec::new();

    for (concern, entry) in &registry.concern_roster {
        let mut notes = Vec::new();
        let mut files = Vec::new();
        if entry.owners.len() > 1 {
            notes.push("multi-owned".to_string());
        }

        for owner in &entry.owners {
            if let Some(doc) = registry.documents.get(owner) {
                files.push(doc.file.clone());
                if is_stale(&doc_path(&doc.file), &doc.scope.paths) {
                    notes.push(format!("stale:{owner}"));
                }
            }
        }

        files.sort();
        files.dedup();
        notes.sort();
        notes.dedup();
        rows.push(ListRow {
            concern: concern.clone(),
            owners: entry.owners.clone(),
            files,
            notes,
        });
    }

    rows
}

fn print_human(rows: &[ListRow], orphaned_concerns: &[String]) {
    if rows.is_empty() {
        println!("No active concerns.");
    } else {
        let headers = ["Concern", "Owners", "Files", "Notes"];
        let concern_width = rows
            .iter()
            .map(|row| row.concern.len())
            .max()
            .unwrap_or(0)
            .max(headers[0].len());
        let owners_width = rows
            .iter()
            .map(|row| row.owners.join(", ").len())
            .max()
            .unwrap_or(0)
            .max(headers[1].len());
        let files_width = rows
            .iter()
            .map(|row| row.files.join(", ").len())
            .max()
            .unwrap_or(0)
            .max(headers[2].len());
        let notes_width = rows
            .iter()
            .map(|row| display_or_dash(&row.notes).len())
            .max()
            .unwrap_or(0)
            .max(headers[3].len());

        println!(
            "{:<concern_width$}  {:<owners_width$}  {:<files_width$}  {:<notes_width$}",
            headers[0], headers[1], headers[2], headers[3],
        );
        println!(
            "{:-<concern_width$}  {:-<owners_width$}  {:-<files_width$}  {:-<notes_width$}",
            "", "", "", "",
        );
        for row in rows {
            let owners = row.owners.join(", ");
            let files = row.files.join(", ");
            let notes = display_or_dash(&row.notes);
            println!(
                "{:<concern_width$}  {:<owners_width$}  {:<files_width$}  {:<notes_width$}",
                row.concern, owners, files, notes
            );
        }
    }

    if !orphaned_concerns.is_empty() {
        println!();
        println!("Orphaned concerns:");
        for concern in orphaned_concerns {
            println!("{concern}");
        }
    }
}

fn display_or_dash(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::build_rows;
    use crate::{
        document::Status,
        registry::{ConcernRosterEntry, DocumentEntry, Registry},
    };

    #[test]
    fn builds_rows_from_registry_roster() {
        let registry = Registry {
            schema_version: 1,
            generated_at: Utc::now(),
            generated_from_commit: None,
            documents: [(
                "ctx-a".to_string(),
                DocumentEntry {
                    file: ".context/a.md".into(),
                    created: Utc::now(),
                    status: Status::Current,
                    concerns: vec!["billing".into()],
                    active_concerns: vec!["billing".into()],
                    scope: Default::default(),
                    superseded_by: vec![],
                },
            )]
            .into_iter()
            .collect(),
            concern_roster: [(
                "billing".to_string(),
                ConcernRosterEntry {
                    owners: vec!["ctx-a".into()],
                },
            )]
            .into_iter()
            .collect(),
            orphaned_concerns: vec![],
            multi_owned_concerns: vec![],
        };

        let rows = build_rows(&registry);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].concern, "billing");
        assert_eq!(rows[0].owners, vec!["ctx-a".to_string()]);
        assert_eq!(rows[0].files, vec![".context/a.md".to_string()]);
    }
}
