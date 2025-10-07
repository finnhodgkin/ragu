use anyhow::{Context, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};

use crate::config::{add_packages_to_config, load_config, SpagoConfig};
use crate::install::{cleanup_unused_packages, install_all_dependencies, InstallResult};
use crate::registry::{PackageName, PackageQuery, PackageSet};

/// Execute the install command
pub async fn execute(packages: &[String], package_set: &PackageSet, verbose: bool) -> Result<()> {
    let spago_dir = PathBuf::from(".spago");

    if packages.is_empty() {
        // Install all dependencies from spago.yaml
        install_all_from_config(&spago_dir, verbose).await
    } else {
        // Install specific packages
        install_specific_packages(packages, package_set, &spago_dir, verbose).await
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
        println!("Package: {}", config.package.name.0.bright_cyan());
        let all_deps = config.all_dependencies();
        println!("Dependencies to install: {}", all_deps.len());
    }

    // Load package set
    let package_set = config.package_set()?;

    // Install all dependencies
    let result = install_all_dependencies(&config, &package_set, spago_dir).await?;

    // Clean up unused packages
    let removed_packages = cleanup_unused_packages(&config, &package_set, spago_dir)?;

    // Report results
    if result.is_success() {
        let total_installed = result.installed.len();

        if verbose {
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
                        pkg.name().0.bright_cyan(),
                        pkg.version().unwrap_or(&"local".to_string()).dimmed()
                    );
                }
            }
        } else {
            // Concise summary for non-verbose mode
            if total_installed > 0 {
                println!(
                    "{} Installed {} dependencies",
                    "✓".green().bold(),
                    total_installed
                );
            } else {
                println!("{} All packages already installed", "✓".green().bold());
            }

            // Report cleanup
            if !removed_packages.is_empty() {
                println!(
                    "  Removed {} unused packages",
                    removed_packages.len().to_string().yellow()
                );
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
    package_set: &PackageSet,
    spago_dir: &PathBuf,
    verbose: bool,
) -> Result<()> {
    // Validate packages exist in package set
    let query = PackageQuery::new(package_set);
    for package_name in packages {
        let package = PackageName::new(package_name);
        let is_in_package_set = query.exists(&package);

        if !is_in_package_set {
            anyhow::bail!("Package '{}' not found in package set", package_name);
        }
    }

    let packages = packages
        .iter()
        .map(|p| PackageName::new(p))
        .collect::<Vec<_>>();

    // Update spago.yaml with the new packages
    add_packages_to_config(&PathBuf::from("spago.yaml"), &packages)
        .context("Failed to update spago.yaml")?;

    // Install packages with all their dependencies
    install_all_from_config(spago_dir, verbose).await?;

    Ok(())
}
