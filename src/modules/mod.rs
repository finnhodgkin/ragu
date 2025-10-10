mod service;

use anyhow::{Context, Result};
use glob::glob;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::sources::BuildSources;

pub use service::{execute_modules_command, ModulesOptions};

/// Information about a discovered module
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// The module name (e.g., "Data.Maybe")
    pub name: String,
    /// The file path where this module is defined
    #[allow(dead_code)]
    pub file_path: PathBuf,
    /// The package this module belongs to
    pub package_name: String,
}

/// Discover all modules in the project and its dependencies
pub fn discover_all_modules(sources: &BuildSources) -> Result<Vec<ModuleInfo>> {
    let mut modules = Vec::new();

    // Discover modules from main sources
    let main_modules = discover_modules_from_glob(&sources.main_sources, "main")?;
    modules.extend(main_modules);

    // Discover modules from dependency sources
    for dependency_glob in &sources.dependency_globs {
        let dependency_modules = discover_modules_from_glob(
            &dependency_glob.glob_pattern,
            &dependency_glob.package_name,
        )?;
        modules.extend(dependency_modules);
    }

    Ok(modules)
}

/// Discover modules from a glob pattern
fn discover_modules_from_glob(glob_pattern: &str, package_name: &str) -> Result<Vec<ModuleInfo>> {
    let mut modules = Vec::new();

    // Use glob to find all matching files
    for entry in glob(glob_pattern).context("Failed to read glob pattern")? {
        let path = entry.context("Failed to read glob entry")?;

        // Only process .purs files
        if path.extension().and_then(|s| s.to_str()) == Some("purs") {
            if let Ok(module_name) = extract_module_name_from_file(&path) {
                modules.push(ModuleInfo {
                    name: module_name,
                    file_path: path,
                    package_name: package_name.to_string(),
                });
            }
        }
    }

    Ok(modules)
}

/// Extract module name from a PureScript file
pub fn extract_module_name_from_file(file_path: &Path) -> Result<String> {
    let content = fs::read_to_string(file_path).context("Failed to read file")?;
    extract_module_name_from_content(&content)
}

/// Extract module name from PureScript file content
fn extract_module_name_from_content(content: &str) -> Result<String> {
    let mut is_comment_block = false;
    // Look for module declaration at the beginning of the file
    for line in content.lines() {
        let line = line.trim();

        if line.starts_with("{-") {
            is_comment_block = true;
        }

        if line.starts_with("-}") {
            is_comment_block = false;
            continue;
        }

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with("--") || is_comment_block {
            continue;
        }

        // Look for module declaration
        if line.starts_with("module ") {
            // Extract module name from "module Module.Name where"
            let module_part = line
                .strip_prefix("module ")
                .context("Failed to parse module declaration")?;

            // Find the end of the module name (before any exports or "where")
            let module_name = if let Some(export_pos) = module_part.find("(") {
                &module_part[..export_pos].trim()
            } else if let Some(where_pos) = module_part.find(" where") {
                &module_part[..where_pos].trim()
            } else {
                module_part.trim()
            };

            return Ok(module_name.to_string());
        }

        // If we hit any non-comment, non-empty line that's not a module declaration,
        // we're probably past the module declaration
        if !line.starts_with("--") && !line.is_empty() {
            break;
        }
    }

    Err(anyhow::anyhow!("No module declaration found"))
}

/// Group modules by package
pub fn group_modules_by_package(modules: &[ModuleInfo]) -> HashMap<String, Vec<ModuleInfo>> {
    let mut grouped: HashMap<String, Vec<ModuleInfo>> = HashMap::new();

    for module in modules {
        grouped
            .entry(module.package_name.clone())
            .or_insert_with(Vec::new)
            .push(module.clone());
    }

    grouped
}

/// Find a specific module by name
pub fn find_module_by_name<'a>(
    modules: &'a [ModuleInfo],
    module_name: &str,
) -> Option<&'a ModuleInfo> {
    modules.iter().find(|m| m.name == module_name)
}

/// Get all module names from a specific package
#[allow(dead_code)]
pub fn get_modules_from_package<'a>(
    modules: &'a [ModuleInfo],
    package_name: &str,
) -> Vec<&'a ModuleInfo> {
    modules
        .iter()
        .filter(|m| m.package_name == package_name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_module_name_from_content() {
        // Test basic module declaration
        let content = "module Data.Maybe where\n\nimport Prelude";
        assert_eq!(
            extract_module_name_from_content(content).unwrap(),
            "Data.Maybe"
        );

        // Test module with exports
        let content = "module Data.Maybe (Maybe(..), maybe) where\n\nimport Prelude";
        assert_eq!(
            extract_module_name_from_content(content).unwrap(),
            "Data.Maybe"
        );

        // Test module with comments before
        let content = "-- This is a comment\nmodule Data.Maybe where\n\nimport Prelude";
        assert_eq!(
            extract_module_name_from_content(content).unwrap(),
            "Data.Maybe"
        );

        // Test module with empty lines before
        let content = "\n\nmodule Data.Maybe where\n\nimport Prelude";
        assert_eq!(
            extract_module_name_from_content(content).unwrap(),
            "Data.Maybe"
        );

        // Test no module declaration
        let content = "import Prelude\n\nmain = pure unit";
        assert!(extract_module_name_from_content(content).is_err());

        // Test module without 'where'
        let content = "module Data.Maybe\n\nimport Prelude";
        assert_eq!(
            extract_module_name_from_content(content).unwrap(),
            "Data.Maybe"
        );

        // Test module with brackets and no space
        let content = "module Data.Maybe(module Exported) where";
        assert_eq!(
            extract_module_name_from_content(content).unwrap(),
            "Data.Maybe"
        );

        // Test module with newline straight after module dec
        let content = "module Data.Maybe\n\nimport Prelude";
        assert_eq!(
            extract_module_name_from_content(content).unwrap(),
            "Data.Maybe"
        );
    }

    #[test]
    fn test_group_modules_by_package() {
        let modules = vec![
            ModuleInfo {
                name: "Data.Maybe".to_string(),
                file_path: PathBuf::from("test1.purs"),
                package_name: "prelude".to_string(),
            },
            ModuleInfo {
                name: "Data.Either".to_string(),
                file_path: PathBuf::from("test2.purs"),
                package_name: "prelude".to_string(),
            },
            ModuleInfo {
                name: "Effect.Console".to_string(),
                file_path: PathBuf::from("test3.purs"),
                package_name: "console".to_string(),
            },
        ];

        let grouped = group_modules_by_package(&modules);

        assert_eq!(grouped.len(), 2);
        assert!(grouped.contains_key("prelude"));
        assert!(grouped.contains_key("console"));

        let prelude_modules = &grouped["prelude"];
        assert_eq!(prelude_modules.len(), 2);
        assert!(prelude_modules.iter().any(|m| m.name == "Data.Maybe"));
        assert!(prelude_modules.iter().any(|m| m.name == "Data.Either"));

        let console_modules = &grouped["console"];
        assert_eq!(console_modules.len(), 1);
        assert!(console_modules.iter().any(|m| m.name == "Effect.Console"));
    }

    #[test]
    fn test_find_module_by_name() {
        let modules = vec![
            ModuleInfo {
                name: "Data.Maybe".to_string(),
                file_path: PathBuf::from("test1.purs"),
                package_name: "prelude".to_string(),
            },
            ModuleInfo {
                name: "Data.Either".to_string(),
                file_path: PathBuf::from("test2.purs"),
                package_name: "prelude".to_string(),
            },
        ];

        // Find existing module
        let found = find_module_by_name(&modules, "Data.Maybe");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Data.Maybe");
        assert_eq!(found.unwrap().package_name, "prelude");

        // Find non-existing module
        let not_found = find_module_by_name(&modules, "NonExistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_modules_from_package() {
        let modules = vec![
            ModuleInfo {
                name: "Data.Maybe".to_string(),
                file_path: PathBuf::from("test1.purs"),
                package_name: "prelude".to_string(),
            },
            ModuleInfo {
                name: "Data.Either".to_string(),
                file_path: PathBuf::from("test2.purs"),
                package_name: "prelude".to_string(),
            },
            ModuleInfo {
                name: "Effect.Console".to_string(),
                file_path: PathBuf::from("test3.purs"),
                package_name: "console".to_string(),
            },
        ];

        // Get modules from prelude package
        let prelude_modules = get_modules_from_package(&modules, "prelude");
        assert_eq!(prelude_modules.len(), 2);
        assert!(prelude_modules.iter().any(|m| m.name == "Data.Maybe"));
        assert!(prelude_modules.iter().any(|m| m.name == "Data.Either"));

        // Get modules from console package
        let console_modules = get_modules_from_package(&modules, "console");
        assert_eq!(console_modules.len(), 1);
        assert!(console_modules.iter().any(|m| m.name == "Effect.Console"));

        // Get modules from non-existing package
        let empty_modules = get_modules_from_package(&modules, "nonexistent");
        assert_eq!(empty_modules.len(), 0);
    }
}
