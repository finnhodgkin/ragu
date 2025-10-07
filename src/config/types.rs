use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::registry::{PackageName, PackageSet};

/// Complete spago.yaml configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpagoConfig {
    pub package: PackageConfig,
    #[serde(default)]
    pub workspace: WorkspaceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JustPackageConfig {
    pub package: PackageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JustWorkspaceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceConfig>,
}

/// Package configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    pub name: PackageName,
    #[serde(default)]
    pub dependencies: Vec<PackageName>,
    #[serde(default)]
    pub test: Option<TestConfig>,
}

/// Test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub main: PackageName,
    #[serde(default)]
    pub dependencies: Vec<PackageName>,
}

/// Workspace configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceConfig {
    #[serde(default)]
    pub is_nested: bool,
    #[serde(default)]
    pub package_set: Option<PackageSetConfig>,
    #[serde(default)]
    pub extra_packages: HashMap<PackageName, ExtraPackageConfig>,
}

/// Detailed extra package configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraPackageConfig {
    /// Git repository URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<String>,
    /// Git reference (branch, tag, or commit hash)
    #[serde(skip_serializing_if = "Option::is_none", rename = "ref")]
    pub ref_: Option<String>,
    /// Local path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Package dependencies (if not in spago.yaml)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<String>>,
}

/// Package set configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSetConfig {
    pub url: String,
}

impl SpagoConfig {
    /// Get all dependencies (package + test)
    pub fn all_dependencies(&self) -> Vec<&PackageName> {
        let mut deps = self.package_dependencies();

        deps.extend(self.test_dependencies());

        // Remove duplicates
        deps.sort_unstable();
        deps.dedup();
        deps
    }

    /// Get only package dependencies (excluding test)
    pub fn package_dependencies(&self) -> Vec<&PackageName> {
        self.package.dependencies.iter().collect()
    }

    /// Get only test dependencies
    pub fn test_dependencies(&self) -> Vec<&PackageName> {
        self.package
            .test
            .as_ref()
            .map(|t| t.dependencies.iter().collect())
            .unwrap_or_default()
    }

    /// Get the package set URL if configured
    pub fn package_set_url(&self) -> Option<&str> {
        self.workspace
            .package_set
            .as_ref()
            .map(|ps| ps.url.as_str())
    }

    pub fn package_set(&self) -> Result<PackageSet> {
        let package_set_url = self
            .package_set_url()
            .context("Package set URL not found in spago.yaml")?;
        let package_set_tag = crate::config::extract_tag_from_url(package_set_url)
            .context("Failed to extract tag from package set URL")?;

        crate::registry::get_package_set(&package_set_tag, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> SpagoConfig {
        SpagoConfig {
            package: PackageConfig {
                name: PackageName::new("test-package"),
                dependencies: vec![PackageName::new("prelude"), PackageName::new("effect")],
                test: Some(TestConfig {
                    main: PackageName::new("Test.Main"),
                    dependencies: vec![PackageName::new("console"), PackageName::new("effect")],
                }),
            },
            workspace: WorkspaceConfig::default(),
        }
    }

    #[test]
    fn test_all_dependencies() {
        let config = create_test_config();
        let deps = config.all_dependencies();

        // Should have console, effect, prelude (effect appears in both but deduplicated)
        assert_eq!(deps.len(), 3);
        assert!(deps.contains(&&PackageName::new("prelude")));
        assert!(deps.contains(&&PackageName::new("effect")));
        assert!(deps.contains(&&PackageName::new("console")));
    }

    #[test]
    fn test_package_dependencies() {
        let config = create_test_config();
        let deps = config.package_dependencies();

        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&&PackageName::new("prelude")));
        assert!(deps.contains(&&PackageName::new("effect")));
    }

    #[test]
    fn test_test_dependencies() {
        let config = create_test_config();
        let deps = config.test_dependencies();

        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&&PackageName::new("console")));
        assert!(deps.contains(&&PackageName::new("effect")));
    }
}
