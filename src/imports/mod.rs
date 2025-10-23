use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::modules::discover_all_modules;

/// Information about an import found in source code
#[derive(Debug, Clone)]
pub struct ImportInfo {
    /// The module name being imported
    pub module_name: String,
    /// The file where this import was found
    #[allow(dead_code)]
    pub file_path: String,
    /// The line number where the import was found
    #[allow(dead_code)]
    pub line_number: usize,
}

/// Execute the imports command
pub fn execute(verbose: bool) -> Result<()> {
    if verbose {
        println!("{} Parsing imports from current project", "→".cyan());
    }

    let config =
        crate::config::load_config_cwd().context("Failed to load spago.yaml configuration")?;

    // Use current project config for sources since that's where dependencies are installed
    let sources = crate::sources::generate_sources(&config, None, false, false, verbose)?;

    let package_modules = discover_all_modules(&sources)?
        .into_iter()
        .map(|m| (m.name, m.package_name))
        .collect::<HashMap<String, String>>();

    // Extract imports from current project's source files
    let imports = extract_imports_from_sources(&std::env::current_dir()?)?;
    // Create a map to group imports by package
    let mut grouped_imports: HashMap<String, Vec<&ImportInfo>> = HashMap::new();

    // Group imports by package name, using "unknown" for those without a package
    for import in &imports {
        let package = package_modules
            .get(&import.module_name)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        grouped_imports.entry(package).or_default().push(import);
    }

    // Sort packages and print imports
    let mut packages: Vec<_> = grouped_imports.keys().collect();
    packages.sort();

    for package in packages {
        let mut imports_in_package = grouped_imports.get(package).unwrap().to_vec();
        imports_in_package.sort_by(|a, b| a.module_name.cmp(&b.module_name));

        for import in imports_in_package {
            if package == "main" {
                println!("{}, ({})", import.module_name, "current".bright_cyan());
            } else if package == "unknown" {
                println!("{}", import.module_name);
            } else {
                println!("{} ({})", import.module_name, package.dimmed());
            }
        }
    }

    Ok(())
}

/// Extract all import statements from PureScript source files
pub fn extract_imports_from_sources(dir: &Path) -> Result<Vec<ImportInfo>> {
    let mut imports = Vec::new();
    let src_dir = dir.join("src");

    if !src_dir.exists() {
        return Ok(imports);
    }

    extract_imports_from_directory(&src_dir, &mut imports)?;

    Ok(imports)
}

/// Recursively extract imports from a directory
fn extract_imports_from_directory(dir: &Path, imports: &mut Vec<ImportInfo>) -> Result<()> {
    let entries = fs::read_dir(dir).context("Failed to read directory")?;

    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_dir() {
            extract_imports_from_directory(&path, imports)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("purs") {
            extract_imports_from_file(&path, imports)?;
        }
    }

    Ok(())
}

/// Extract imports from a single PureScript file
fn extract_imports_from_file(file_path: &Path, imports: &mut Vec<ImportInfo>) -> Result<()> {
    let content = fs::read_to_string(file_path).context("Failed to read file")?;

    let mut is_comment_block = false;

    for (line_number, line) in content.lines().enumerate() {
        let line = line.trim_end();

        // Skip comments and empty lines
        if line.is_empty()
            || line.starts_with("--")
            || line.starts_with(" ")
            || line.starts_with("module ")
        {
            continue;
        } else if line.starts_with("{-") {
            is_comment_block = true;
            continue;
        } else if line.starts_with("-}") {
            is_comment_block = false;
            continue;
        } else if is_comment_block {
            continue;
        }
        // Look for import statements
        else if line.starts_with("import ") {
            if let Some(module_name) = extract_module_name_from_import(line) {
                imports.push(ImportInfo {
                    module_name,
                    file_path: file_path.to_string_lossy().to_string(),
                    line_number: line_number + 1,
                });
            }
        } else {
            break;
        }
    }

    Ok(())
}

/// Extract module name from an import statement
fn extract_module_name_from_import(import_line: &str) -> Option<String> {
    // Remove "import " prefix
    let module_part = import_line.strip_prefix("import ")?;

    // Find the end of the module name (before any imports or "as" or "hiding")
    let module_name = if let Some(hiding_pos) = module_part.find(" hiding") {
        &module_part[..hiding_pos]
    } else if let Some(import_pos) = module_part.find(" (") {
        &module_part[..import_pos]
    } else if let Some(as_pos) = module_part.find(" as ") {
        &module_part[..as_pos]
    } else {
        module_part.split_whitespace().next().unwrap_or(module_part)
    };

    Some(module_name.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_module_name_from_import() {
        // Basic import
        assert_eq!(
            extract_module_name_from_import("import Data.Maybe"),
            Some("Data.Maybe".to_string())
        );

        // Import with exports
        assert_eq!(
            extract_module_name_from_import("import Data.Maybe (Maybe(..), maybe)"),
            Some("Data.Maybe".to_string())
        );

        // Import with as
        assert_eq!(
            extract_module_name_from_import("import Data.Maybe as Maybe"),
            Some("Data.Maybe".to_string())
        );

        // Import with hiding
        assert_eq!(
            extract_module_name_from_import("import Data.Maybe hiding (fromMaybe)"),
            Some("Data.Maybe".to_string())
        );

        // Complex import
        assert_eq!(
            extract_module_name_from_import("import Data.Maybe (Maybe(..), maybe) as Maybe"),
            Some("Data.Maybe".to_string())
        );
    }
}
