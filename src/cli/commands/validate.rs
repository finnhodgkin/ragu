use anyhow::{Context, Result};
use colored::Colorize;

use crate::config::{extract_tag_from_url, load_config, validate_config, validate_transitive_deps};
use crate::registry::{get_package_set, PackageQuery};

pub fn execute(path: Option<String>, force_refresh: bool, verbose: bool) -> Result<()> {
    let config_path = path.as_deref().unwrap_or("spago.yaml");

    if verbose {
        println!("\nValidating configuration: {}\n", config_path);
    }

    // Load config
    let config = load_config(config_path)?;

    if verbose {
        println!("Package: {}", config.package.name.bright_cyan());
        println!("Dependencies: {}", config.package.dependencies.len());

        if let Some(test) = &config.package.test {
            println!("Test dependencies: {}", test.dependencies.len());
        }
    }

    // Determine package set tag
    let tag = if let Some(url) = config.package_set_url() {
        if verbose {
            println!("\nPackage set URL: {}", url.dimmed());
        }
        extract_tag_from_url(url).context("Could not extract tag from package set URL")?
    } else {
        if verbose {
            println!("\nNo package set URL specified, using latest");
        }
        use crate::registry::list_available_tags;
        let tags = list_available_tags()?;
        tags.first().cloned().context("No tags available")?
    };

    if verbose {
        println!("Package set tag: {}", tag.cyan());
        println!("\nLoading package set...");
    }

    // Load package set (uses cache - blazingly fast!)
    let package_set = get_package_set(&tag, force_refresh)?;
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
        println!("{} Configuration is valid!", "✓".green().bold());

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
