use std::fs;

use anyhow::{Context, Result};

const TEMPLATE: &str = r#"module Main where

import Prelude

import Effect (Effect)
import Effect.Console (log)

main :: Effect Unit
main = do
  log "🍝"
"#;

pub fn write() -> Result<()> {
    fs::create_dir("src").context("Failed to create src directory")?;
    fs::write("src/Main.purs", TEMPLATE).context("Failed to write src/Main.purs")?;
    Ok(())
}
