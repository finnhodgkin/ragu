use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::SpagoConfig;
use crate::install::InstallManager;
use crate::registry::{Package, PackageName, PackageQuery, PackageSet};

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

/// Execute the sources command - outputs just the source globs for piping
pub fn execute_sources(verbose: bool) -> Result<()> {
    if verbose {
        println!("{} Generating source globs", "→".cyan());
    }

    // Load spago.yaml configuration
    let config =
        crate::config::load_config_cwd().context("Failed to load spago.yaml configuration")?;

    // Generate source globs for dependencies
    let sources = generate_sources(&config, None, false, verbose)?;

    // Output main sources
    println!("{}", sources.main_sources);

    // Output dependency sources
    for glob in &sources.dependency_globs {
        println!("{}", glob.glob_pattern);
    }

    Ok(())
}

/// Generate source globs for all dependencies
pub fn generate_sources(
    config: &SpagoConfig,
    package_set: Option<PackageSet>,
    all: bool,
    verbose: bool,
) -> Result<BuildSources> {
    let spago_dir = &config.spago_dir();

    if !spago_dir.exists() {
        return Err(anyhow::anyhow!(
            "No .spago directory found. Run 'spago install' first to install dependencies."
        ));
    }

    let package_set = match package_set {
        None => config.package_set()?,
        Some(package_set) => package_set,
    };

    let mut all_dependencies = HashSet::new();
    let mut processed_packages: HashSet<PackageName> = HashSet::new();

    let manager = InstallManager::new(spago_dir)?;
    let query = PackageQuery::new(&package_set);

    let direct_package_dependencies: Vec<PackageName> = if config.is_workspace_root() || all {
        query.all_workspace_dependencies()
    } else {
        config.package_dependencies().into_iter().cloned().collect()
    };

    for dep_name in direct_package_dependencies {
        manager.collect_dependencies_recursive(
            &dep_name,
            &query,
            &mut all_dependencies,
            &mut processed_packages,
        )?;
    }

    let main_sources = "./src/**/*.purs".to_string();

    let mut dependency_globs = Vec::new();
    // Generate globs for each dependency (including transitive ones)
    for dep_name in all_dependencies {
        // The current package should be handled by main sources instead.
        // This generally shouldn't happen, but if there are funky local
        // circular deps, this will prevent multiple sources the for the same
        // package.
        if dep_name == config.package.name {
            continue;
        }
        if let Some(glob) = generate_dependency_glob(&dep_name, spago_dir, &package_set, verbose)? {
            if glob.glob_pattern != main_sources {
                dependency_globs.push(glob);
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

    Ok(BuildSources {
        dependency_globs,
        main_sources,
    })
}

#[cfg(test)]
mod tests;

/// Generate a glob pattern for a specific dependency
pub fn generate_dependency_glob(
    package_name: &PackageName,
    spago_dir: &Path,
    package_set: &PackageSet,
    verbose: bool,
) -> Result<Option<DependencyGlob>> {
    // Find the installed package directory
    let package_dir = find_package_directory(package_name, spago_dir, package_set)?;

    if let Some(dir) = package_dir {
        // Check if the package has source files
        let src_dir = dir.join("src");
        if src_dir.exists() && src_dir.is_dir() {
            let glob_pattern = format!("{}/**/*.purs", src_dir.display());

            if verbose {
                println!("  {} -> {}", package_name.0, glob_pattern);
            }

            return Ok(Some(DependencyGlob {
                package_name: package_name.0.clone(),
                glob_pattern,
                local_path: dir,
            }));
        } else if verbose {
            return Err(anyhow::anyhow!(
                "No src directory found for package {}",
                package_name.0
            ));
        }
    } else if verbose {
        println!("  {} -> Package not found", package_name.0);
    }

    Err(anyhow::anyhow!(
        "Package {} not found. Couldn't generate a glob for it.",
        package_name.0
    ))
}

/// Find the installed package directory in .spago
pub fn find_package_directory(
    package_name: &PackageName,
    spago_dir: &Path,
    package_set: &PackageSet,
) -> Result<Option<PathBuf>> {
    if let Some(Package::Local(package)) = package_set.get(package_name) {
        return Ok(Some(package.path.clone()));
    }

    let entries = fs::read_dir(spago_dir).context("Failed to read .spago directory")?;

    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_dir() {
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // Check if this directory matches the package name
            // Package directories are typically named like "package-name-version"
            if dir_name == package_name.0 {
                return Ok(Some(path));
            }
        }
    }

    Err(anyhow::anyhow!(
        "Package {} not found in .spago",
        package_name.0
    ))
}
