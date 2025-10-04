// Module structure for the package registry
mod cache;
mod package_sets;
mod packages;
mod types;

// Re-export public API
pub use cache::{clear_cache, clear_cache_for_tag, clear_tags_cache, get_cache_dir};
pub use package_sets::{
    get_latest_tag, get_package_set, list_available_tags, list_available_tags_with_options,
};
pub use packages::{PackageQuery, PackageSetStats, ValidationResult};
pub use types::{Package, PackageInfo, PackageSet};
