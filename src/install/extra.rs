use anyhow::{Context, Result};
use git2::Repository;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::ExtraPackageConfig;
use crate::registry::PackageName;

/// Install an extra package with detailed configuration
pub fn install_extra_package(
    package_name: &PackageName,
    config: &ExtraPackageConfig,
    spago_dir: &Path,
) -> Result<()> {
    if let Some(git_url) = &config.git {
        install_git_extra_package(package_name, git_url, config, spago_dir)
    } else if let Some(path) = &config.path {
        install_local_extra_package(package_name, path, spago_dir)
    } else {
        anyhow::bail!(
            "Extra package {} has no git URL or path specified",
            package_name.0
        );
    }
}

/// Install an extra package from a Git repository
fn install_git_extra_package(
    package_name: &PackageName,
    git_url: &str,
    config: &ExtraPackageConfig,
    spago_dir: &Path,
) -> Result<()> {
    let package_dir = spago_dir.join(package_name.0.clone());

    // Git packages require a ref
    let ref_name = config.ref_.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Git package '{}' requires a 'ref' field", package_name.0)
    })?;

    // Check if package is already installed
    if package_dir.exists() {
        // Check if it's a valid git repository
        if Repository::open(&package_dir).is_ok() {
            // Package is already installed, skip
            return Ok(());
        } else {
            // Remove invalid directory
            std::fs::remove_dir_all(&package_dir)
                .context("Failed to remove invalid package directory")?;
        }
    }

    // Clone the repository
    Repository::clone(git_url, &package_dir).context("Failed to clone repository")?;

    // Checkout the specific reference
    let repo = Repository::open(&package_dir).context("Failed to open repository")?;

    let (object, _) = repo
        .revparse_ext(ref_name)
        .context(format!("Failed to find reference: {}", ref_name))?;

    repo.checkout_tree(&object, None)
        .context("Failed to checkout reference")?;

    // Prune the package (same as regular packages)
    crate::install::git::prune_package(&package_dir)?;

    Ok(())
}

/// Install an extra package from a local path
fn install_local_extra_package(
    package_name: &PackageName,
    local_path: &str,
    spago_dir: &Path,
) -> Result<()> {
    let source_path = PathBuf::from(local_path);
    let dest_path = spago_dir.join(package_name.0.clone());

    if !source_path.exists() {
        anyhow::bail!("Local path does not exist: {}", local_path);
    }

    if !source_path.is_dir() {
        anyhow::bail!("Local path is not a directory: {}", local_path);
    }

    // Remove existing package if it exists
    if dest_path.exists() {
        fs::remove_dir_all(&dest_path).context("Failed to remove existing package")?;
    }

    // Copy the package
    copy_dir_all(&source_path, &dest_path).context("Failed to copy local package")?;

    // Prune the package (same as regular packages)
    crate::install::git::prune_package(&dest_path)?;

    Ok(())
}
/// Copy only the src directory and readme from a package
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    // Copy src directory if it exists
    let src_dir = src.join("src");
    if src_dir.exists() {
        let dst_src = dst.join("src");
        if !dst_src.exists() {
            fs::create_dir_all(&dst_src)?;
        }

        for entry in fs::read_dir(&src_dir)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst_src.join(entry.file_name());

            if src_path.is_dir() {
                copy_dir_all(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }
    }

    // Copy readme if it exists
    for readme_name in &["README.md", "README", "Readme.md", "readme.md"] {
        let readme = src.join(readme_name);
        if readme.exists() {
            fs::copy(&readme, dst.join(readme_name))?;
            break;
        }
    }

    Ok(())
}
