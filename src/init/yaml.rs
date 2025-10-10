use std::fs;

use anyhow::{Context, Result};
use heck::ToPascalCase;

pub fn template(name: &String, registry_version: &String) -> String {
    let name_pascal = name.to_pascal_case();
    format!(
        r#"package:
  name: {name}
  dependencies:
    - console
    - effect
    - prelude
  test:
    main: {name_pascal}.Test.Main
    dependencies: []
workspace:
  packageSet:
    registry: {registry_version}
  extraPackages: {{}}
"#
    )
}

pub fn write(name: &String, registry_version: &String) -> Result<()> {
    let template = template(name, registry_version);
    fs::write("spago.yaml", template).context("Failed to write spago.yaml")?;
    Ok(())
}
