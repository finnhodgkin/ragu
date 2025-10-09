use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tempfile;
use walkdir::WalkDir;

use crate::{
    config::load_config_cwd,
    registry::{
        add_workspace_packages, clear_registry_package_set_cache, load_registry_index_from_cache,
        load_registry_package_set_from_cache,
        package_sets::add_extra_packages,
        save_registry_index_to_cache, save_registry_package_set_to_cache,
        types::{RegistryIndex, RegistryPackage},
        Package, PackageName, PackageSet,
    },
};

/// Get a package set by registry version, using cache if available
///
/// This function will:
/// 1. Check if the package set is cached locally (as binary for speed)
/// 2. If cached, load it from disk
/// 3. If not cached, fetch from GitHub and cache it
///
/// # Arguments
/// * `registry_version` - The registry version (e.g., "62.1.0")
/// * `force_refresh` - If true, bypass cache and fetch fresh from GitHub
pub fn get_package_set_by_registry_version(
    registry_version: &str,
    force_refresh: bool,
) -> Result<PackageSet> {
    // Try loading from cache first (unless force refresh)

    let mut package_set = match load_registry_package_set_from_cache(registry_version) {
        Ok(Some(cached)) if !force_refresh => cached,
        Ok(_) => {
            let fetched = fetch_registry_package_set(registry_version)?;
            // Save to cache for future use
            save_registry_package_set_to_cache(registry_version, &fetched)?;
            fetched
        }
        Err(_) => {
            // Clear cache on error
            clear_registry_package_set_cache(registry_version)?;
            let fetched = fetch_registry_package_set(registry_version)?;
            // Save to cache for future use
            save_registry_package_set_to_cache(registry_version, &fetched)?;
            fetched
        }
    };

    let config = load_config_cwd()?;

    let extra_packages = config.workspace.extra_packages;

    // Add extra packages. These won't be saved to cache because they are not part of the package set.
    add_extra_packages(&mut package_set, &extra_packages);
    // Add local workspace packages.
    add_workspace_packages(&mut package_set, &config.workspace_root);

    Ok(package_set)
}

fn fetch_registry_package_set(registry_version: &str) -> Result<PackageSet> {
    let registry_set = fetch_registry_package_set_from_github(registry_version)?;
    let index = fetch_registry_index_from_github_or_cache()?;

    let mut package_set = HashMap::new();

    for (name, version) in registry_set.0 {
        let registry_package = index.get_package(&name, &version);
        if let Some(registry_package) = registry_package {
            package_set.insert(name, Package::Registry(registry_package.clone()));
        }
    }

    Ok(package_set)
}

const REGISTRY_REPO_URL: &str = "https://github.com/purescript/registry/blob/main/package-sets/";

fn fetch_registry_package_set_from_github(registry_version: &str) -> Result<RegistryPackageSet> {
    let url = format!("https://raw.githubusercontent.com/purescript/registry/refs/heads/main/package-sets/{}.json", registry_version);

    println!("Fetching package set from: {}", url);

    let response =
        reqwest::blocking::get(&url).context("Failed to fetch package set from GitHub")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch package set: HTTP {} for version '{}'",
            response.status(),
            registry_version
        );
    }

    #[derive(Deserialize)]
    struct RawPackageSet {
        packages: HashMap<String, String>,
    }

    let raw_package_set: RawPackageSet = response
        .json()
        .context("Failed to parse package set JSON")?;

    let package_set = raw_package_set
        .packages
        .into_iter()
        .map(|(name, version)| (PackageName::new(&name), version))
        .collect();

    Ok(RegistryPackageSet(package_set))
}

fn fetch_registry_index_from_github_or_cache() -> Result<RegistryIndex> {
    match load_registry_index_from_cache()? {
        Some(cached) => Ok(cached),
        None => {
            let index = "https://github.com/purescript/registry-index.git";
            let temp_dir = tempfile::tempdir()?;
            let repo = git2::Repository::clone(index, temp_dir.path())?;

            // Walk through all files in the repository
            let mut registry_map: HashMap<PackageName, HashMap<String, RegistryPackage>> =
                HashMap::new();
            for entry in WalkDir::new(repo.workdir().unwrap())
                .into_iter()
                .filter_entry(|e| {
                    !e.path().ends_with(".git") && !(e.file_name().to_str() == Some("README.md"))
                })
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let contents = std::fs::read_to_string(entry.path())?;
                for line in contents.lines() {
                    if !line.trim().is_empty() {
                        // Parse the raw JSON first into a temporary structure
                        let raw_json: serde_json::Value = serde_json::from_str(line)?;

                        // Extract the required fields to construct a RegistryPackage
                        let name = PackageName::new(
                            raw_json["name"]
                                .as_str()
                                .context("No name found for package")?,
                        );
                        let version = raw_json["version"]
                            .as_str()
                            .context("Version not found")?
                            .to_string();

                        // Extract dependencies from the dependencies object
                        let dependencies = raw_json["dependencies"]
                            .as_object()
                            .unwrap()
                            .keys()
                            .map(|dep| PackageName::new(dep))
                            .collect();

                        // Create the RegistryPackage
                        let registry_package = RegistryPackage {
                            name: name.clone(),
                            version: version.clone(),
                            dependencies,
                        };

                        // Insert into nested HashMap structure
                        registry_map
                            .entry(name)
                            .or_insert_with(HashMap::new)
                            .insert(version, registry_package);
                    }
                }
            }
            let registry_index = RegistryIndex(registry_map);
            // Save to cache for future use
            save_registry_index_to_cache(&registry_index)?;
            Ok(registry_index)
        }
    }
}
struct RegistryPackageSet(HashMap<PackageName, String>);

impl RegistryIndex {
    fn get_package(&self, name: &PackageName, version: &str) -> Option<&RegistryPackage> {
        self.0.get(name).and_then(|versions| versions.get(version))
    }

    fn get_versions(&self, name: &PackageName) -> Option<Vec<&str>> {
        self.0
            .get(name)
            .map(|versions| versions.keys().map(|v| v.as_str()).collect())
    }
}
