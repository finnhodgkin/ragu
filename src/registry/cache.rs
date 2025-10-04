use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

use super::types::PackageSet;

/// Cached tags with timestamp
#[derive(Debug, Serialize, Deserialize)]
struct CachedTags {
    tags: Vec<String>,
    fetched_at: DateTime<Utc>,
}

/// Default TTL for tag cache (24 hours)
const TAG_CACHE_TTL_HOURS: i64 = 24;

/// Get the cache directory for spago
pub fn get_cache_dir() -> Result<PathBuf> {
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
    let cache_dir = get_cache_dir()?;
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
        let remaining = max_age - age;
        let hours = remaining.num_hours();
        let minutes = remaining.num_minutes() % 60;
        println!("Loaded tags from cache (fresh for {}h {}m)", hours, minutes);
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

/// Clear the tags cache
pub fn clear_tags_cache() -> Result<()> {
    let cache_path = get_tags_cache_path()?;

    if cache_path.exists() {
        fs::remove_file(&cache_path).context("Failed to remove tags cache")?;
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
