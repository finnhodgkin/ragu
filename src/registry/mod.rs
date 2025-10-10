// Module structure for the package registry
mod cache;
mod package_sets;
mod packages;
mod purescript_registry;
mod types;
mod workspace_packages;

// Re-export public API
pub use cache::{
    clear_cache, clear_cache_for_tag, clear_registry_package_set_cache, get_cache_dir,
    load_registry_index_from_cache, load_registry_package_set_from_cache,
    save_registry_index_to_cache, save_registry_package_set_to_cache,
};
pub use package_sets::{
    get_package_set, list_available_registry_versions_with_options,
    list_available_tags_with_options,
};
pub use packages::PackageQuery;
pub use purescript_registry::get_package_set_by_registry_version;
pub use types::{
    LocalPackage, Package, PackageName, PackageSet, PackageSetPackage, RegistryPackage,
};
pub use workspace_packages::add_workspace_packages;
