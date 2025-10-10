use std::collections::HashSet;

use crate::config::SpagoConfig;
use crate::registry::{PackageName, PackageQuery};

/// Result of configuration validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

/// Validation error
#[derive(Debug, Clone)]
pub enum ValidationError {
    MissingDependency {
        package: PackageName,
        context: DependencyContext,
    },
    EmptyName,
}

/// Context where a dependency is declared
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyContext {
    Package,
    Test,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: ValidationError) {
        self.is_valid = false;
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate a spago.yaml configuration against a package set
pub fn validate_config(config: &SpagoConfig, query: &PackageQuery) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Validate package name
    if config.package.name.0.is_empty() {
        result.add_error(ValidationError::EmptyName);
    }

    // Validate package dependencies
    for dep in &config.package.dependencies {
        if !query.exists(dep) {
            result.add_error(ValidationError::MissingDependency {
                package: dep.clone(),
                context: DependencyContext::Package,
            });
        }
    }

    // Validate test dependencies
    if let Some(test) = &config.package.test {
        for dep in &test.dependencies {
            if !query.exists(dep) {
                result.add_error(ValidationError::MissingDependency {
                    package: dep.clone(),
                    context: DependencyContext::Test,
                });
            }
        }
    }

    // Check for duplicate dependencies
    let mut seen = HashSet::new();
    for dep in &config.package.dependencies {
        if !seen.insert(dep) {
            result.add_warning(format!("Duplicate dependency in package: {}", dep.0));
        }
    }

    if let Some(test) = &config.package.test {
        let mut seen = HashSet::new();
        for dep in &test.dependencies {
            if !seen.insert(dep) {
                result.add_warning(format!("Duplicate dependency in test: {}", dep.0));
            }
        }

        // Warn about test dependencies already in package dependencies
        for dep in &test.dependencies {
            if config.package.dependencies.contains(dep) {
                result.add_warning(format!(
                    "Test dependency '{}' already in package dependencies (redundant)",
                    dep.0
                ));
            }
        }
    }

    result
}

/// Check if all transitive dependencies are satisfied
pub fn validate_transitive_deps(config: &SpagoConfig, query: &PackageQuery) -> ValidationResult {
    let mut result = ValidationResult::new();

    // For each direct dependency, check its transitive dependencies
    for dep in &config.package.dependencies {
        if let Ok(transitive) = query.get_transitive_dependencies(dep) {
            for trans_dep in transitive {
                if !query.exists(&trans_dep.name()) {
                    result.add_error(ValidationError::MissingDependency {
                        package: trans_dep.name().clone(),
                        context: DependencyContext::Package,
                    });
                }
            }
        }
    }

    result
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::MissingDependency { package, context } => {
                let ctx = match context {
                    DependencyContext::Package => "package dependencies",
                    DependencyContext::Test => "test dependencies",
                };
                write!(
                    f,
                    "Package '{}' not found in package set ({})",
                    package.0, ctx
                )
            }
            ValidationError::EmptyName => {
                write!(f, "Package name cannot be empty")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PackageConfig;
    use crate::registry::{Package, PackageSet, PackageSetPackage};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn create_test_package_set() -> PackageSet {
        let mut set = HashMap::new();

        set.insert(
            PackageName::new("prelude"),
            Package::Remote(PackageSetPackage {
                name: PackageName::new("prelude"),
                dependencies: vec![],
                repo: "https://github.com/purescript/purescript-prelude".to_string(),
                version: "v6.0.0".to_string(),
            }),
        );

        set.insert(
            PackageName::new("effect"),
            Package::Remote(PackageSetPackage {
                name: PackageName::new("effect"),
                dependencies: vec![PackageName::new("prelude")],
                repo: "https://github.com/purescript/purescript-effect".to_string(),
                version: "v4.0.0".to_string(),
            }),
        );

        set
    }

    #[test]
    fn test_valid_config() {
        let config = SpagoConfig {
            package: PackageConfig {
                name: PackageName::new("test"),
                dependencies: vec![PackageName::new("prelude"), PackageName::new("effect")],
                test: None,
            },
            workspace: Default::default(),
            workspace_root: PathBuf::from("."),
        };

        let package_set = create_test_package_set();
        let query = PackageQuery::new(&package_set);
        let result = validate_config(&config, &query);

        assert!(result.is_valid);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_missing_dependency() {
        let config = SpagoConfig {
            package: PackageConfig {
                name: PackageName::new("test"),
                dependencies: vec![PackageName::new("nonexistent")],
                test: None,
            },
            workspace: Default::default(),
            workspace_root: PathBuf::from("."),
        };

        let package_set = create_test_package_set();
        let query = PackageQuery::new(&package_set);
        let result = validate_config(&config, &query);

        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);

        match &result.errors[0] {
            ValidationError::MissingDependency { package, context } => {
                assert_eq!(*package, PackageName::new("nonexistent"));
                assert_eq!(*context, DependencyContext::Package);
            }
            _ => panic!("Expected MissingDependency error"),
        }
    }
}
