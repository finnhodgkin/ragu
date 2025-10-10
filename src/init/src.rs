use std::fs;

use anyhow::{Context, Result};
use heck::ToPascalCase;

fn template(name: &String, is_nested_package: bool) -> String {
    let module_name = if is_nested_package {
        name.to_pascal_case()
    } else {
        "Main".to_string()
    };
    format!(
        r#"module {module_name} where

import Prelude

import Effect (Effect)
import Effect.Console (log)

main :: Effect Unit
main = do
  log "🍝"
"#
    )
}

pub fn write(name: &String, is_nested_package: bool) -> Result<()> {
    fs::create_dir("src").context("Failed to create src directory")?;
    fs::write("src/Main.purs", template(name, is_nested_package))
        .context("Failed to write src/Main.purs")?;
    Ok(())
}
