use anyhow::Result;
use colored::Colorize;

use crate::config::{load_config_cwd, validate_config, validate_transitive_deps};
use crate::registry::PackageQuery;

pub async fn execute(verbose: bool) -> Result<()> {
    if verbose {
        println!("Validating spago.yaml configuration");
    }

    // Load config
    let config = load_config_cwd()?;

    if verbose {
        println!("Package: {}", config.package.name.0.bright_cyan());
        println!("Dependencies: {}", config.package.dependencies.len());

        if let Some(test) = &config.package.test {
            println!("Test dependencies: {}", test.dependencies.len());
        }
    }

    let package_set = config.package_set().await?;
    let query = PackageQuery::new(&package_set);

    if verbose {
        println!("Validating dependencies...");
    }

    let result = validate_config(&config, &query);

    // Always validate transitive dependencies
    let trans_result = validate_transitive_deps(&config, &query);

    // Check if we have any errors
    let has_errors = !result.errors.is_empty() || !trans_result.errors.is_empty();

    if !has_errors && result.warnings.is_empty() {
        // Success - show minimal success message
        println!("{} Configuration is valid", "✓".green().bold());

        if verbose {
            let all_deps = config.all_dependencies();
            println!("  Total unique dependencies: {}", all_deps.len());
            println!();
        }
        Ok(())
    } else {
        // Errors or warnings - always show
        if !result.errors.is_empty() || !trans_result.errors.is_empty() {
            println!("{} Validation errors:", "✗".red().bold());
            for error in &result.errors {
                println!("  {} {}", "✗".red(), error);
            }
            for error in &trans_result.errors {
                println!("  {} {}", "✗".red(), error);
            }
        }

        if !result.warnings.is_empty() {
            if has_errors {
                println!();
            }
            println!("{} Warnings:", "⚠".yellow().bold());
            for warning in &result.warnings {
                println!("  {} {}", "⚠".yellow(), warning);
            }
        }

        if has_errors {
            std::process::exit(1);
        }

        Ok(())
    }
}
