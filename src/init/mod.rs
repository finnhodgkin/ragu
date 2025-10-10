use anyhow::{Context, Result};
use colored::Colorize;
use heck::ToPascalCase;

use crate::registry::list_available_registry_versions_with_options;

mod extras;
mod ignore;
mod src;
mod test;
mod yaml;

pub fn execute(name: String) -> Result<()> {
    let registry_version = list_available_registry_versions_with_options(false, None)?
        .first()
        .context("Failed to get registry version")?
        .clone();

    yaml::write(&name, &registry_version)?;
    src::write()?;
    test::write(&name)?;
    ignore::write()?;
    extras::write()?;

    println!(
        "{} successfully initialised {}, run 'build' to get started",
        "✓".green().bold(),
        name.to_pascal_case()
    );
    Ok(())
}
