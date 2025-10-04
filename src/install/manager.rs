use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::task;

use super::cache::GlobalPackageCache;
use super::git::{fetch_package, PackageInfo};
use crate::registry::{Package, PackageQuery, PackageSet};

/// Result of an installation operation
#[derive(Debug)]
pub struct InstallResult {
    pub installed: Vec<PackageInfo>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
}

impl InstallResult {
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Manages package installation in the .spago directory
pub struct InstallManager {
    spago_dir: PathBuf,
    global_cache: GlobalPackageCache,
}

impl InstallManager {
    pub fn new(spago_dir: &Path) -> Result<Self> {
        Ok(Self {
            spago_dir: spago_dir.to_path_buf(),
            global_cache: GlobalPackageCache::new()?,
        })
    }

    /// Install specified packages and their dependencies
    pub async fn install_packages(
        &self,
        package_names: &[String],
        package_set: &PackageSet,
    ) -> Result<InstallResult> {
        // Ensure .spago directory exists
        fs::create_dir_all(&self.spago_dir).context("Failed to create .spago directory")?;

        let query = PackageQuery::new(package_set);
        let mut all_packages = HashSet::new();
        let mut processed = HashSet::new();

        // Collect all packages to install (including dependencies)
        for package_name in package_names {
            self.collect_dependencies_recursive(
                package_name,
                package_set,
                &query,
                &mut all_packages,
                &mut processed,
            )?;
        }

        // Install packages in parallel
        let mut tasks = Vec::new();
        let package_set = Arc::new(package_set.clone());
        let spago_dir = self.spago_dir.clone();
        let global_cache = Arc::new(self.global_cache.clone());

        for package_name in all_packages {
            let package_set = package_set.clone();
            let spago_dir = spago_dir.clone();
            let global_cache = global_cache.clone();

            let task = task::spawn(async move {
                Self::install_single_package(&package_name, &package_set, &spago_dir, &global_cache)
                    .await
            });
            tasks.push(task);
        }

        // Wait for all tasks to complete
        let mut installed = Vec::new();
        let mut skipped = Vec::new();
        let mut errors = Vec::new();

        for task in tasks {
            match task.await? {
                Ok(package_info) => {
                    if let Some(info) = package_info {
                        installed.push(info);
                    }
                }
                Err(e) => {
                    errors.push(e.to_string());
                }
            }
        }

        Ok(InstallResult {
            installed,
            skipped,
            errors,
        })
    }

    /// Collect all dependencies recursively
    fn collect_dependencies_recursive(
        &self,
        package_name: &str,
        package_set: &PackageSet,
        query: &PackageQuery,
        all_packages: &mut HashSet<String>,
        processed: &mut HashSet<String>,
    ) -> Result<()> {
        if processed.contains(package_name) {
            return Ok(());
        }

        processed.insert(package_name.to_string());

        // Get package info from package set
        let package = package_set.get(package_name).ok_or_else(|| {
            anyhow::anyhow!("Package '{}' not found in package set", package_name)
        })?;

        // Add dependencies first
        for dep_name in &package.dependencies {
            self.collect_dependencies_recursive(
                dep_name,
                package_set,
                query,
                all_packages,
                processed,
            )?;
        }

        // Add this package
        all_packages.insert(package_name.to_string());
        Ok(())
    }

    /// Install a single package (used by parallel tasks)
    async fn install_single_package(
        package_name: &str,
        package_set: &PackageSet,
        spago_dir: &Path,
        global_cache: &GlobalPackageCache,
    ) -> Result<Option<PackageInfo>> {
        let package = package_set.get(package_name).ok_or_else(|| {
            anyhow::anyhow!("Package '{}' not found in package set", package_name)
        })?;

        let package_name_clean = super::git::extract_package_name(&package.repo);
        let folder_name = format!("{}-{}", package_name_clean, package.version);
        let package_dir = spago_dir.join(&folder_name);

        // Check if already installed
        if package_dir.exists() {
            return Ok(None); // Already installed
        }

        // Check global cache first
        if global_cache.is_cached(&package_name_clean, &package.version)? {
            // Copy from cache
            global_cache.copy_from_cache(&package_name_clean, &package.version, &package_dir)?;
            return Ok(Some(PackageInfo {
                name: package_name_clean,
                version: package.version.clone(),
                repo_url: package.repo.clone(),
                local_path: package_dir,
            }));
        }

        // Fetch from Git and cache
        let package_info = fetch_package(package, spago_dir)?;

        // Cache the package for future use
        global_cache.cache_package(
            &package_info.name,
            &package_info.version,
            &package_info.local_path,
        )?;

        Ok(Some(package_info))
    }
}
