use std::fs;

use anyhow::{Context, Result};
use heck::ToPascalCase;

fn template(name: &String) -> String {
    let pascal_name = name.to_pascal_case();
    format!(
        r#"module {pascal_name}.Test.Main where

import Prelude

import Effect (Effect)
import Effect.Class.Console (log)

main :: Effect Unit
main = do
  log "🍕"
  log "You should add some tests."
"#
    )
}

pub fn write(name: &String) -> Result<()> {
    fs::create_dir("test").context("Failed to create test directory")?;
    let template = template(name);
    fs::write("test/Main.purs", template).context("Failed to write test/Main.purs")?;
    Ok(())
}
