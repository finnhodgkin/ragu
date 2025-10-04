use anyhow::{Context, Result};
use serde_yaml;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::config::{PackageConfig, SpagoConfig};

/// Update spago.yaml with new packages
pub fn add_packages_to_config(config_path: &Path, new_packages: &[String]) -> Result<()> {
    // Load existing config
    let mut config = load_config(config_path)?;

    // Get current dependencies as a set
    let mut current_deps: HashSet<String> = config.package.dependencies.iter().cloned().collect();

    // Add new packages to the set
    for package in new_packages {
        current_deps.insert(package.clone());
    }

    // Convert back to sorted vector
    let mut updated_deps: Vec<String> = current_deps.into_iter().collect();
    updated_deps.sort();

    // Update the config
    config.package.dependencies = updated_deps;

    // Write back to file
    save_config(config_path, &config)?;

    Ok(())
}

/// Remove packages from spago.yaml
pub fn remove_packages_from_config(
    config_path: &Path,
    packages_to_remove: &[String],
) -> Result<()> {
    // Load existing config
    let mut config = load_config(config_path)?;

    // Remove packages from dependencies
    config
        .package
        .dependencies
        .retain(|dep| !packages_to_remove.contains(dep));

    // Write back to file
    save_config(config_path, &config)?;

    Ok(())
}

/// Load spago.yaml configuration
fn load_config(path: &Path) -> Result<SpagoConfig> {
    let content = fs::read_to_string(path).context("Failed to read spago.yaml")?;

    let config: SpagoConfig =
        serde_yaml::from_str(&content).context("Failed to parse spago.yaml")?;

    Ok(config)
}

/// Save spago.yaml configuration
fn save_config(path: &Path, config: &SpagoConfig) -> Result<()> {
    let content = serde_yaml::to_string(config).context("Failed to serialize spago.yaml")?;

    fs::write(path, content).context("Failed to write spago.yaml")?;

    Ok(())
}
