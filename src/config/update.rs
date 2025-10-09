use anyhow::{Context, Result};
use serde_yaml::{self, Value};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use yaml_rust::{Yaml, YamlEmitter, YamlLoader};

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

    // Write back to file preserving formatting
    save_config_preserving_formatting(config_path, &config)?;

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

    // Write back to file preserving formatting
    save_config_preserving_formatting(config_path, &config)?;

    Ok(())
}

/// Load spago.yaml configuration as YAML value (preserves unknown fields)
fn load_config_as_value(path: &Path) -> Result<Value> {
    let content = fs::read_to_string(path).context("Failed to read spago.yaml")?;

    let config: Value = serde_yaml::from_str(&content).context("Failed to parse spago.yaml")?;

    Ok(config)
}

/// Save spago.yaml configuration preserving original formatting
fn save_config_preserving_formatting(path: &Path, config: &Value) -> Result<()> {
    // Read the original file content to preserve formatting
    let original_content =
        fs::read_to_string(path).context("Failed to read original spago.yaml")?;

    // Parse the original YAML with yaml-rust to preserve formatting
    let docs =
        YamlLoader::load_from_str(&original_content).context("Failed to parse original YAML")?;

    if docs.is_empty() {
        return Err(anyhow::anyhow!("No YAML documents found"));
    }

    let mut yaml_doc = docs[0].clone();

    // Get the new dependencies from the serde_yaml Value
    let dependencies = config
        .get("package")
        .and_then(|p| p.get("dependencies"))
        .context("Missing dependencies in config")?;

    let deps_array: Vec<String> = match dependencies {
        Value::Sequence(seq) => seq
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => return Err(anyhow::anyhow!("Dependencies must be a list")),
    };

    // Update the dependencies in the yaml-rust document
    if let Yaml::Hash(ref mut root_hash) = yaml_doc {
        if let Some(package_section) = root_hash.get_mut(&Yaml::String("package".to_string())) {
            if let Yaml::Hash(ref mut package_hash) = package_section {
                if let Some(deps) = package_hash.get_mut(&Yaml::String("dependencies".to_string()))
                {
                    *deps = Yaml::Array(
                        deps_array
                            .into_iter()
                            .map(|dep| Yaml::String(dep))
                            .collect(),
                    );
                }
            }
        }
    }

    // Emit the YAML with preserved formatting
    let mut out_str = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut out_str);
        emitter.dump(&yaml_doc).context("Failed to emit YAML")?;
    }

    // Remove the YAML document separator if present
    if out_str.starts_with("---\n") {
        out_str = out_str
            .strip_prefix("---\n")
            .unwrap_or(&out_str)
            .to_string();
    }

    // Ensure the file ends with a newline
    if !out_str.ends_with('\n') {
        out_str.push('\n');
    }

    fs::write(path, out_str).context("Failed to write spago.yaml")?;

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
