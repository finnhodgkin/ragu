use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet, VecDeque};

use super::types::{Package, PackageInfo, PackageSet};

/// Fast package query interface for looking up packages in a package set
pub struct PackageQuery<'a> {
    package_set: &'a PackageSet,
}

impl<'a> PackageQuery<'a> {
    /// Create a new package query interface for a package set
    pub fn new(package_set: &'a PackageSet) -> Self {
        Self { package_set }
    }

    /// Get a specific package by name
    pub fn get(&self, name: &str) -> Option<PackageInfo<'a>> {
        self.package_set
            .get(name)
            .map(|pkg| PackageInfo::new(name, pkg))
    }

    /// Check if a package exists in the set
    pub fn exists(&self, name: &str) -> bool {
        self.package_set.contains_key(name)
    }

    /// Get multiple packages by name
    pub fn get_many(&self, names: &[&str]) -> Vec<Option<PackageInfo<'a>>> {
        names.iter().map(|name| self.get(name)).collect()
    }

    /// Get all packages that match a predicate
    pub fn filter<F>(&self, predicate: F) -> Vec<PackageInfo<'a>>
    where
        F: Fn(&str, &Package) -> bool,
    {
        self.package_set
            .iter()
            .filter(|(name, pkg)| predicate(name, pkg))
            .map(|(name, pkg)| PackageInfo::new(name, pkg))
            .collect()
    }

    /// Get all direct dependencies of a package
    pub fn get_dependencies(&self, name: &str) -> Result<Vec<PackageInfo<'a>>> {
        let package = self
            .get(name)
            .context(format!("Package '{}' not found in package set", name))?;

        let deps: Vec<PackageInfo<'a>> = package
            .package
            .dependencies
            .iter()
            .filter_map(|dep_name| self.get(dep_name))
            .collect();

        Ok(deps)
    }

    /// Get all transitive dependencies of a package (BFS traversal)
    /// Returns a vector of packages in dependency order
    pub fn get_transitive_dependencies(&self, name: &str) -> Result<Vec<PackageInfo<'a>>> {
        let _root = self
            .get(name)
            .context(format!("Package '{}' not found in package set", name))?;

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Start with the root package's dependencies
        queue.push_back(name.to_string());
        visited.insert(name.to_string());

        while let Some(current_name) = queue.pop_front() {
            if let Some(pkg_info) = self.get(&current_name) {
                // Add this package to results (skip the root)
                if current_name != name {
                    result.push(pkg_info.clone());
                }

                // Queue up dependencies
                for dep_name in &pkg_info.package.dependencies {
                    if !visited.contains(dep_name) {
                        visited.insert(dep_name.clone());
                        queue.push_back(dep_name.clone());
                    }
                }
            }
        }

        Ok(result)
    }

    /// Get packages that depend on a specific package (reverse dependencies)
    pub fn get_dependents(&self, name: &str) -> Vec<PackageInfo<'a>> {
        self.filter(|_pkg_name, pkg| pkg.dependencies.iter().any(|dep| dep == name))
    }

    /// Find packages by partial name match
    pub fn search(&self, query: &str) -> Vec<PackageInfo<'a>> {
        let query_lower = query.to_lowercase();
        self.filter(|name, _pkg| name.to_lowercase().contains(&query_lower))
    }

    /// Get package statistics
    pub fn stats(&self) -> PackageSetStats {
        let total_packages = self.package_set.len();
        let mut total_dependencies = 0;
        let mut max_deps = 0;
        let mut min_deps = usize::MAX;
        let mut no_deps_count = 0;

        for pkg in self.package_set.values() {
            let dep_count = pkg.dependencies.len();
            total_dependencies += dep_count;

            if dep_count > max_deps {
                max_deps = dep_count;
            }
            if dep_count < min_deps {
                min_deps = dep_count;
            }
            if dep_count == 0 {
                no_deps_count += 1;
            }
        }

        let avg_deps = if total_packages > 0 {
            total_dependencies as f64 / total_packages as f64
        } else {
            0.0
        };

        PackageSetStats {
            total_packages,
            total_dependencies,
            avg_dependencies: avg_deps,
            max_dependencies: max_deps,
            min_dependencies: if min_deps == usize::MAX { 0 } else { min_deps },
            packages_with_no_deps: no_deps_count,
        }
    }

    /// Validate that all dependencies in the package set actually exist
    pub fn validate(&self) -> ValidationResult {
        let mut missing_deps: HashMap<String, Vec<String>> = HashMap::new();

        for (pkg_name, pkg) in self.package_set.iter() {
            for dep_name in &pkg.dependencies {
                if !self.exists(dep_name) {
                    missing_deps
                        .entry(pkg_name.clone())
                        .or_insert_with(Vec::new)
                        .push(dep_name.clone());
                }
            }
        }

        ValidationResult {
            is_valid: missing_deps.is_empty(),
            missing_dependencies: missing_deps,
        }
    }
}

/// Statistics about a package set
#[derive(Debug, Clone)]
pub struct PackageSetStats {
    pub total_packages: usize,
    pub total_dependencies: usize,
    pub avg_dependencies: f64,
    pub max_dependencies: usize,
    pub min_dependencies: usize,
    pub packages_with_no_deps: usize,
}

/// Result of package set validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    /// Map of package name to list of missing dependencies
    pub missing_dependencies: HashMap<String, Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::types::Package;

    fn create_test_package_set() -> PackageSet {
        let mut set = HashMap::new();

        set.insert(
            "prelude".to_string(),
            Package {
                dependencies: vec![],
                repo: "https://github.com/purescript/purescript-prelude".to_string(),
                version: "v6.0.0".to_string(),
            },
        );

        set.insert(
            "effect".to_string(),
            Package {
                dependencies: vec!["prelude".to_string()],
                repo: "https://github.com/purescript/purescript-effect".to_string(),
                version: "v4.0.0".to_string(),
            },
        );

        set.insert(
            "console".to_string(),
            Package {
                dependencies: vec!["effect".to_string(), "prelude".to_string()],
                repo: "https://github.com/purescript/purescript-console".to_string(),
                version: "v6.0.0".to_string(),
            },
        );

        set
    }

    #[test]
    fn test_get_package() {
        let set = create_test_package_set();
        let query = PackageQuery::new(&set);

        let pkg = query.get("prelude");
        assert!(pkg.is_some());
        assert_eq!(pkg.unwrap().package.version, "v6.0.0");

        let missing = query.get("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_exists() {
        let set = create_test_package_set();
        let query = PackageQuery::new(&set);

        assert!(query.exists("prelude"));
        assert!(query.exists("effect"));
        assert!(!query.exists("nonexistent"));
    }

    #[test]
    fn test_get_dependencies() {
        let set = create_test_package_set();
        let query = PackageQuery::new(&set);

        let deps = query.get_dependencies("console").unwrap();
        assert_eq!(deps.len(), 2);

        let prelude_deps = query.get_dependencies("prelude").unwrap();
        assert_eq!(prelude_deps.len(), 0);
    }

    #[test]
    fn test_transitive_dependencies() {
        let set = create_test_package_set();
        let query = PackageQuery::new(&set);

        let trans_deps = query.get_transitive_dependencies("console").unwrap();
        // Should include effect and prelude
        assert!(trans_deps.len() >= 2);
    }

    #[test]
    fn test_search() {
        let set = create_test_package_set();
        let query = PackageQuery::new(&set);

        let results = query.search("eff");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "effect");

        let results = query.search("e");
        assert!(results.len() >= 2); // effect and prelude
    }
}
