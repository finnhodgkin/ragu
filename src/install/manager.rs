use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::task;

use super::cache::GlobalPackageCache;
use super::git::{fetch_package, PackageInfo};
use crate::config::{ExtraPackageConfig, SpagoConfig};
use crate::registry::{Package, PackageName, PackageQuery, PackageSet};

/// Result of an installation operation
#[derive(Debug)]
pub struct InstallResult {
    pub installed: Vec<InstalledPackage>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum InstalledPackage {
    Git(PackageInfo),
    Local(LocalPackageInfo),
}

impl InstalledPackage {
    pub fn name(&self) -> &PackageName {
        match self {
            InstalledPackage::Git(package) => &package.name,
            InstalledPackage::Local(package) => &package.name,
        }
    }

    pub fn version(&self) -> Option<&String> {
        match self {
            InstalledPackage::Git(package) => Some(&package.version),
            InstalledPackage::Local(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalPackageInfo {
    pub name: PackageName,
    pub path: PathBuf,
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

    /// Install all packages in the spago config
    pub async fn install_packages(
        &self,
        package_set: &PackageSet,
        config: &SpagoConfig,
    ) -> Result<InstallResult> {
        // Ensure .spago directory exists
        fs::create_dir_all(&self.spago_dir).context("Failed to create .spago directory")?;

        let query = PackageQuery::new(package_set);
        let mut all_packages = HashSet::new();
        let mut processed = HashSet::new();

        let direct_package_dependencies: Vec<PackageName> = if config.is_workspace_root() {
            query.all_workspace_dependencies()
        } else {
            config.package_dependencies().into_iter().cloned().collect()
        };

        // Collect all packages to install (including dependencies)
        for package_name in direct_package_dependencies {
            self.collect_dependencies_recursive(
                &package_name,
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
        let mut errors = Vec::new();

        for task in tasks {
            match task.await? {
                Ok(package_info) => match package_info {
                    Some(git_installed @ InstalledPackage::Git(_)) => {
                        installed.push(git_installed);
                    }
                    Some(local_installed @ InstalledPackage::Local(_)) => {
                        installed.push(local_installed);
                    }
                    None => {}
                },
                Err(e) => {
                    errors.push(e.to_string());
                }
            }
        }

        if !errors.is_empty() {
            return Err(anyhow::anyhow!(
                "Failed to install dependencies: {}",
                errors.join(", ")
            ));
        }

        Ok(InstallResult { installed, errors })
    }

    /// Collect all dependencies recursively
    pub fn collect_dependencies_recursive(
        &self,
        package_name: &PackageName,
        query: &PackageQuery,
        all_packages: &mut HashSet<PackageName>,
        processed: &mut HashSet<PackageName>,
    ) -> Result<()> {
        if processed.contains(package_name) {
            return Ok(());
        }

        processed.insert(package_name.clone());

        // Get package info from package set
        let package = query.get(package_name).ok_or_else(|| {
            anyhow::anyhow!("Package '{}' not found in package set", package_name.0)
        })?;

        // Add dependencies first
        for dep_name in package.dependencies().iter() {
            self.collect_dependencies_recursive(dep_name, query, all_packages, processed)?;
        }

        // Add this package
        all_packages.insert(package_name.clone());
        Ok(())
    }

    /// Install a single package (used by parallel tasks)
    async fn install_single_package(
        package_name: &PackageName,
        package_set: &PackageSet,
        spago_dir: &Path,
        global_cache: &GlobalPackageCache,
    ) -> Result<Option<InstalledPackage>> {
        let package = package_set.get(package_name).ok_or_else(|| {
            anyhow::anyhow!("Package '{}' not found in package set", package_name.0)
        })?;

        match package {
            Package::Local(package) => Ok(None),
            Package::Remote(package) => {
                let folder_name = &package.name.0;
                let package_dir = spago_dir.join(&folder_name);

                // Check if already installed
                if package_dir.exists() {
                    return Ok(None); // Already installed
                }

                // Check global cache first
                if global_cache.is_cached(&package.name, &package.version)? {
                    // Copy from cache
                    global_cache.copy_from_cache(&package.name, &package.version, &package_dir)?;
                    return Ok(Some(InstalledPackage::Git(PackageInfo {
                        name: package.name.clone(),
                        version: package.version.clone(),
                        repo_url: package.repo.clone(),
                        local_path: package_dir,
                    })));
                }

                // Fetch from Git and cache
                let package_info = fetch_package(package, spago_dir)?;

                // Cache the package for future use
                global_cache.cache_package(
                    &package_info.name,
                    &package_info.version,
                    &package_info.local_path,
                )?;

                Ok(Some(InstalledPackage::Git(package_info)))
            }
        }
    }
}
