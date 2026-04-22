use anyhow::{bail, Result};

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
