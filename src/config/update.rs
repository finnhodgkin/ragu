use anyhow::{Context, Result};
use serde_yaml::{self, Value};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::registry::PackageName;

/// Update spago.yaml with new packages
pub fn add_packages_to_config(config_path: &Path, new_packages: &[PackageName]) -> Result<()> {
    // Load existing config as YAML value
    let mut config = load_config_as_value(config_path)?;

    // Get current dependencies from the YAML
    let current_deps = get_dependencies_from_value(&config)?;
    let mut current_deps_set: HashSet<PackageName> = current_deps.into_iter().collect();

    // Add new packages to the set
    for package in new_packages {
        current_deps_set.insert(package.clone());
    }

    // Convert back to sorted vector
    let mut updated_deps: Vec<PackageName> = current_deps_set.into_iter().collect();
    updated_deps.sort();

    // Update the dependencies in the YAML value
    update_dependencies_in_value(&mut config, updated_deps)?;

    // Write back to file
    save_config_as_value(config_path, &config)?;

    Ok(())
}

/// Remove packages from spago.yaml
pub fn remove_packages_from_config(
    config_path: &Path,
    packages_to_remove: &Vec<PackageName>,
) -> Result<()> {
    // Load existing config as YAML value
    let mut config = load_config_as_value(config_path)?;

    // Get current dependencies from the YAML
    let current_deps = get_dependencies_from_value(&config)?;

    // Remove packages from dependencies
    let updated_deps: Vec<PackageName> = current_deps
        .into_iter()
        .filter(|dep| !packages_to_remove.contains(dep))
        .collect();

    // Update the dependencies in the YAML value
    update_dependencies_in_value(&mut config, updated_deps)?;

    // Write back to file
    save_config_as_value(config_path, &config)?;

    Ok(())
}

/// Load spago.yaml configuration as YAML value (preserves unknown fields)
fn load_config_as_value(path: &Path) -> Result<Value> {
    let content = fs::read_to_string(path).context("Failed to read spago.yaml")?;

    let config: Value = serde_yaml::from_str(&content).context("Failed to parse spago.yaml")?;

    Ok(config)
}

/// Save spago.yaml configuration from YAML value
fn save_config_as_value(path: &Path, config: &Value) -> Result<()> {
    let content = serde_yaml::to_string(config).context("Failed to serialize spago.yaml")?;

    fs::write(path, content).context("Failed to write spago.yaml")?;

    Ok(())
}

/// Extract dependencies from YAML value
fn get_dependencies_from_value(config: &Value) -> Result<Vec<PackageName>> {
    let package = config
        .get("package")
        .context("Missing 'package' section in spago.yaml")?;

    let dependencies = package
        .get("dependencies")
        .context("Missing 'dependencies' in package section")?;

    match dependencies {
        Value::Sequence(seq) => {
            let deps: Result<Vec<PackageName>, _> = seq
                .iter()
                .map(|v| {
                    if let Value::String(s) = v {
                        Ok(PackageName(s.clone()))
                    } else {
                        Err(anyhow::anyhow!("Invalid dependency format"))
                    }
                })
                .collect();
            deps
        }
        _ => Err(anyhow::anyhow!("Dependencies must be a list")),
    }
}

/// Update dependencies in YAML value
fn update_dependencies_in_value(config: &mut Value, dependencies: Vec<PackageName>) -> Result<()> {
    let package = config
        .get_mut("package")
        .context("Missing 'package' section in spago.yaml")?;

    let deps_value: Value = dependencies
        .into_iter()
        .map(|dep| Value::String(dep.0))
        .collect();

    package["dependencies"] = deps_value;

    Ok(())
}
