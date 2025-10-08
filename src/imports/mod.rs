use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::modules::{discover_all_modules, find_module_by_name};

/// Information about an import found in source code
#[derive(Debug, Clone)]
pub struct ImportInfo {
    /// The module name being imported
    pub module_name: String,
    /// The file where this import was found
    pub file_path: String,
    /// The line number where the import was found
    pub line_number: usize,
}

const PRIMITIVE: [&str; 1] = ["Prim.Row"];

/// Execute the imports command
pub fn execute(verbose: bool) -> Result<()> {
    if verbose {
        println!("{} Parsing imports from current project", "→".cyan());
    }

    let config =
        crate::config::load_config_cwd().context("Failed to load spago.yaml configuration")?;

    // Use current project config for sources since that's where dependencies are installed
    let sources = crate::sources::generate_sources(&config, None, false, verbose)?;

    let workspace_root_config =
        crate::config::load_config(Path::join(&config.workspace_root, "spago.yaml"))?;

    let workspace_package_set = workspace_root_config.package_set()?;

    // Use current project config for both sources and workspace sources
    // This ensures we're looking in the correct .spago directory (current project's)
    // but with access to the workspace root's package set
    let workspace_sources = crate::sources::generate_sources(
        &workspace_root_config,
        Some(workspace_package_set),
        true,
        verbose,
    )?;

    let package_modules = discover_all_modules(&sources)?;
    let workspace_modules = discover_all_modules(&workspace_sources)?;

    // Extract imports from current project's source files
    let imports = extract_imports_from_sources(&std::env::current_dir()?)?;

    let deps = config.package_dependencies();
    let mut installed = HashSet::new();
    let mut indirect_deps_to_install = HashSet::new();
    let mut workspace_deps_to_install = HashSet::new();
    let mut not_found_deps = HashSet::new();

    for import in imports {
        if PRIMITIVE.contains(&import.module_name.as_str()) {
            continue;
        }

        if let Some(module) = find_module_by_name(&package_modules, &import.module_name) {
            if module.package_name == "main" || module.package_name == config.package.name.0 {
                continue;
            } else if deps.iter().any(|f| f.0 == module.package_name) {
                installed.insert(module.package_name.clone());
            } else {
                indirect_deps_to_install.insert(module.package_name.clone());
            }
        } else if let Some(module) = find_module_by_name(&workspace_modules, &import.module_name) {
            workspace_deps_to_install.insert(module.package_name.clone());
        } else {
            not_found_deps.insert(import.module_name);
        }
    }

    let mut to_uninstall = HashSet::new();
    for dep in &deps {
        if !installed.contains(&dep.0) {
            to_uninstall.insert(dep.0.clone());
        }
    }

    // Verbose logging for detailed analysis
    if verbose {
        if !indirect_deps_to_install.is_empty() {
            println!("Transitive dependencies that you are directly calling:");
            for dep in &indirect_deps_to_install {
                println!("  {}", dep);
            }
        }

        if !workspace_deps_to_install.is_empty() {
            println!("Workspace dependencies that you are missing but directly calling:");
            for dep in &workspace_deps_to_install {
                println!("  {}", dep);
            }
        }

        if !not_found_deps.is_empty() {
            println!("Modules that don't appear to be in your project at all:");
            for dep in &not_found_deps {
                println!("  {}", dep);
            }
        }

        if !to_uninstall.is_empty() {
            println!("Dependencies that have no imports:");
            for dep in &to_uninstall {
                println!("  {}", dep);
            }
        }
    }

    let all_to_install: Vec<String> = indirect_deps_to_install
        .union(&workspace_deps_to_install)
        .cloned()
        .collect();

    let mut errors = false;

    // Check if there's anything to fix
    if all_to_install.is_empty() && to_uninstall.is_empty() {
        println!("{} All imports are properly configured", "✓".green());
        return Ok(());
    }

    // Show fix commands
    println!("To fix these issues, you can run:");
    if !all_to_install.is_empty() {
        println!("spago-rust install {}", all_to_install.join(" "));
        errors = true;
    }
    if !to_uninstall.is_empty() {
        let to_uninstall_str = to_uninstall
            .iter()
            .cloned()
            .collect::<Vec<String>>()
            .join(" ");
        println!("spago-rust uninstall {}", to_uninstall_str);
    }

    // Show error for modules not found in workspace
    if !not_found_deps.is_empty() {
        errors = true;
        println!(
            "{} Modules not found in workspace: {}",
            "❌".red().bold(),
            not_found_deps
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    if errors {
        std::process::exit(1);
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

    for (line_number, line) in content.lines().enumerate() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with("--") {
            continue;
        }

        // Look for import statements
        if line.starts_with("import ") {
            if let Some(module_name) = extract_module_name_from_import(line) {
                imports.push(ImportInfo {
                    module_name,
                    file_path: file_path.to_string_lossy().to_string(),
                    line_number: line_number + 1,
                });
            }
        }
    }

    Ok(())
}

/// Extract module name from an import statement
fn extract_module_name_from_import(import_line: &str) -> Option<String> {
    // Remove "import " prefix
    let module_part = import_line.strip_prefix("import ")?;

    // Find the end of the module name (before any imports or "as" or "hiding")
    let module_name = if let Some(import_pos) = module_part.find(" (") {
        &module_part[..import_pos]
    } else if let Some(as_pos) = module_part.find(" as ") {
        &module_part[..as_pos]
    } else if let Some(hiding_pos) = module_part.find(" hiding ") {
        &module_part[..hiding_pos]
    } else {
        module_part
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
