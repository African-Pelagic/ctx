use std::path::Path;

use anyhow::Result;

use crate::{
    cli::{SourceArgs, SourceCommand, SourceQueryArgs},
    output::OutputMode,
    source::{self, Filter, Hit},
};

pub(crate) fn collect_query(base: &Path, args: &SourceQueryArgs) -> Result<Vec<Hit>> {
    let filter = Filter {
        source: args.source.clone(),
        domain: args.domain.clone(),
    };
    source::query(base, &args.query, &filter, args.limit)
}

pub fn run(args: SourceArgs, output_mode: OutputMode) -> Result<()> {
    let base = Path::new(".");
    match args.command {
        SourceCommand::Add(add) => {
            let meta = source::add(base, &add.file, add.name.as_deref(), add.domain.as_deref())?;
            match output_mode {
                OutputMode::Json => println!("{}", serde_json::to_string_pretty(&meta)?),
                OutputMode::Porcelain => println!("{}\t{}", meta.name, meta.sha256),
                OutputMode::Human => println!(
                    "Added source {} ({} bytes, sha256 {})",
                    meta.name,
                    meta.bytes,
                    &meta.sha256[..12]
                ),
            }
        }
        SourceCommand::List => {
            let sources = source::list(base)?;
            match output_mode {
                OutputMode::Json => println!("{}", serde_json::to_string_pretty(&sources)?),
                OutputMode::Porcelain => {
                    for s in &sources {
                        println!("{}\t{}\t{}", s.name, s.domain.as_deref().unwrap_or(""), s.sha256);
                    }
                }
                OutputMode::Human if sources.is_empty() => println!("No sources imported."),
                OutputMode::Human => {
                    for s in &sources {
                        println!(
                            "{}  domain={}  {} bytes  {}",
                            s.name,
                            s.domain.as_deref().unwrap_or("-"),
                            s.bytes,
                            s.origin
                        );
                    }
                }
            }
        }
        SourceCommand::Query(query) => {
            let hits = collect_query(base, &query)?;
            match output_mode {
                OutputMode::Json => println!("{}", serde_json::to_string_pretty(&hits)?),
                OutputMode::Porcelain => {
                    for h in &hits {
                        println!(
                            "{}\t{}\t{}\t{:.3}\t{}",
                            h.chunk.source, h.chunk.start_line, h.chunk.end_line, h.score, h.chunk.heading
                        );
                    }
                }
                OutputMode::Human if hits.is_empty() => println!("No matching source chunks."),
                OutputMode::Human => {
                    for (index, h) in hits.iter().enumerate() {
                        if index > 0 {
                            println!();
                        }
                        let heading = if h.chunk.heading.is_empty() {
                            String::new()
                        } else {
                            format!("#{}", h.chunk.heading)
                        };
                        println!(
                            "# {}{} (lines {}-{}, score {:.2})",
                            h.chunk.source, heading, h.chunk.start_line, h.chunk.end_line, h.score
                        );
                        println!("{}", h.chunk.text);
                    }
                }
            }
        }
    }
    Ok(())
}
