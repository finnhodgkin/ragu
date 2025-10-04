pub mod cache;
mod git;
mod manager;

pub use cache::GlobalPackageCache;
pub use git::{fetch_package, PackageInfo as GitPackageInfo};
pub use manager::{InstallManager, InstallResult};

use anyhow::Result;
use std::path::Path;

/// Install packages from the package set
pub async fn install_packages(
    package_names: &[String],
    package_set: &crate::registry::PackageSet,
    spago_dir: &Path,
) -> Result<InstallResult> {
    let manager = InstallManager::new(spago_dir)?;
    manager.install_packages(package_names, package_set).await
}

/// Install all dependencies from spago.yaml
pub async fn install_all_dependencies(
    config: &crate::config::SpagoConfig,
    package_set: &crate::registry::PackageSet,
    spago_dir: &Path,
) -> Result<InstallResult> {
    let manager = InstallManager::new(spago_dir)?;
    let all_deps: Vec<String> = config
        .all_dependencies()
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    manager.install_packages(&all_deps, package_set).await
}
