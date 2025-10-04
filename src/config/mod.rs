mod types;
mod update;
mod validation;

pub use types::{
    ExtraPackage, ExtraPackageConfig, PackageConfig, SpagoConfig, TestConfig, WorkspaceConfig,
};
pub use update::{add_packages_to_config, remove_packages_from_config};
pub use validation::{
    validate_config, validate_transitive_deps, ValidationError, ValidationResult,
};

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Load and parse a spago.yaml file
pub fn load_config(path: impl AsRef<Path>) -> Result<SpagoConfig> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path)
        .context(format!("Failed to read config file: {}", path.display()))?;

    let config: SpagoConfig =
        serde_yaml::from_str(&contents).context("Failed to parse spago.yaml")?;

    Ok(config)
}

/// Load config from the current directory
pub fn load_config_cwd() -> Result<SpagoConfig> {
    load_config("spago.yaml")
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
