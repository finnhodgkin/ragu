use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::registry::{PackageName, PackageSetPackage};

/// Information about a fetched package
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: PackageName,
    pub version: String,
    pub local_path: std::path::PathBuf,
}

/// Fetch a package from its Git repository
pub fn fetch_package(package: &PackageSetPackage, spago_dir: &Path) -> Result<PackageInfo> {
    let package_name = package.name.clone();
    let folder_name = package.name.0.clone();
    let package_dir = spago_dir.join(&folder_name);

    // Clone the repository and checkout the specific tag
    let callbacks = git2::RemoteCallbacks::new();
    let mut fetch_options = git2::FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch_options);

    // Clone the repository first
    let repo = match builder.clone(&package.repo, &package_dir) {
        Ok(repo) => repo,
        Err(e) => {
            // Clean up any partial clone if it exists
            if package_dir.exists() {
                let _ = fs::remove_dir_all(&package_dir);
            }
            return Err(e).context(format!("Failed to clone repository for {}", package_name.0));
        }
    };

    // Try to checkout the reference (could be tag, branch, or commit)
    let (_object, _reference) = repo
        .revparse_ext(&package.version)
        .and_then(|result| {
            // If revparse succeeds, try to checkout the files
            repo.checkout_tree(&result.0, None).map(|_| result)
        })
        .map_err(|e| {
            // Clean up the directory if any step fails to prevent security risk
            if package_dir.exists() {
                let _ = fs::remove_dir_all(&package_dir);
            }
            e
        })
        .context(format!(
            "Failed to parse and checkout reference '{}' for {}",
            package.version, package_name.0
        ))?;

    // Prune the package to only keep README and src folders
    prune_package(&package_dir)?;

    Ok(PackageInfo {
        name: package_name,
        version: package.version.clone(),
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
