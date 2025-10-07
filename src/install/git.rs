use anyhow::{Context, Result};
use git2::Repository;
use std::fs;
use std::path::Path;

use crate::registry::{PackageName, PackageSetPackage};

/// Information about a fetched package
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: PackageName,
    pub version: String,
    pub repo_url: String,
    pub local_path: std::path::PathBuf,
}

/// Fetch a package from its Git repository
pub fn fetch_package(package: &PackageSetPackage, spago_dir: &Path) -> Result<PackageInfo> {
    let package_name = package.name.clone();
    let folder_name = package.name.0.clone();
    let package_dir = spago_dir.join(&folder_name);

    // Check if package is already installed with the correct version
    if package_dir.exists() {
        if let Ok(repo) = Repository::open(&package_dir) {
            // Check if we're on the right tag/commit
            if let Ok(reference) = repo.find_reference(&format!("refs/tags/{}", package.version)) {
                if let Some(oid) = reference.target() {
                    if let Ok(head) = repo.head() {
                        if let Some(head_oid) = head.target() {
                            if head_oid == oid {
                                return Ok(PackageInfo {
                                    name: package_name,
                                    version: package.version.clone(),
                                    repo_url: package.repo.clone(),
                                    local_path: package_dir,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Clone or update the repository
    if package_dir.exists() {
        // Update existing repository
        let repo = Repository::open(&package_dir).context("Failed to open existing repository")?;

        // Fetch latest changes
        let mut remote = repo
            .find_remote("origin")
            .context("Failed to find origin remote")?;

        remote
            .fetch(&[] as &[&str], None, None)
            .context("Failed to fetch from remote")?;

        // For now, just use the latest commit
        // TODO: Implement proper version/tag checking
    } else {
        // Clone new repository
        Repository::clone(&package.repo, &package_dir).context("Failed to clone repository")?;
    }

    // Open the repository to get current state
    let repo = Repository::open(&package_dir).context("Failed to open repository")?;

    // Get the current HEAD commit
    let head = repo.head().context("Failed to get HEAD")?;
    let head_oid = head.target().context("Failed to get HEAD target")?;

    // Prune the package to only keep README and src folders
    prune_package(&package_dir)?;

    Ok(PackageInfo {
        name: package_name,
        version: package.version.clone(),
        repo_url: package.repo.clone(),
        local_path: package_dir,
    })
}

/// Prune a package directory to only keep README, spago.yaml and src folders
pub fn prune_package(package_dir: &Path) -> Result<()> {
    let entries = fs::read_dir(package_dir).context("Failed to read package directory")?;

    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let entry_path = entry.path();
        let file_name = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Keep only README, spago.yaml and src directory
        let should_keep = file_name == "src"
            || file_name == "README.md"
            || file_name == "readme.md"
            || file_name == "README"
            || file_name == "readme"
            || file_name == "spago.yaml";

        if !should_keep {
            if entry_path.is_dir() {
                fs::remove_dir_all(&entry_path).context("Failed to remove directory")?;
            } else {
                fs::remove_file(&entry_path).context("Failed to remove file")?;
            }
        }
    }

    Ok(())
}
