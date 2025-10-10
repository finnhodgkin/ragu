pub mod cache;
pub mod cleanup;
mod git;
mod manager;

pub use cleanup::cleanup_unused_packages;
pub use manager::{InstallManager, InstallResult};

use anyhow::Result;

/// Install all dependencies from spago.yaml
pub async fn install_all_dependencies(
    config: &crate::config::SpagoConfig,
    package_set: &crate::registry::PackageSet,
) -> Result<InstallResult> {
    let manager = InstallManager::new(&config.spago_dir())?;
    manager.install_packages(package_set, config).await
}
