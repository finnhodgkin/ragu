mod types;
mod update;
mod validation;

pub use types::{ExtraPackageConfig, PackageConfig, SpagoConfig, WorkspaceConfig};
pub use update::{add_packages_to_config, remove_packages_from_config};
pub use validation::{validate_config, validate_transitive_deps};

use anyhow::{Context, Result};
use std::path::Path;
use std::{fs, path::PathBuf};

use crate::config::types::{JustPackageConfig, JustWorkspaceConfig};
use crate::registry::PackageName;

/// Load and parse a spago.yaml file
pub fn load_config(path: impl AsRef<Path>, ignore_when_workspace: bool) -> Result<SpagoConfig> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path)
        .context(format!("Failed to read config file: {}", path.display()))?;

    let package_config: PackageConfig = serde_yaml::from_str::<JustPackageConfig>(&contents)
        .unwrap_or(JustPackageConfig {
            package: PackageConfig {
                name: PackageName::new("workspace_root"),
                dependencies: vec![],
                test: None,
            },
        })
        .package;

    let workspace_config: Option<WorkspaceConfig> =
        serde_yaml::from_str::<JustWorkspaceConfig>(&contents)
            .context("Failed to parse workspace section of spago.yaml")?
            .workspace;

    if ignore_when_workspace {
        return Err(anyhow::anyhow!(
            "Workspace section found in spago.yaml, but ignore_when_workspace is true"
        ));
    }

    let cwd = path.parent().context("Failed to get current directory")?;
    Ok(match workspace_config {
        Some(workspace_config) => SpagoConfig {
            package: package_config,
            workspace: workspace_config,
            workspace_root: if cwd.to_path_buf() == PathBuf::from("") {
                PathBuf::from(".")
            } else {
                cwd.to_path_buf()
            },
        },
        None => {
            let above = cwd.join("..");
            let (workspace_root, workspace) = find_workspace_root(&above)?;
            SpagoConfig {
                workspace,
                package: package_config,
                workspace_root,
            }
        }
    })
}

/// Load config from the current directory
pub fn load_config_cwd() -> Result<SpagoConfig> {
    let config = load_config("spago.yaml", false)?;

    Ok(config)
}

fn find_workspace_root(parent: &Path) -> Result<(PathBuf, WorkspaceConfig)> {
    fs::exists(parent).context("Failed to find workspace config by traversing up.")?;

    let spago_yaml = parent.join("spago.yaml");
    if spago_yaml.exists() {
        let contents = fs::read_to_string(spago_yaml).context("Failed to read spago.yaml")?;
        let workspace_config: Option<WorkspaceConfig> =
            serde_yaml::from_str::<JustWorkspaceConfig>(&contents)
                .context("Failed to parse spago.yaml")?
                .workspace;
        match workspace_config {
            Some(workspace_config) => Ok((parent.to_path_buf(), workspace_config)),
            None => find_workspace_root(&parent.join("..")),
        }
    } else {
        find_workspace_root(&parent.join(".."))
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
