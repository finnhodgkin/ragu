use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::registry::{get_cache_dir, PackageName};

/// Global package cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPackage {
    pub name: PackageName,
    pub version: String,
    pub key: String,
    pub cached_path: PathBuf,
    pub installed_at: chrono::DateTime<chrono::Utc>,
}

/// Global package cache manager
#[derive(Clone)]
pub struct GlobalPackageCache {
    cache_dir: PathBuf,
    index_path: PathBuf,
}

pub const CACHE_KEY: &str = concat!("spago-rust@", env!("CARGO_PKG_VERSION"));

type Index = HashMap<PackageName, CachedPackage>;

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
    fn load_index(&self) -> Result<Index> {
        if !self.index_path.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(&self.index_path).context("Failed to read cache index")?;

        let index: Index = serde_json::from_str(&content).unwrap_or(HashMap::new());

        Ok(index)
    }

    /// Save the cache index
    fn save_index(&self, index: &Index) -> Result<()> {
        let content =
            serde_json::to_string_pretty(index).context("Failed to serialize cache index")?;

        fs::write(&self.index_path, content).context("Failed to write cache index")?;

        Ok(())
    }

    /// Check if a package is cached with the correct version
    pub fn is_cached(&self, name: &PackageName, version: &str) -> Result<bool> {
        let index = self.load_index()?;
        Ok(index
            .get(name)
            .map(|cached| cached.version == version)
            .unwrap_or(false))
    }

    /// Get cached package path
    pub fn get_cached_path(&self, name: &PackageName, version: &str) -> Result<Option<PathBuf>> {
        let index = self.load_index()?;
        Ok(index
            .get(name)
            .filter(|cached| cached.version == version && cached.key == CACHE_KEY)
            .map(|cached| cached.cached_path.clone()))
    }

    /// Add a package to the cache
    pub fn cache_package(
        &self,
        name: &PackageName,
        version: &str,
        source_path: &Path,
    ) -> Result<PathBuf> {
        let cached_name = format!("{}-{}-{}", name.0, version, CACHE_KEY);
        let cached_path = self.cache_dir.join(&cached_name);

        // Copy the package to cache
        if cached_path.exists() {
            fs::remove_dir_all(&cached_path).context("Failed to remove existing cached package")?;
        }

        copy_dir_all(source_path, &cached_path).context("Failed to copy package to cache")?;

        // Update index
        let mut index = self.load_index()?;
        index.insert(
            name.clone(),
            CachedPackage {
                name: name.clone(),
                version: version.to_string(),
                key: CACHE_KEY.to_string(),
                cached_path: cached_path.clone(),
                installed_at: chrono::Utc::now(),
            },
        );

        self.save_index(&index)?;
        Ok(cached_path)
    }

    /// Copy a package from cache to destination
    pub fn copy_from_cache(
        &self,
        name: &PackageName,
        version: &str,
        dest_path: &Path,
    ) -> Result<()> {
        if let Some(cached_path) = self.get_cached_path(name, version)? {
            if dest_path.exists() {
                fs::remove_dir_all(dest_path).context("Failed to remove existing destination")?;
            }

            copy_dir_all(&cached_path, dest_path).context("Failed to copy from cache")?;
        } else {
            anyhow::bail!("Package {} version {} not found in cache", name.0, version);
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
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
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
