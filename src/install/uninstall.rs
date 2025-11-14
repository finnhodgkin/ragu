use anyhow::{Context, Result};
use colored::Colorize;
use std::path::PathBuf;

use crate::config::{load_config_cwd, remove_packages_from_config};
use crate::install::cleanup_unused_packages;
use crate::registry::{PackageName, PackageSet};

/// Execute the uninstall command
pub async fn execute(
    packages: Vec<PackageName>,
    package_set: &PackageSet,
    verbose: bool,
) -> Result<()> {
    if packages.is_empty() {
        anyhow::bail!("No packages specified to uninstall");
    }

    if verbose {
        println!(
            "Uninstalling packages: {}",
            packages
                .iter()
                .map(|p| p.0.clone())
                .collect::<Vec<String>>()
                .join(", ")
        );
    }

    // Load current configuration
    let config = load_config_cwd().context("Failed to load spago.yaml. Run 'init' first.")?;

    // Validate that packages are actually installed
    for package_name in &packages {
        if !config.package.dependencies.contains(package_name) {
            anyhow::bail!(
                "Package '{}' is not installed in spago.yaml",
                package_name.0
            );
        }
    }

    // Remove packages from spago.yaml
    remove_packages_from_config(&PathBuf::from("spago.yaml"), &packages)
        .context("Failed to update spago.yaml")?;

    if verbose {
        println!("Removed packages from spago.yaml");
    }

    // Create updated config for cleanup
    let mut updated_config = config.clone();
    for package_name in packages {
        updated_config
            .package
            .dependencies
            .retain(|dep| dep != &package_name);
    }

    // Clean up unused packages from .spago directory
    let removed_packages = cleanup_unused_packages(&updated_config, package_set)?;

    // Report results
    if !removed_packages.is_empty() {
        if verbose {
            println!("\nRemoved packages:");
            for pkg in &removed_packages {
                println!("  {} {}", "→".red(), pkg.dimmed());
            }
        } else {
            println!(
                "{} Uninstalled {} packages",
                "✓".green().bold(),
                removed_packages.len()
            );
        }
    } else {
        println!("{} No packages were removed", "✓".green().bold());
    }

    Ok(())
}
