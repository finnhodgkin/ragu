use anyhow::{Context, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};

use crate::config::{add_packages_to_config, load_config, SpagoConfig};
use crate::install::{
    cleanup_unused_packages, install_all_dependencies, install_all_dependencies_with_config, install_packages,
    install_packages_with_config, InstallResult,
};
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

    // Install all dependencies with extra packages support
    let result = install_all_dependencies_with_config(&config, &package_set, spago_dir).await?;

    // Clean up unused packages
    let removed_packages = cleanup_unused_packages(&config, &package_set, spago_dir)?;

    // Report results
    if result.is_success() {
        let total_installed = result.installed.len();
        let total_skipped = result.skipped.len();

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

    // Load config for extra packages
    let config = load_config("spago.yaml").ok();

    // Validate packages exist in package set or are available as extra packages
    let query = PackageQuery::new(package_set);
    for package_name in packages {
        let is_in_package_set = query.exists(package_name);
        let is_available_extra_package = config
            .as_ref()
            .map(|c| c.workspace.extra_packages.contains_key(package_name))
            .unwrap_or(false);

        if !is_in_package_set && !is_available_extra_package {
            anyhow::bail!(
                "Package '{}' not found in package set or extra packages",
                package_name
            );
        }
    }

    // Install packages
    let result = if no_deps {
        // Install only the specified packages, no dependencies
        install_packages_with_config(packages, package_set, spago_dir, config.as_ref()).await?
    } else {
        // Install packages with all their dependencies
        install_packages_with_config(packages, package_set, spago_dir, config.as_ref()).await?
    };

    // Update spago.yaml with the new packages
    if result.is_success() && !packages.is_empty() {
        add_packages_to_config(&PathBuf::from("spago.yaml"), packages)
            .context("Failed to update spago.yaml")?;
    }

    // Report results
    if result.is_success() {
        let total_installed = result.installed.len();
        let total_skipped = result.skipped.len();

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
            // Concise summary for non-verbose mode
            if total_installed > 0 {
                // Show which packages were explicitly requested
                let requested_packages = packages.join(", ");
                let dependency_count = total_installed.saturating_sub(packages.len());

                if dependency_count > 0 {
                    println!(
                        "{} Installed {} with {} dependencies",
                        "✓".green().bold(),
                        requested_packages.bright_cyan(),
                        dependency_count
                    );
                } else {
                    println!(
                        "{} Installed {}",
                        "✓".green().bold(),
                        requested_packages.bright_cyan()
                    );
                }
            } else {
                println!("{} All packages already installed", "✓".green().bold());
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
