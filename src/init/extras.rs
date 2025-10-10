use std::fs;

use anyhow::{Context, Result};

pub fn write() -> Result<()> {
    fs::write(".psc-ide-port", "15566").context("Failed to write .psc-ide-port")?;
    fs::write(".purs-repl", "import Prelude").context("Failed to write .purs-repl")?;
    Ok(())
}
