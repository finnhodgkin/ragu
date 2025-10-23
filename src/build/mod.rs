pub mod compiler;

use anyhow::{Context, Result};
use colored::Colorize;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{install::install_all_dependencies, test::TEST_SOURCES};

/// Execute the build command
pub async fn execute(
    watch: bool,
    clear: bool,
    test: bool,
    compiler_args: Vec<String>,
    include_rts_stats: bool,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("{} Build command executing", "→".cyan());
        println!("  Watch: {}", watch);
        println!("  Clear: {}", clear);
    }

    // Load spago.yaml configuration
    let config =
        crate::config::load_config_cwd().context("Failed to load spago.yaml configuration")?;

    let package_set = config.package_set()?;

    install_all_dependencies(&config, &package_set, test).await?;

    // Generate source globs for dependencies
    let sources =
        crate::sources::generate_sources(&config, Some(package_set), false, test, verbose)?;

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

    if test {
        all_sources.push(TEST_SOURCES.to_string());
    }

    // Remove any sources that don't contain any .purs files
    all_sources = all_sources
        .into_par_iter()
        .filter(|source| {
            let files = glob::glob(source);
            if let Ok(files) = files {
                files.peekable().peek().is_some()
            } else {
                false
            }
        })
        .collect();

    // Execute the purs compiler
    compiler::execute_compiler(
        &all_sources,
        &config.output_dir(),
        compiler_args,
        include_rts_stats,
        verbose,
    )?;

    println!("{} Build successful", "✓".green());

    Ok(())
}
