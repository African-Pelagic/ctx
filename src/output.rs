use std::process;

use anyhow::{Result, bail};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputMode {
    Human,
    Json,
    Porcelain,
}

impl OutputMode {
    pub fn from_flags(json: bool, porcelain: bool) -> Result<Self> {
        if json && porcelain {
            bail!("--json and --porcelain cannot be used together");
        }

        if json {
            Ok(Self::Json)
        } else if porcelain {
            Ok(Self::Porcelain)
        } else {
            Ok(Self::Human)
        }
    }
}

#[derive(Serialize)]
struct ErrorPayload<'a> {
    error: &'a str,
}

pub fn print_error_and_exit(output_mode: OutputMode, error: &anyhow::Error) -> ! {
    match output_mode {
        OutputMode::Human => {
            eprintln!("Error: {error}");
        }
        OutputMode::Json => {
            let payload = serde_json::to_string_pretty(&ErrorPayload {
                error: &error.to_string(),
            })
            .unwrap_or_else(|_| format!("{{\"error\":\"{}\"}}", error));
            eprintln!("{payload}");
        }
        OutputMode::Porcelain => {
            eprintln!("error {}", error);
        }
    }

    process::exit(1);
}
