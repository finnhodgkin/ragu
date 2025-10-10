use std::path::PathBuf;

use walkdir::WalkDir;

use crate::registry::{LocalPackage, Package, PackageSet};

/// Common directories to skip when searching for workspace packages
const SKIP_DIRS: [&str; 3] = [".spago", "node_modules", "output"];

/// Add local workspace packages to the package set so they can be accessed
/// in the build process from the start
pub fn add_workspace_packages(package_set: &mut PackageSet, workspace_root: &PathBuf) {
    for entry in WalkDir::new(workspace_root)
        .min_depth(1)
        .max_depth(5)
        .into_iter()
        .filter_entry(|e| {
            // Skip hidden directories and our skip list
            let file_name = e.file_name().to_string_lossy();
            !file_name.starts_with('.') && !SKIP_DIRS.iter().any(|dir| file_name == *dir)
        })
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let root_spago_yaml = workspace_root.join("spago.yaml");

        // Look for spago.yaml files
        if entry.file_type().is_file()
            && entry.file_name() == "spago.yaml"
            && entry.path() != root_spago_yaml
        {
            if let Ok(config) = crate::config::load_config(entry.path(), true) {
                let path = entry.path().parent().unwrap().to_path_buf();
                // Add the package to our set
                package_set.insert(
                    config.package.name.clone(),
                    Package::Local(LocalPackage {
                        name: config.package.name,
                        dependencies: config.package.dependencies,
                        test_dependencies: config
                            .package
                            .test
                            .map(|t| t.dependencies)
                            .unwrap_or_default(),
                        path,
                    }),
                );
            }
        }
    }
}
