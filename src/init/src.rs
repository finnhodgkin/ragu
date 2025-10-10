use std::fs;

use anyhow::{Context, Result};
use heck::ToPascalCase;

fn template(name: &String) -> String {
    let pascal_name = name.to_pascal_case();
    format!(
        r#"module {pascal_name}.Main where

import Prelude

import Effect (Effect)
import Effect.Console (log)

main :: Effect Unit
main = do
  log "🍝"
"#
    )
}

pub fn write(name: &String) -> Result<()> {
    fs::create_dir("src").context("Failed to create src directory")?;
    let template = template(name);
    fs::write("src/Main.purs", template).context("Failed to write src/Main.purs")?;
    Ok(())
}
