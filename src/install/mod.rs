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

/// Install all dependencies from spago.yaml
pub async fn install_all_dependencies(
    config: &crate::config::SpagoConfig,
    package_set: &crate::registry::PackageSet,
    spago_dir: &Path,
) -> Result<InstallResult> {
    let manager = InstallManager::new(spago_dir)?;
    manager.install_packages(package_set, config).await
}
