pub mod cache;
pub mod cleanup;
pub mod extra;
mod git;
mod manager;

pub use cache::GlobalPackageCache;
pub use cleanup::cleanup_unused_packages;
pub use extra::install_extra_package;
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

/// Install packages with extra packages configuration
pub async fn install_packages_with_config(
    package_names: &[String],
    package_set: &crate::registry::PackageSet,
    spago_dir: &Path,
    config: Option<&crate::config::SpagoConfig>,
) -> Result<InstallResult> {
    let manager = InstallManager::new(spago_dir)?;
    manager
        .install_packages_with_config(package_names, package_set, config)
        .await
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

/// Install all dependencies from spago.yaml with extra packages support
pub async fn install_all_dependencies_with_config(
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
    manager
        .install_packages_with_config(&all_deps, package_set, Some(config))
        .await
}
