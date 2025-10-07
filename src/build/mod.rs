mod compiler;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::install::install_all_dependencies;

/// Execute the build command
pub async fn execute(watch: bool, clear: bool, verbose: bool) -> Result<()> {
    if verbose {
        println!("{} Build command executing", "→".cyan());
        println!("  Watch: {}", watch);
        println!("  Clear: {}", clear);
    }

    // Load spago.yaml configuration
    let config =
        crate::config::load_config_cwd().context("Failed to load spago.yaml configuration")?;

    let package_set = config.package_set()?;

    install_all_dependencies(&config, &package_set).await?;

    // Generate source globs for dependencies
    let sources = crate::sources::generate_sources(&config, Some(package_set), verbose)?;

    if verbose {
        println!(
            "{} Generated {} dependency globs",
            "→".cyan(),
            sources.dependency_globs.len()
        );
        for glob in &sources.dependency_globs {
            println!("  {}: {}", glob.package_name.blue(), glob.glob_pattern);
        }
    }

    // Collect all source globs into a Vec
    let mut all_sources = sources
        .dependency_globs
        .iter()
        .map(|g| g.glob_pattern.clone())
        .collect::<Vec<String>>();

    all_sources.push(sources.main_sources.clone());

    // Execute the purs compiler
    compiler::execute_compiler(&all_sources, &config.output_dir(), verbose)?;

    Ok(())
}
