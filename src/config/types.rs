use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete spago.yaml configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpagoConfig {
    pub package: PackageConfig,
    #[serde(default)]
    pub workspace: WorkspaceConfig,
}

/// Package configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    pub name: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub test: Option<TestConfig>,
}

/// Test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub main: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// Workspace configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceConfig {
    #[serde(default)]
    pub package_set: Option<PackageSetConfig>,
    #[serde(default)]
    pub extra_packages: HashMap<String, String>,
}

/// Package set configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSetConfig {
    pub url: String,
}

impl SpagoConfig {
    /// Get all dependencies (package + test)
    pub fn all_dependencies(&self) -> Vec<&str> {
        let mut deps: Vec<&str> = self
            .package
            .dependencies
            .iter()
            .map(|s| s.as_str())
            .collect();

        if let Some(test) = &self.package.test {
            deps.extend(test.dependencies.iter().map(|s| s.as_str()));
        }

        // Remove duplicates
        deps.sort_unstable();
        deps.dedup();
        deps
    }

    /// Get only package dependencies (excluding test)
    pub fn package_dependencies(&self) -> Vec<&str> {
        self.package
            .dependencies
            .iter()
            .map(|s| s.as_str())
            .collect()
    }

    /// Get only test dependencies
    pub fn test_dependencies(&self) -> Vec<&str> {
        self.package
            .test
            .as_ref()
            .map(|t| t.dependencies.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Get the package set URL if configured
    pub fn package_set_url(&self) -> Option<&str> {
        self.workspace
            .package_set
            .as_ref()
            .map(|ps| ps.url.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> SpagoConfig {
        SpagoConfig {
            package: PackageConfig {
                name: "test-package".to_string(),
                dependencies: vec!["prelude".to_string(), "effect".to_string()],
                test: Some(TestConfig {
                    main: "Test.Main".to_string(),
                    dependencies: vec!["console".to_string(), "effect".to_string()],
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
        assert!(deps.contains(&"prelude"));
        assert!(deps.contains(&"effect"));
        assert!(deps.contains(&"console"));
    }

    #[test]
    fn test_package_dependencies() {
        let config = create_test_config();
        let deps = config.package_dependencies();

        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"prelude"));
        assert!(deps.contains(&"effect"));
    }

    #[test]
    fn test_test_dependencies() {
        let config = create_test_config();
        let deps = config.test_dependencies();

        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"console"));
        assert!(deps.contains(&"effect"));
    }
}
