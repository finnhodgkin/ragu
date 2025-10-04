use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::config::SpagoConfig;
use crate::registry::{PackageQuery, PackageSet};

/// Clean up unused packages from .spago directory
pub fn cleanup_unused_packages(
    config: &SpagoConfig,
    package_set: &PackageSet,
    spago_dir: &Path,
) -> Result<Vec<String>> {
    if !spago_dir.exists() {
        return Ok(Vec::new());
    }

    // Get all required packages (including transitive dependencies)
    let query = PackageQuery::new(package_set);
    let mut required_packages = HashSet::new();
    let mut processed = HashSet::new();

    // Collect all dependencies from spago.yaml
    for dep in config.all_dependencies() {
        collect_dependencies_recursive(
            dep,
            package_set,
            &query,
            &mut required_packages,
            &mut processed,
        )?;
    }

    // Get all currently installed packages
    let mut installed_packages = Vec::new();
    let mut removed_packages = Vec::new();

    if let Ok(entries) = fs::read_dir(spago_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let package_name = entry.file_name().to_string_lossy().to_string();

                // Extract clean package name (remove version suffix for regular packages)
                let clean_name = if package_name.contains('-') {
                    // Try to extract package name before version
                    let parts: Vec<&str> = package_name.split('-').collect();
                    if parts.len() >= 2 {
                        // Check if last part looks like a version (contains dots)
                        let last_part = parts.last().unwrap();
                        if last_part.contains('.') {
                            // Remove version part
                            parts[..parts.len() - 1].join("-")
                        } else {
                            package_name.clone()
                        }
                    } else {
                        package_name.clone()
                    }
                } else {
                    package_name.clone()
                };

                // Check if this package is still required
                if !required_packages.contains(&clean_name) {
                    // Remove the package
                    if let Err(e) = fs::remove_dir_all(&path) {
                        eprintln!(
                            "Warning: Failed to remove unused package {}: {}",
                            package_name, e
                        );
                    } else {
                        removed_packages.push(package_name);
                    }
                } else {
                    installed_packages.push(package_name);
                }
            }
        }
    }

    Ok(removed_packages)
}

/// Collect all dependencies recursively (same logic as install manager)
fn collect_dependencies_recursive(
    package_name: &str,
    package_set: &PackageSet,
    query: &PackageQuery,
    all_packages: &mut HashSet<String>,
    processed: &mut HashSet<String>,
) -> Result<()> {
    if processed.contains(package_name) {
        return Ok(());
    }

    processed.insert(package_name.to_string());

    // Get package info from package set
    if let Some(package) = package_set.get(package_name) {
        // Add dependencies first
        for dep_name in &package.dependencies {
            collect_dependencies_recursive(dep_name, package_set, query, all_packages, processed)?;
        }

        // Add this package
        all_packages.insert(package_name.to_string());
    }

    Ok(())
}
