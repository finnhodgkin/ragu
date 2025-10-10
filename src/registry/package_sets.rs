use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::config::{load_config_cwd, ExtraPackageConfig};
use crate::registry::cache::{load_cached_registry_versions, save_cached_registry_versions};
use crate::registry::types::{PackageInSet, PackageName, PackageSetPackage};
use crate::registry::{add_workspace_packages, clear_cache_for_tag, LocalPackage, Package};

use super::cache::{load_cached_tags, load_from_cache, save_cached_tags, save_to_cache};
use super::types::PackageSet;

/// The GitHub raw content URL for package sets
const GITHUB_RAW_URL: &str = "https://raw.githubusercontent.com/purescript/package-sets";

/// Fetch a package set from GitHub by tag
fn fetch_from_github(tag: &str) -> Result<PackageSet> {
    let url = format!("{}/{}/packages.json", GITHUB_RAW_URL, tag);

    println!("Fetching package set from: {}", url);

    let response =
        reqwest::blocking::get(&url).context("Failed to fetch package set from GitHub")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch package set: HTTP {} for tag '{}'",
            response.status(),
            tag
        );
    }

    let package_set: PackageSet = response
        .json()
        .map(|packages: HashMap<PackageName, PackageInSet>| {
            packages
                .into_iter()
                .map(|(name, package)| {
                    (
                        name.clone(),
                        Package::Remote(PackageSetPackage {
                            name: name.clone(),
                            dependencies: package.dependencies,
                            repo: package.repo,
                            version: package.version,
                        }),
                    )
                })
                .collect()
        })
        .context("Failed to parse package set JSON")?;

    Ok(package_set)
}

/// Get a package set by tag, using cache if available
///
/// This function will:
/// 1. Check if the package set is cached locally (as binary for speed)
/// 2. If cached, load it from disk
/// 3. If not cached, fetch from GitHub and cache it
///
/// # Arguments
/// * `tag` - The git tag of the package set (e.g., "psc-0.15.15-20251004")
/// * `force_refresh` - If true, bypass cache and fetch fresh from GitHub
pub fn get_package_set(tag: &str, force_refresh: bool) -> Result<PackageSet> {
    // Try loading from cache first (unless force refresh)

    let mut package_set = match load_from_cache(tag) {
        Ok(Some(cached)) if !force_refresh => cached,
        Ok(_) => fetch_from_github(tag)?,
        Err(_) => {
            clear_cache_for_tag(tag)?;
            fetch_from_github(tag)?
        }
    };

    // Save to cache
    save_to_cache(tag, &package_set)?;

    let config = load_config_cwd()?;

    let extra_packages = config.workspace.extra_packages;

    // Add extra packages. These won't be saved to cache because they are not part of the package set.
    add_extra_packages(&mut package_set, &extra_packages);
    // Add local workspace packages.
    add_workspace_packages(&mut package_set, &config.workspace_root);

    Ok(package_set)
}

pub fn add_extra_packages(
    package_set: &mut PackageSet,
    extra_packages: &HashMap<PackageName, ExtraPackageConfig>,
) {
    for (name, package) in extra_packages {
        match (package.git.as_ref(), package.path.as_ref()) {
            (Some(git), None) => {
                package_set.insert(
                    name.clone(),
                    Package::Remote(PackageSetPackage {
                        name: name.clone(),
                        dependencies: package
                            .dependencies
                            .as_ref()
                            .unwrap_or(&vec![])
                            .iter()
                            .map(|d| PackageName::new(d))
                            .collect(),
                        repo: git.clone(),
                        version: package.ref_.clone().unwrap_or_default(),
                    }),
                );
            }
            (None, Some(path)) => {
                package_set.insert(
                    name.clone(),
                    Package::Local(LocalPackage {
                        name: name.clone(),
                        dependencies: package
                            .dependencies
                            .as_ref()
                            .unwrap_or(&vec![])
                            .iter()
                            .map(|d| PackageName::new(d))
                            .collect(),
                        path: PathBuf::from(path.clone()),
                    }),
                );
            }
            _ => {}
        }
    }
}

/// Response from GitHub API when listing tags
#[derive(Debug, Deserialize)]
struct GitHubTag {
    name: String,
}

/// Fetch tags from GitHub API (without cache)
fn fetch_tags_from_github() -> Result<Vec<String>> {
    let url = "https://api.github.com/repos/purescript/package-sets/tags?per_page=100";

    println!("Fetching available tags from GitHub API...");

    let client = reqwest::blocking::Client::builder()
        .user_agent("spago-rust/0.1.0") // GitHub API requires a user agent
        .build()?;

    let response = client
        .get(url)
        .send()
        .context("Failed to fetch tags from GitHub API")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch tags: HTTP {} - {}",
            response.status(),
            response.text().unwrap_or_default()
        );
    }

    let tags: Vec<GitHubTag> = response
        .json()
        .context("Failed to parse GitHub API response")?;

    let tag_names: Vec<String> = tags.into_iter().map(|t| t.name).collect();

    Ok(tag_names)
}

/// List available tags with custom TTL or force refresh
///
/// # Arguments
/// * `force_refresh` - If true, bypass cache and fetch fresh from GitHub
/// * `ttl_hours` - Custom TTL in hours (None = use default 24 hours)
pub fn list_available_tags_with_options(
    force_refresh: bool,
    ttl_hours: Option<i64>,
) -> Result<Vec<String>> {
    if !force_refresh {
        // Try to load from cache with custom TTL
        if let Some(cached_tags) = load_cached_tags(ttl_hours)? {
            return Ok(cached_tags);
        }
    }

    // Force refresh or cache miss - fetch from GitHub
    let tags = fetch_tags_from_github()?;

    // Save to cache
    save_cached_tags(&tags)?;

    Ok(tags)
}

/// List available registry versions with custom TTL or force refresh
pub fn list_available_registry_versions_with_options(
    force_refresh: bool,
    ttl_hours: Option<i64>,
) -> Result<Vec<String>> {
    if !force_refresh {
        // Try to load from cache with custom TTL
        if let Some(cached_tags) = load_cached_registry_versions(ttl_hours)? {
            return Ok(cached_tags);
        }
    }

    // Clone registry repo to temp dir
    let temp_dir = tempfile::tempdir()?;
    git2::Repository::clone(
        "https://github.com/purescript/registry.git",
        temp_dir.path(),
    )
    .context("Failed to clone the registry repository to check for new registry versions")?;

    // Look for package set files in package-sets directory
    let package_sets_dir = temp_dir.path().join("package-sets");
    let mut versions = Vec::new();

    for entry in std::fs::read_dir(package_sets_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Some(version) = path.file_stem().and_then(|s| s.to_str()) {
                versions.push(version.to_string());
            }
        }
    }

    // Sort versions in descending order using semantic versioning comparison
    versions.sort_by(|a, b| {
        // Split version strings into numeric components
        let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
        let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();

        // Compare components from most significant to least significant
        // This will compare all parts (major.minor.patch)
        b_parts
            .iter()
            .zip(a_parts.iter())
            .find(|(b_num, a_num)| b_num != a_num)
            .map(|(b_num, a_num)| a_num.cmp(b_num))
            .unwrap_or_else(|| b_parts.len().cmp(&a_parts.len()))
    });

    versions.reverse();

    // Save to cache
    save_cached_registry_versions(&versions)?;

    Ok(versions)
}
