use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a package in the package set
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Package {
    pub dependencies: Vec<String>,
    pub repo: String,
    pub version: String,
}

impl Package {
    /// Get the package name from its repository URL
    pub fn name_from_repo(&self) -> Option<String> {
        self.repo.split('/').last().map(|s| s.to_string())
    }
}

/// The complete package set - a map of package names to package information
pub type PackageSet = HashMap<String, Package>;

/// Information about a package with its dependencies resolved
#[derive(Debug, Clone)]
pub struct PackageInfo<'a> {
    pub name: String,
    pub package: &'a Package,
}

impl<'a> PackageInfo<'a> {
    pub fn new(name: impl Into<String>, package: &'a Package) -> Self {
        Self {
            name: name.into(),
            package,
        }
    }

    /// Get the number of direct dependencies
    pub fn dep_count(&self) -> usize {
        self.package.dependencies.len()
    }

    /// Check if this package depends on another package
    pub fn depends_on(&self, pkg_name: &str) -> bool {
        self.package.dependencies.iter().any(|d| d == pkg_name)
    }
}
