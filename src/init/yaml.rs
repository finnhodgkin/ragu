use std::fs;

use anyhow::{Context, Result};
use heck::ToPascalCase;

pub fn template(name: &String, registry_version: &String, is_nested_package: bool) -> String {
    let name_pascal = name.to_pascal_case();

    let workspace = if is_nested_package {
        "".to_string()
    } else {
        format!(
            r#"workspace:
  packageSet:
    registry: {registry_version}
  extraPackages: {{}}
"#
        )
    };

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
{workspace}"#
    )
}

pub fn write(name: &String, registry_version: &String, is_nested_package: bool) -> Result<()> {
    let template = template(name, registry_version, is_nested_package);
    fs::write("spago.yaml", template).context("Failed to write spago.yaml")?;
    Ok(())
}
