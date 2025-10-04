use anyhow::{Context, Result};
use colored::Colorize;
use std::path::PathBuf;

use crate::config::{load_config, SpagoConfig};
use crate::install::{install_all_dependencies, install_packages, InstallResult};
use crate::registry::{PackageQuery, PackageSet};

/// Execute the install command
pub async fn execute(
    packages: &[String],
    no_deps: bool,
    package_set: &PackageSet,
    verbose: bool,
) -> Result<()> {
    let spago_dir = PathBuf::from(".spago");

    if packages.is_empty() {
        // Install all dependencies from spago.yaml
        install_all_from_config(&spago_dir, verbose).await
    } else {
        // Install specific packages
        install_specific_packages(packages, no_deps, package_set, &spago_dir, verbose).await
    }
}

/// Install all dependencies from spago.yaml
async fn install_all_from_config(spago_dir: &PathBuf, verbose: bool) -> Result<()> {
    if verbose {
        println!("Loading spago.yaml configuration...");
    }

    let config =
        load_config("spago.yaml").context("Failed to load spago.yaml. Run 'spago init' first.")?;

    if verbose {
        println!("Package: {}", config.package.name.bright_cyan());
        let all_deps = config.all_dependencies();
        println!("Dependencies to install: {}", all_deps.len());
    }

    // Load package set
    let package_set_url = config
        .package_set_url()
        .context("Package set URL not found in spago.yaml")?;

    let package_set_tag = crate::config::extract_tag_from_url(package_set_url)
        .context("Failed to extract tag from package set URL")?;

    if verbose {
        println!("Package set tag: {}", package_set_tag.cyan());
        println!("Loading package set...");
    }

    let package_set = crate::registry::get_package_set(&package_set_tag, false)?;

    // Install all dependencies
    let result = install_all_dependencies(&config, &package_set, spago_dir).await?;

    // Report results
    if result.is_success() {
        println!(
            "{} Installation completed successfully!",
            "✓".green().bold()
        );

        if !result.installed.is_empty() {
            println!("\nInstalled packages:");
            for pkg in &result.installed {
                println!(
                    "  {} {} ({})",
                    "→".cyan(),
                    pkg.name.bright_cyan(),
                    pkg.version.dimmed()
                );
            }
        }

        if !result.skipped.is_empty() {
            println!("\nSkipped packages (already installed):");
            for pkg in &result.skipped {
                println!("  {} {}", "→".yellow(), pkg.dimmed());
            }
        }
    } else {
        println!("{} Installation failed:", "✗".red().bold());
        for error in &result.errors {
            println!("  {} {}", "✗".red(), error);
        }
        anyhow::bail!("Installation failed");
    }

    Ok(())
}

/// Install specific packages
async fn install_specific_packages(
    packages: &[String],
    no_deps: bool,
    package_set: &PackageSet,
    spago_dir: &PathBuf,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("Installing packages: {}", packages.join(", "));
        if no_deps {
            println!("Skipping dependencies (--no-deps)");
        }
    }

    // Validate packages exist in package set
    let query = PackageQuery::new(package_set);
    for package_name in packages {
        if !query.exists(package_name) {
            anyhow::bail!("Package '{}' not found in package set", package_name);
        }
    }

    // Install packages
    let result = if no_deps {
        // Install only the specified packages, no dependencies
        install_packages(packages, package_set, spago_dir).await?
    } else {
        // Install packages with all their dependencies
        install_packages(packages, package_set, spago_dir).await?
    };

    // Report results
    if result.is_success() {
        println!(
            "{} Installation completed successfully!",
            "✓".green().bold()
        );

        if !result.installed.is_empty() {
            println!("\nInstalled packages:");
            for pkg in &result.installed {
                println!(
                    "  {} {} ({})",
                    "→".cyan(),
                    pkg.name.bright_cyan(),
                    pkg.version.dimmed()
                );
            }
        }

        if !result.skipped.is_empty() {
            println!("\nSkipped packages (already installed):");
            for pkg in &result.skipped {
                println!("  {} {}", "→".yellow(), pkg.dimmed());
            }
        }
    } else {
        println!("{} Installation failed:", "✗".red().bold());
        for error in &result.errors {
            println!("  {} {}", "✗".red(), error);
        }
        anyhow::bail!("Installation failed");
    }

    Ok(())
}
