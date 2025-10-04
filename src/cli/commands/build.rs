use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::SpagoConfig;

/// Build command result containing source globs for each dependency
#[derive(Debug)]
pub struct BuildSources {
    /// Source globs for each dependency package
    pub dependency_globs: Vec<DependencyGlob>,
    /// Main source files (src/**/*.purs)
    pub main_sources: String,
}

/// Source globs for a specific dependency
#[derive(Debug, Clone)]
pub struct DependencyGlob {
    /// Name of the dependency package
    pub package_name: String,
    /// Glob pattern for this dependency's source files
    pub glob_pattern: String,
    /// Local path to the dependency
    #[allow(dead_code)]
    pub local_path: PathBuf,
}

/// Execute the build command
pub fn execute(watch: bool, clear: bool, verbose: bool) -> Result<()> {
    if verbose {
        println!("{} Build command executing", "→".cyan());
        println!("  Watch: {}", watch);
        println!("  Clear: {}", clear);
    }

    // Load spago.yaml configuration
    let config =
        crate::config::load_config_cwd().context("Failed to load spago.yaml configuration")?;

    // Generate source globs for dependencies
    let sources = generate_sources(&config, verbose)?;

    if verbose {
        println!(
            "{} Generated {} dependency globs",
            "→".cyan(),
            sources.dependency_globs.len()
        );
        for glob in &sources.dependency_globs {
            println!("  {}: {}", glob.package_name.blue(), glob.glob_pattern);
        }
    }

    // TODO: Implement actual purs compiler invocation
    println!("{} Build sources generated successfully", "✓".green());
    println!("  Main sources: {}", sources.main_sources);
    println!(
        "  Dependencies: {} packages",
        sources.dependency_globs.len()
    );

    Ok(())
}

/// Execute the sources command - outputs just the source globs for piping
pub fn execute_sources(verbose: bool) -> Result<()> {
    if verbose {
        println!("{} Generating source globs", "→".cyan());
    }

    // Load spago.yaml configuration
    let config =
        crate::config::load_config_cwd().context("Failed to load spago.yaml configuration")?;

    // Generate source globs for dependencies
    let sources = generate_sources(&config, verbose)?;

    // Output main sources
    println!("{}", sources.main_sources);

    // Output dependency sources
    for glob in &sources.dependency_globs {
        println!("{}", glob.glob_pattern);
    }

    Ok(())
}

/// Generate source globs for all dependencies
pub fn generate_sources(config: &SpagoConfig, verbose: bool) -> Result<BuildSources> {
    let spago_dir = Path::new(".spago");

    if !spago_dir.exists() {
        return Err(anyhow::anyhow!(
            "No .spago directory found. Run 'spago install' first to install dependencies."
        ));
    }

    // Get package dependencies (excluding test dependencies for now)
    let package_deps = config.package_dependencies();

    if verbose {
        println!(
            "{} Found {} direct package dependencies",
            "→".cyan(),
            package_deps.len()
        );
    }

    let mut dependency_globs = Vec::new();
    let mut processed_packages: HashSet<String> = HashSet::new();

    // Generate globs for each dependency (including transitive ones)
    for dep_name in package_deps {
        if processed_packages.contains(dep_name) {
            continue;
        }

        if let Some(glob) = generate_dependency_glob(dep_name, spago_dir, verbose)? {
            dependency_globs.push(glob);
            processed_packages.insert(dep_name.to_string());
        }
    }

    // Also scan for any other packages that might be transitive dependencies
    // This is a fallback to catch any packages that weren't in the direct dependencies
    if let Ok(entries) = std::fs::read_dir(spago_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        // Extract package name from directory name (format: package-name-version)
                        if let Some(package_name) = extract_package_name_from_dir(dir_name) {
                            if !processed_packages.contains(&package_name) {
                                if let Some(glob) =
                                    generate_dependency_glob(&package_name, spago_dir, verbose)?
                                {
                                    dependency_globs.push(glob);
                                    processed_packages.insert(package_name);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if verbose {
        println!(
            "{} Generated {} total dependency globs",
            "→".cyan(),
            dependency_globs.len()
        );
    }

    // Main sources glob (src/**/*.purs)
    let main_sources = "src/**/*.purs".to_string();

    Ok(BuildSources {
        dependency_globs,
        main_sources,
    })
}

/// Generate a glob pattern for a specific dependency
fn generate_dependency_glob(
    package_name: &str,
    spago_dir: &Path,
    verbose: bool,
) -> Result<Option<DependencyGlob>> {
    // Find the installed package directory
    let package_dir = find_package_directory(package_name, spago_dir)?;

    if let Some(dir) = package_dir {
        // Check if the package has source files
        let src_dir = dir.join("src");
        if src_dir.exists() && src_dir.is_dir() {
            let glob_pattern = format!("{}/**/*.purs", src_dir.display());

            if verbose {
                println!("  {} -> {}", package_name, glob_pattern);
            }

            return Ok(Some(DependencyGlob {
                package_name: package_name.to_string(),
                glob_pattern,
                local_path: dir,
            }));
        } else if verbose {
            println!("  {} -> No src directory found", package_name);
        }
    } else if verbose {
        println!("  {} -> Package not found in .spago", package_name);
    }

    Ok(None)
}

/// Extract package name from directory name (format: package-name-version)
fn extract_package_name_from_dir(dir_name: &str) -> Option<String> {
    // Find the last dash that's followed by a version number
    // This handles cases like "package-name-1.0.0" or "package-name-v1.0.0"
    let parts: Vec<&str> = dir_name.split('-').collect();
    if parts.len() >= 2 {
        // Try to find where the version starts
        for i in (1..parts.len()).rev() {
            let potential_version = &parts[i..].join("-");
            // Check if this looks like a version (contains digits and dots)
            if potential_version.chars().any(|c| c.is_ascii_digit()) {
                let package_name = parts[..i].join("-");
                if !package_name.is_empty() {
                    return Some(package_name);
                }
            }
        }
    }
    None
}

/// Find the installed package directory in .spago
fn find_package_directory(package_name: &str, spago_dir: &Path) -> Result<Option<PathBuf>> {
    let entries = fs::read_dir(spago_dir).context("Failed to read .spago directory")?;

    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_dir() {
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Check if this directory matches the package name
            // Package directories are typically named like "package-name-version"
            if dir_name.starts_with(package_name) {
                return Ok(Some(path));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_generate_dependency_glob() {
        let temp_dir = tempdir().unwrap();
        let spago_dir = temp_dir.path();

        // Create a mock package directory structure
        let package_dir = spago_dir.join("test-package-1.0.0");
        let src_dir = package_dir.join("src");
        fs::create_dir_all(&src_dir).unwrap();

        // Create a test .purs file
        fs::write(src_dir.join("Test.purs"), "module Test where").unwrap();

        let result = generate_dependency_glob("test-package", spago_dir, false).unwrap();

        assert!(result.is_some());
        let glob = result.unwrap();
        assert_eq!(glob.package_name, "test-package");
        assert!(glob
            .glob_pattern
            .contains("test-package-1.0.0/src/**/*.purs"));
    }

    #[test]
    fn test_find_package_directory() {
        let temp_dir = tempdir().unwrap();
        let spago_dir = temp_dir.path();

        // Create mock package directories
        fs::create_dir_all(spago_dir.join("package-a-1.0.0")).unwrap();
        fs::create_dir_all(spago_dir.join("package-b-2.0.0")).unwrap();
        fs::create_dir_all(spago_dir.join("other-package-1.0.0")).unwrap();

        // Test finding existing package
        let result = find_package_directory("package-a", spago_dir).unwrap();
        assert!(result.is_some());
        assert!(result
            .unwrap()
            .to_string_lossy()
            .contains("package-a-1.0.0"));

        // Test finding non-existing package
        let result = find_package_directory("nonexistent", spago_dir).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_generate_sources_with_mock_config() {
        let temp_dir = tempdir().unwrap();
        let spago_dir = temp_dir.path().join(".spago");

        // Create mock package directories with source files
        let package_a_dir = spago_dir.join("package-a-1.0.0");
        let package_a_src = package_a_dir.join("src");
        fs::create_dir_all(&package_a_src).unwrap();
        fs::write(package_a_src.join("ModuleA.purs"), "module ModuleA where").unwrap();

        let package_b_dir = spago_dir.join("package-b-2.0.0");
        let package_b_src = package_b_dir.join("src");
        fs::create_dir_all(&package_b_src).unwrap();
        fs::write(package_b_src.join("ModuleB.purs"), "module ModuleB where").unwrap();

        // Create mock config
        let config = SpagoConfig {
            package: crate::config::PackageConfig {
                name: "test-project".to_string(),
                dependencies: vec!["package-a".to_string(), "package-b".to_string()],
                test: None,
            },
            workspace: crate::config::WorkspaceConfig::default(),
        };

        // Change to temp directory for the test
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = generate_sources(&config, false).unwrap();

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert_eq!(result.dependency_globs.len(), 2);
        assert_eq!(result.main_sources, "src/**/*.purs");

        // Check that both packages are included
        let package_names: HashSet<String> = result
            .dependency_globs
            .iter()
            .map(|g| g.package_name.clone())
            .collect();
        assert!(package_names.contains("package-a"));
        assert!(package_names.contains("package-b"));
    }
}
