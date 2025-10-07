use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::config::{load_config_cwd, ExtraPackageConfig};
use crate::registry::types::{PackageInSet, PackageName, PackageSetPackage};
use crate::registry::{clear_cache_for_tag, LocalPackage, Package};

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

    Ok(package_set)
}

fn add_extra_packages(
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

/// List all available tags from the package-sets repository
///
/// Uses cached tags if available and fresh (< 24 hours old).
/// Falls back to GitHub API if cache is stale or missing.
///
/// # Notes
/// - Uses 24-hour cache to avoid excessive API calls
/// - GitHub API has rate limits (60 requests/hour for unauthenticated requests)
/// - Returns up to 100 most recent tags (GitHub API pagination limit)
/// - Tags are returned in chronological order (newest first)
pub fn list_available_tags() -> Result<Vec<String>> {
    // Try to load from cache first
    if let Some(cached_tags) = load_cached_tags(None)? {
        return Ok(cached_tags);
    }

    // Cache miss or stale - fetch from GitHub
    let tags = fetch_tags_from_github()?;

    // Save to cache for next time
    save_cached_tags(&tags)?;

    Ok(tags)
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

/// Get the latest (most recent) package set tag
///
/// This is a convenience function that fetches available tags and returns the first one.
/// Useful when you don't need to list all tags and just want the latest version.
pub fn get_latest_tag() -> Result<String> {
    let tags = list_available_tags()?;

    tags.first()
        .cloned()
        .context("No tags available in the package-sets repository")
}
