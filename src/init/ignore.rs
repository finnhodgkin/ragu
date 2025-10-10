use std::fs;

use anyhow::{Context, Result};

const TEMPLATE: &str = r#"bower_components/
node_modules/
.pulp-cache/
output/
output-es/
generated-docs/
.psc-package/
.psc*
.purs*
.psa*
.spago
"#;

pub fn write() -> Result<()> {
    fs::write(".gitignore", TEMPLATE).context("Failed to write .gitignore")?;
    Ok(())
}
