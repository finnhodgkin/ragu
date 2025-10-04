use anyhow::{Context, Result};
use git2::{Repository, RepositoryInitOptions};
use std::path::Path;

use crate::registry::Package;

/// Information about a fetched package
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub repo_url: String,
    pub local_path: std::path::PathBuf,
}

/// Extract package name from repository URL (remove .git suffix and purescript- prefix)
pub fn extract_package_name(repo_url: &str) -> String {
    let name = repo_url.split('/').last().unwrap_or("unknown");
    let name = if name.ends_with(".git") {
        &name[..name.len() - 4]
    } else {
        name
    };

    // Remove purescript- prefix if present
    if name.starts_with("purescript-") {
        &name[11..] // Remove "purescript-" (11 characters)
    } else {
        name
    }
    .to_string()
}

/// Fetch a package from its Git repository
pub fn fetch_package(package: &Package, spago_dir: &Path) -> Result<PackageInfo> {
    let package_name = extract_package_name(&package.repo);
    let folder_name = format!("{}-{}", package_name, package.version);
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
                                    name: package_dir
                                        .file_name()
                                        .unwrap()
                                        .to_string_lossy()
                                        .to_string(),
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

    Ok(PackageInfo {
        name: package_name,
        version: package.version.clone(),
        repo_url: package.repo.clone(),
        local_path: package_dir,
    })
}
