use anyhow::Result;
use std::collections::HashSet;
use std::fs;

use crate::config::SpagoConfig;
use crate::install::InstallManager;
use crate::registry::{PackageName, PackageQuery, PackageSet};

/// Clean up unused packages from .spago directory
pub fn cleanup_unused_packages(
    config: &SpagoConfig,
    package_set: &PackageSet,
) -> Result<Vec<String>> {
    let spago_dir = config.spago_dir();

    if !spago_dir.exists() {
        return Ok(Vec::new());
    }

    // Get all required packages (including transitive dependencies)
    let query = PackageQuery::new(package_set);
    let mut required_packages = HashSet::new();
    let mut processed = HashSet::new();

    let manager = InstallManager::new(&spago_dir)?;

    let mut dependencies_to_keep: Vec<PackageName> =
        config.package_dependencies().into_iter().cloned().collect();

    let all_local_deps: Vec<PackageName> = query.all_workspace_dependencies();

    dependencies_to_keep.extend(all_local_deps);
    dependencies_to_keep.sort_unstable();
    dependencies_to_keep.dedup();

    // Collect all dependencies from spago.yaml
    for dep in dependencies_to_keep {
        manager.collect_dependencies_recursive(
            &dep,
            &query,
            &mut required_packages,
            &mut processed,
        )?;
    }

    // Get all currently installed packages
    let mut installed_packages = Vec::new();
    let mut removed_packages = Vec::new();

    if let Ok(entries) = fs::read_dir(&spago_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let package_name = entry.file_name().to_string_lossy().to_string();

                // Check if this package is still required
                if !required_packages.contains(&PackageName::new(&package_name)) {
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
