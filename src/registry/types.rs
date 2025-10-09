use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

/// The complete package set - a map of package names to package information
pub type PackageSet = HashMap<PackageName, Package>;

/// A package name, shared between all package types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[serde(transparent)]
pub struct PackageName(pub String);

impl PackageName {
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

/// A remote package from your package set
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageInSet {
    pub dependencies: Vec<PackageName>,
    pub repo: String,
    pub version: String,
}

/// A remote package from your package set
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageSetPackage {
    pub name: PackageName,
    pub dependencies: Vec<PackageName>,
    pub repo: String,
    pub version: String,
}

/// A local dependency in your filesystem
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalPackage {
    pub name: PackageName,
    pub dependencies: Vec<PackageName>,
    pub path: PathBuf,
}

/// A registry package
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryPackage {
    pub name: PackageName,
    pub version: String,
    pub dependencies: Vec<PackageName>,
}

/// Registry index containing all packages and their versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryIndex(
    pub std::collections::HashMap<PackageName, std::collections::HashMap<String, RegistryPackage>>,
);

/// A package in the package set
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Package {
    /// A remote package set package
    Remote(PackageSetPackage),
    /// A local filesystem package
    Local(LocalPackage),
    /// A registry package
    Registry(RegistryPackage),
}

impl Package {
    pub fn new(package: PackageSetPackage) -> Self {
        Package::Remote(package)
    }

    pub fn new_local(
        name: PackageName,
        path: PathBuf,
        dependencies: Option<Vec<PackageName>>,
    ) -> Self {
        Package::Local(LocalPackage {
            name,
            path,
            dependencies: dependencies.unwrap_or_default(),
        })
    }

    /// Get the number of direct dependencies
    pub fn dep_count(&self) -> usize {
        self.dependencies().len()
    }

    /// Check if this package depends on another package
    pub fn depends_on(&self, pkg_name: &PackageName) -> bool {
        self.dependencies().iter().any(|d| d == pkg_name)
    }

    /// Get the package name from its repository URL
    pub fn name(&self) -> &PackageName {
        match self {
            Package::Remote(package) => &package.name,
            Package::Local(package) => &package.name,
            Package::Registry(package) => &package.name,
        }
    }

    pub fn version(&self) -> Option<&String> {
        match self {
            Package::Remote(package) => Some(&package.version),
            Package::Local(_) => None,
            Package::Registry(package) => Some(&package.version),
        }
    }

    /// Get the full list of dependencies
    pub fn dependencies(&self) -> &Vec<PackageName> {
        match self {
            Package::Remote(package) => &package.dependencies,
            Package::Local(package) => &package.dependencies,
            Package::Registry(package) => &package.dependencies,
        }
    }
}
