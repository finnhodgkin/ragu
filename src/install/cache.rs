use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::registry::get_cache_dir;

/// Global package cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPackage {
    pub name: String,
    pub version: String,
    pub repo_url: String,
    pub cached_path: PathBuf,
    pub installed_at: chrono::DateTime<chrono::Utc>,
}

/// Global package cache manager
#[derive(Clone)]
pub struct GlobalPackageCache {
    cache_dir: PathBuf,
    index_path: PathBuf,
}

impl GlobalPackageCache {
    pub fn new() -> Result<Self> {
        let cache_dir = get_cache_dir()?.join("packages");
        let index_path = cache_dir.join("index.json");

        // Ensure cache directory exists
        fs::create_dir_all(&cache_dir)
            .context("Failed to create global package cache directory")?;

        Ok(Self {
            cache_dir,
            index_path,
        })
    }

    /// Load the cache index
    fn load_index(&self) -> Result<HashMap<String, CachedPackage>> {
        if !self.index_path.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(&self.index_path).context("Failed to read cache index")?;

        let index: HashMap<String, CachedPackage> =
            serde_json::from_str(&content).context("Failed to parse cache index")?;

        Ok(index)
    }

    /// Save the cache index
    fn save_index(&self, index: &HashMap<String, CachedPackage>) -> Result<()> {
        let content =
            serde_json::to_string_pretty(index).context("Failed to serialize cache index")?;

        fs::write(&self.index_path, content).context("Failed to write cache index")?;

        Ok(())
    }

    /// Check if a package is cached with the correct version
    pub fn is_cached(&self, name: &str, version: &str) -> Result<bool> {
        let index = self.load_index()?;
        Ok(index
            .get(name)
            .map(|cached| cached.version == version)
            .unwrap_or(false))
    }

    /// Get cached package path
    pub fn get_cached_path(&self, name: &str, version: &str) -> Result<Option<PathBuf>> {
        let index = self.load_index()?;
        Ok(index
            .get(name)
            .filter(|cached| cached.version == version)
            .map(|cached| cached.cached_path.clone()))
    }

    /// Add a package to the cache
    pub fn cache_package(&self, name: &str, version: &str, source_path: &Path) -> Result<PathBuf> {
        let cached_name = format!("{}-{}", name, version);
        let cached_path = self.cache_dir.join(&cached_name);

        // Copy the package to cache
        if cached_path.exists() {
            fs::remove_dir_all(&cached_path).context("Failed to remove existing cached package")?;
        }

        copy_dir_all(source_path, &cached_path).context("Failed to copy package to cache")?;

        // Update index
        let mut index = self.load_index()?;
        index.insert(
            name.to_string(),
            CachedPackage {
                name: name.to_string(),
                version: version.to_string(),
                repo_url: String::new(), // Will be filled by caller
                cached_path: cached_path.clone(),
                installed_at: chrono::Utc::now(),
            },
        );

        self.save_index(&index)?;
        Ok(cached_path)
    }

    /// Copy a package from cache to destination
    pub fn copy_from_cache(&self, name: &str, version: &str, dest_path: &Path) -> Result<()> {
        if let Some(cached_path) = self.get_cached_path(name, version)? {
            if dest_path.exists() {
                fs::remove_dir_all(dest_path).context("Failed to remove existing destination")?;
            }

            copy_dir_all(&cached_path, dest_path).context("Failed to copy from cache")?;
        } else {
            anyhow::bail!("Package {} version {} not found in cache", name, version);
        }

        Ok(())
    }

    /// Clear all cached packages
    pub fn clear_all(&self) -> Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)
                .context("Failed to remove package cache directory")?;
        }

        // Recreate the cache directory
        fs::create_dir_all(&self.cache_dir)
            .context("Failed to recreate package cache directory")?;

        Ok(())
    }
}

/// Recursively copy a directory
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
