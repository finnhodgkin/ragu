mod types;
mod update;
mod validation;

pub use types::{ExtraPackageConfig, PackageConfig, SpagoConfig, TestConfig, WorkspaceConfig};
pub use update::{add_packages_to_config, remove_packages_from_config};
pub use validation::{
    validate_config, validate_transitive_deps, ValidationError, ValidationResult,
};

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::config::types::{JustPackageConfig, JustWorkspaceConfig};

/// Load and parse a spago.yaml file
pub fn load_config(path: impl AsRef<Path>) -> Result<SpagoConfig> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path)
        .context(format!("Failed to read config file: {}", path.display()))?;

    let package_config: PackageConfig = serde_yaml::from_str::<JustPackageConfig>(&contents)
        .context("Failed to parse package section of spago.yaml")?
        .package;

    let workspace_config: Option<WorkspaceConfig> =
        serde_yaml::from_str::<JustWorkspaceConfig>(&contents)
            .context("Failed to parse workspace section of spago.yaml")?
            .workspace;

    Ok(match workspace_config {
        Some(workspace_config) => SpagoConfig {
            package: package_config,
            workspace: workspace_config,
        },
        None => {
            let cwd = path.parent().context("Failed to get current directory")?;
            let above = cwd.join("..");
            SpagoConfig {
                workspace: traverse_up_to_workspace_config(&above)?,
                package: package_config,
            }
        }
    })
}

/// Load config from the current directory
pub fn load_config_cwd() -> Result<SpagoConfig> {
    let config = load_config("spago.yaml")?;

    Ok(config)
}

fn traverse_up_to_workspace_config(parent: &Path) -> Result<WorkspaceConfig> {
    fs::exists(parent).context("Failed to find workspace config by traversing up.")?;

    let spago_yaml = parent.join("spago.yaml");
    if spago_yaml.exists() {
        let contents = fs::read_to_string(spago_yaml).context("Failed to read spago.yaml")?;
        let workspace_config: Option<WorkspaceConfig> =
            serde_yaml::from_str::<JustWorkspaceConfig>(&contents)
                .context("Failed to parse spago.yaml")?
                .workspace;
        match workspace_config {
            Some(workspace_config) => Ok(workspace_config),
            None => traverse_up_to_workspace_config(&parent.join("..")),
        }
    } else {
        traverse_up_to_workspace_config(&parent.join(".."))
    }
}

/// Extract package set tag from URL
/// Example: "https://raw.githubusercontent.com/purescript/package-sets/psc-0.15.15-20251004/packages.json"
/// Returns: "psc-0.15.15-20251004"
pub fn extract_tag_from_url(url: &str) -> Option<String> {
    // URL format: .../package-sets/{tag}/packages.json
    let parts: Vec<&str> = url.split('/').collect();

    // Find "package-sets" and get the next part
    for (i, part) in parts.iter().enumerate() {
        if *part == "package-sets" && i + 1 < parts.len() {
            let tag = parts[i + 1];
            // Make sure it's not "packages.json"
            if tag != "packages.json" {
                return Some(tag.to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tag_from_url() {
        let url = "https://raw.githubusercontent.com/purescript/package-sets/psc-0.15.15-20251004/packages.json";
        assert_eq!(
            extract_tag_from_url(url),
            Some("psc-0.15.15-20251004".to_string())
        );

        let url2 = "https://example.com/package-sets/psc-0.15.14/packages.json";
        assert_eq!(extract_tag_from_url(url2), Some("psc-0.15.14".to_string()));

        let url3 = "https://example.com/other/path";
        assert_eq!(extract_tag_from_url(url3), None);
    }
}
