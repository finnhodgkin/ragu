use anyhow::{Context, Result};
use std::collections::{HashSet, VecDeque};

use crate::config::load_config_cwd;
use crate::registry::{
    types::{Package, PackageName},
    LocalPackage,
};

use super::types::PackageSet;

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
    pub fn get(&self, name: &PackageName) -> Option<&Package> {
        self.package_set.get(name)
    }

    /// Check if a package exists in the set
    pub fn exists(&self, name: &PackageName) -> bool {
        self.package_set.contains_key(name)
    }

    /// Get all packages that match a predicate
    pub fn filter<F>(&self, predicate: F) -> Vec<&Package>
    where
        F: Fn(&Package) -> bool,
    {
        self.package_set
            .iter()
            .filter(|(_, pkg)| predicate(pkg))
            .map(|(_, pkg)| pkg)
            .collect()
    }

    pub fn local_packages(&self) -> Vec<&LocalPackage> {
        let mut result = Vec::new();
        for (_, package) in self.package_set {
            if let Package::Local(package) = package {
                result.push(package);
            }
        }
        result
    }

    pub fn all_workspace_dependencies(&self) -> Vec<PackageName> {
        let mut local: Vec<PackageName> = self
            .local_packages()
            .iter()
            .flat_map(|p| {
                let mut deps = p.dependencies.clone();
                deps.push(p.name.clone());
                deps
            })
            .collect();

        local.sort_unstable();
        local.dedup();

        local
    }

    pub fn all_workspace_test_dependencies(&self) -> Vec<PackageName> {
        let mut result = Vec::new();
        for (_, package) in self.package_set {
            if let Package::Local(package) = package {
                result.extend(package.test_dependencies.clone());
            }
        }
        result
    }

    /// Get all direct dependencies of a package
    pub fn get_dependencies(&self, name: &PackageName) -> Result<Vec<&Package>> {
        let deps = self
            .get(name)
            .context(format!("Package '{}' not found in package set", name.0))?
            .dependencies()
            .iter()
            .filter_map(|dep_name| self.get(dep_name))
            .collect();

        Ok(deps)
    }

    /// Get all transitive dependencies of a package (BFS traversal)
    /// Returns a vector of packages in dependency order
    pub fn get_transitive_dependencies(&self, name: &PackageName) -> Result<Vec<&Package>> {
        let _root = self
            .get(name)
            .context(format!("Package '{}' not found in package set", name.0))?;

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Start with the root package's dependencies
        queue.push_back(name);
        visited.insert(name);
        while let Some(current_name) = queue.pop_front() {
            if let Some(pkg) = self.get(current_name) {
                // Add this package to results (skip the root)
                if current_name != name {
                    result.push(pkg);
                }

                // Queue up dependencies
                for dep in pkg.dependencies() {
                    if !visited.contains(dep) {
                        visited.insert(dep);
                        queue.push_back(dep);
                    }
                }
            }
        }

        Ok(result)
    }

    /// Get packages that depend on a specific package (reverse dependencies)
    pub fn get_dependents(&self, name: &PackageName) -> Vec<&Package> {
        self.filter(|pkg| pkg.dependencies().iter().any(|dep| dep == name))
    }

    /// Get count of packages that depend on a specific package
    pub fn get_dependents_count(&self, name: &PackageName) -> usize {
        self.get_dependents(name).len()
    }

    /// Get all packages with their dependents count
    pub fn get_packages_with_dependents_count(&self) -> Vec<(&Package, usize)> {
        self.package_set
            .iter()
            .map(|(_, pkg)| {
                let dependents_count = self.get_dependents_count(pkg.name());
                (pkg, dependents_count)
            })
            .collect()
    }

    /// Identify circular dependency chains
    pub fn check_circular_dependencies() -> Result<()> {
        let config = load_config_cwd()?;
        let package_set = config.package_set()?;
        let query = PackageQuery::new(&package_set);

        let all_local_packages = query.local_packages();
        let mut found_circular = false;

        for package in all_local_packages {
            if let Some(circular_chain) = query.find_circular_dependency_chain(&package.name) {
                found_circular = true;
                println!("⚠️  Circular dependency detected:");
                println!("   Chain: {}", circular_chain.join(" → "));
                println!("   This creates a loop where packages depend on each other in a cycle.");
            }
        }

        if !found_circular {
            println!("🎉 No circular dependencies found");
        }

        Ok(())
    }

    /// Find a circular dependency chain starting from a package
    fn find_circular_dependency_chain(&self, start_package: &PackageName) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        self.dfs_circular_detection(start_package, &mut visited, &mut path)
    }

    /// DFS-based circular dependency detection
    fn dfs_circular_detection(
        &self,
        current: &PackageName,
        visited: &mut HashSet<PackageName>,
        path: &mut Vec<PackageName>,
    ) -> Option<Vec<String>> {
        // If we've already visited this node in the current path, we found a cycle
        if path.contains(current) {
            // Find where the cycle starts in the path
            let cycle_start = path.iter().position(|p| p == current).unwrap();
            let mut cycle = path[cycle_start..].to_vec();
            cycle.push(current.clone());

            // Convert to string names for display
            return Some(cycle.iter().map(|p| p.0.clone()).collect());
        }

        // If we've already fully explored this node, no cycle through it
        if visited.contains(current) {
            return None;
        }

        // Add current node to path
        path.push(current.clone());

        // Get dependencies of current package
        if let Some(pkg) = self.get(current) {
            for dep in pkg.dependencies() {
                if let Some(cycle) = self.dfs_circular_detection(dep, visited, path) {
                    return Some(cycle);
                }
            }
        }

        // Remove current node from path and mark as visited
        path.pop();
        visited.insert(current.clone());

        None
    }

    /// Find packages by partial name match
    pub fn search(&self, query: &str) -> Vec<&Package> {
        let query_lower = query.to_lowercase();
        self.filter(|pkg| pkg.name().0.to_lowercase().contains(&query_lower))
    }

    /// Get package statistics
    pub fn stats(&self) -> PackageSetStats {
        let total_packages = self.package_set.len();
        let mut total_dependencies = 0;
        let mut max_deps = 0;
        let mut min_deps = usize::MAX;
        let mut no_deps_count = 0;

        for pkg in self.package_set.values() {
            let dep_count = pkg.dependencies().len();
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::registry::types::{Package, PackageSetPackage};

    fn create_test_package_set() -> PackageSet {
        let mut set = HashMap::new();

        set.insert(
            PackageName::new("prelude"),
            Package::new(PackageSetPackage {
                name: PackageName::new("prelude"),
                dependencies: vec![],
                repo: "https://github.com/purescript/purescript-prelude".to_string(),
                version: "v6.0.0".to_string(),
            }),
        );

        set.insert(
            PackageName("effect".to_string()),
            Package::new(PackageSetPackage {
                name: PackageName::new("effect"),
                dependencies: vec![PackageName::new("prelude")],
                repo: "https://github.com/purescript/purescript-effect".to_string(),
                version: "v4.0.0".to_string(),
            }),
        );

        set.insert(
            PackageName("console".to_string()),
            Package::new(PackageSetPackage {
                name: PackageName::new("console"),
                dependencies: vec![PackageName::new("effect"), PackageName::new("prelude")],
                repo: "https://github.com/purescript/purescript-console".to_string(),
                version: "v6.0.0".to_string(),
            }),
        );

        set
    }

    #[test]
    fn test_get_package() {
        let set = create_test_package_set();
        let query = PackageQuery::new(&set);

        let pkg = query.get(&PackageName("prelude".to_string()));
        assert!(pkg.is_some());
        assert_eq!(pkg.unwrap().version(), Some(&"v6.0.0".to_string()));

        let missing = query.get(&PackageName("nonexistent".to_string()));
        assert!(missing.is_none());
    }

    #[test]
    fn test_exists() {
        let set = create_test_package_set();
        let query = PackageQuery::new(&set);

        assert!(query.exists(&PackageName::new("prelude")));
        assert!(query.exists(&PackageName::new("effect")));
        assert!(!query.exists(&PackageName::new("nonexistent")));
    }

    #[test]
    fn test_get_dependencies() {
        let set = create_test_package_set();
        let query = PackageQuery::new(&set);

        let deps = query
            .get_dependencies(&PackageName::new("console"))
            .unwrap();
        assert_eq!(deps.len(), 2);

        let prelude_deps = query
            .get_dependencies(&PackageName::new("prelude"))
            .unwrap();
        assert_eq!(prelude_deps.len(), 0);
    }

    #[test]
    fn test_transitive_dependencies() {
        let set = create_test_package_set();
        let query = PackageQuery::new(&set);

        let trans_deps = query
            .get_transitive_dependencies(&PackageName::new("console"))
            .unwrap();
        // Should include effect and prelude
        assert!(trans_deps.len() >= 2);
    }

    #[test]
    fn test_search() {
        let set = create_test_package_set();
        let query = PackageQuery::new(&set);

        let results = query.search("eff");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name(), &PackageName::new("effect"));

        let results = query.search("e");
        assert!(results.len() >= 2); // effect and prelude
    }
}
