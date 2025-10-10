use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

use super::types::{PackageSet, RegistryIndex};

/// Cached tags with timestamp
#[derive(Debug, Serialize, Deserialize)]
struct CachedTags {
    tags: Vec<String>,
    fetched_at: DateTime<Utc>,
}

/// Cached registry versions with timestamp
#[derive(Debug, Serialize, Deserialize)]
struct CachedRegistryVersions {
    versions: Vec<String>,
    fetched_at: DateTime<Utc>,
}

/// Default TTL for tag cache (24 hours)
const TAG_CACHE_TTL_HOURS: i64 = 24;

/// Get the cache directory for spago
pub fn get_cache_dir() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .context("Failed to get system cache directory")?
        .join("spago-rust");

    fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;

    Ok(cache_dir)
}

/// Get the cache directory for the package sets
pub fn get_package_set_cache_dir() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .context("Failed to get system cache directory")?
        .join("spago-rust")
        .join("package-sets");

    fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;

    Ok(cache_dir)
}

/// Get the cache directory for metadata (tags, etc.)
fn get_metadata_cache_dir() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .context("Failed to get system cache directory")?
        .join("spago-rust")
        .join("metadata");

    fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;

    Ok(cache_dir)
}

/// Generate a cache key from the tag
fn cache_key(tag: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(tag.as_bytes());
    hex::encode(hasher.finalize())
}

/// Get the path to the cached package set for a given tag
pub fn get_cache_path(tag: &str) -> Result<PathBuf> {
    let cache_dir = get_package_set_cache_dir()?;
    let key = cache_key(tag);
    Ok(cache_dir.join(format!("{}.bin", key)))
}

/// Try to load a package set from the cache
pub fn load_from_cache(tag: &str) -> Result<Option<PackageSet>> {
    let cache_path = get_cache_path(tag)?;

    if !cache_path.exists() {
        return Ok(None);
    }

    let cached_data = fs::read(&cache_path).context("Failed to read cache file")?;

    let package_set: PackageSet =
        bincode::deserialize(&cached_data).context("Failed to deserialize cached package set")?;

    Ok(Some(package_set))
}

/// Save a package set to the cache
pub fn save_to_cache(tag: &str, package_set: &PackageSet) -> Result<()> {
    let cache_path = get_cache_path(tag)?;

    let encoded = bincode::serialize(package_set).context("Failed to serialize package set")?;

    fs::write(&cache_path, encoded).context("Failed to write cache file")?;

    Ok(())
}

/// Clear the entire package set cache
pub fn clear_cache() -> Result<()> {
    let cache_dir = get_cache_dir()?;

    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir).context("Failed to clear cache directory")?;
    }

    Ok(())
}

/// Clear a specific cached package set by tag
pub fn clear_cache_for_tag(tag: &str) -> Result<()> {
    let cache_path = get_cache_path(tag)?;

    if cache_path.exists() {
        fs::remove_file(&cache_path)
            .context(format!("Failed to remove cache for tag '{}'", tag))?;
    }

    Ok(())
}

/// Get path to the tags cache file
fn get_tags_cache_path() -> Result<PathBuf> {
    let cache_dir = get_metadata_cache_dir()?;
    Ok(cache_dir.join("tags.json"))
}

/// Get path to the registry versions cache file
fn get_registry_versions_cache_path() -> Result<PathBuf> {
    let cache_dir = get_metadata_cache_dir()?;
    Ok(cache_dir.join("registry-versions.json"))
}

/// Load cached tags if they exist and are fresh
pub fn load_cached_tags(ttl_hours: Option<i64>) -> Result<Option<Vec<String>>> {
    let cache_path = get_tags_cache_path()?;

    if !cache_path.exists() {
        return Ok(None);
    }

    let cached_data = fs::read_to_string(&cache_path).context("Failed to read tags cache file")?;

    let cached: CachedTags =
        serde_json::from_str(&cached_data).context("Failed to deserialize cached tags")?;

    // Check if cache is still fresh
    let ttl = ttl_hours.unwrap_or(TAG_CACHE_TTL_HOURS);
    let age = Utc::now().signed_duration_since(cached.fetched_at);
    let max_age = Duration::hours(ttl);

    if age < max_age {
        // Cache is fresh - return silently
        // Note: We removed the "Loaded tags from cache" message
        // to keep output minimal by default
        Ok(Some(cached.tags))
    } else {
        // Cache is stale
        Ok(None)
    }
}

/// Save tags to cache with current timestamp
pub fn save_cached_tags(tags: &[String]) -> Result<()> {
    let cache_path = get_tags_cache_path()?;

    let cached = CachedTags {
        tags: tags.to_vec(),
        fetched_at: Utc::now(),
    };

    let json = serde_json::to_string_pretty(&cached).context("Failed to serialize tags cache")?;

    fs::write(&cache_path, json).context("Failed to write tags cache file")?;

    Ok(())
}

/// Load cached registry versions if they exist and are fresh
pub fn load_cached_registry_versions(ttl_hours: Option<i64>) -> Result<Option<Vec<String>>> {
    let cache_path = get_registry_versions_cache_path()?;

    if !cache_path.exists() {
        return Ok(None);
    }

    let cached_data =
        fs::read_to_string(&cache_path).context("Failed to read registry versions cache file")?;

    let cached: CachedRegistryVersions = serde_json::from_str(&cached_data)
        .context("Failed to deserialize cached registry versions")?;

    // Check if cache is still fresh
    let ttl = ttl_hours.unwrap_or(TAG_CACHE_TTL_HOURS);
    let age = Utc::now().signed_duration_since(cached.fetched_at);
    let max_age = Duration::hours(ttl);

    if age < max_age {
        // Cache is fresh - return silently
        // Note: We removed the "Loaded registry versions from cache" message
        // to keep output minimal by default
        Ok(Some(cached.versions))
    } else {
        // Cache is stale
        Ok(None)
    }
}

/// Save registry versions to cache with current timestamp
pub fn save_cached_registry_versions(versions: &[String]) -> Result<()> {
    let cache_path = get_registry_versions_cache_path()?;

    let cached = CachedRegistryVersions {
        versions: versions.to_vec(),
        fetched_at: Utc::now(),
    };

    let json = serde_json::to_string_pretty(&cached)
        .context("Failed to serialize registry versions cache")?;

    fs::write(&cache_path, json).context("Failed to write registry versions cache file")?;

    Ok(())
}

/// Get the registry cache directory
pub fn get_registry_cache_dir() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .context("Failed to get system cache directory")?
        .join("spago-rust")
        .join("registry");

    fs::create_dir_all(&cache_dir).context("Failed to create registry cache directory")?;

    Ok(cache_dir)
}

/// Get the path to the cached registry index
pub fn get_registry_index_cache_path() -> Result<PathBuf> {
    let cache_dir = get_registry_cache_dir()?;
    Ok(cache_dir.join("index.bin"))
}

/// Get the path to the cached registry package set for a given version
pub fn get_registry_package_set_cache_path(version: &str) -> Result<PathBuf> {
    let cache_dir = get_registry_cache_dir()?;
    let key = cache_key(version);
    Ok(cache_dir.join(format!("package-set-{}.bin", key)))
}

/// Load registry index from cache
pub fn load_registry_index_from_cache() -> Result<Option<RegistryIndex>> {
    let cache_path = get_registry_index_cache_path()?;

    if !cache_path.exists() {
        return Ok(None);
    }

    let cached_data = fs::read(&cache_path).context("Failed to read registry index cache file")?;

    let registry_index: RegistryIndex = bincode::deserialize(&cached_data)
        .context("Failed to deserialize cached registry index")?;

    Ok(Some(registry_index))
}

/// Save registry index to cache
pub fn save_registry_index_to_cache(registry_index: &RegistryIndex) -> Result<()> {
    let cache_path = get_registry_index_cache_path()?;

    let encoded =
        bincode::serialize(registry_index).context("Failed to serialize registry index")?;

    fs::write(&cache_path, encoded).context("Failed to write registry index cache file")?;

    Ok(())
}

/// Load registry package set from cache
pub fn load_registry_package_set_from_cache(version: &str) -> Result<Option<PackageSet>> {
    let cache_path = get_registry_package_set_cache_path(version)?;

    if !cache_path.exists() {
        return Ok(None);
    }

    let cached_data =
        fs::read(&cache_path).context("Failed to read registry package set cache file")?;

    let package_set: PackageSet = bincode::deserialize(&cached_data)
        .context("Failed to deserialize cached registry package set")?;

    Ok(Some(package_set))
}

/// Save registry package set to cache
pub fn save_registry_package_set_to_cache(version: &str, package_set: &PackageSet) -> Result<()> {
    let cache_path = get_registry_package_set_cache_path(version)?;

    let encoded =
        bincode::serialize(package_set).context("Failed to serialize registry package set")?;

    fs::write(&cache_path, encoded).context("Failed to write registry package set cache file")?;

    Ok(())
}

/// Clear a specific cached registry package set by version
pub fn clear_registry_package_set_cache(version: &str) -> Result<()> {
    let cache_path = get_registry_package_set_cache_path(version)?;

    if cache_path.exists() {
        fs::remove_file(&cache_path).context(format!(
            "Failed to remove registry cache for version '{}'",
            version
        ))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        let key1 = cache_key("psc-0.15.15-20251004");
        let key2 = cache_key("psc-0.15.15-20251004");
        let key3 = cache_key("psc-0.15.14-20251003");

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
}
