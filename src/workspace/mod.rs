use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use colored::Colorize;

use crate::config::{add_packages_to_config, load_config_cwd, remove_packages_from_config};
use crate::imports::extract_imports_from_sources;
use crate::modules::discover_all_modules;
use crate::registry::{LocalPackage, PackageName, PackageQuery};

pub fn execute_local_packages() -> Result<()> {
    let config = load_config_cwd()?;
    let package_set = config.package_set()?;
    let query = PackageQuery::new(&package_set);

    let all_local_packages = query.local_packages();
    for package in all_local_packages {
        println!("{}", package.path.display());
    }

    Ok(())
}

/// Check for circular dependencies in the workspace
pub fn check_circular_dependencies() -> Result<()> {
    let found_circular = PackageQuery::check_circular_dependencies()?;
    if found_circular {
        std::process::exit(1);
    }
    Ok(())
}

pub fn check_deps(
    package: Option<String>,
    commands_only: bool,
    broken_only: bool,
    fix: bool,
) -> Result<()> {
    let stats = fetch_workspace_dependency_stats()?;
    let config = load_config_cwd()?;
    let package_set = config.package_set()?;
    let query = PackageQuery::new(&package_set);

    if let Some(package) = package {
        let package = PackageName::new(&package);
        display_dependency_stats(&package, &stats, commands_only, broken_only);
        display_fix_instructions(Some(&package), &stats);
        return Ok(());
    }

    let all_local_packages = query.local_packages();
    for package in all_local_packages {
        if fix {
            fix_dependency_issues(&package, &stats)?;
        } else {
            display_dependency_stats(&package.name, &stats, commands_only, broken_only);
        }
    }

    let has_issues =
        stats.not_found.len() > 0 || stats.to_install.len() > 0 || stats.to_uninstall.len() > 0;

    // If we're not fixing dependencies and there are any issues, exit with an error
    if !fix && has_issues {
        display_fix_instructions(None, &stats);
        std::process::exit(1);
    } else if fix && has_issues {
        println!("{}", "Fixing dependencies...".green());
    } else {
        println!("{}", "No issues detected.".green());
    }
    Ok(())
}

pub struct DependencyStats {
    pub to_install: HashMap<PackageName, HashSet<PackageName>>,
    pub to_uninstall: HashMap<PackageName, HashSet<PackageName>>,
    pub not_found: HashMap<PackageName, HashSet<String>>,
}

pub fn fetch_workspace_dependency_stats() -> Result<DependencyStats> {
    let config = load_config_cwd()?;
    let package_set = config.package_set()?;
    let query = PackageQuery::new(&package_set);

    let all_local_packages = query.local_packages();

    let workspace_root_config =
        crate::config::load_config(Path::join(&config.workspace_root, "spago.yaml"), false)?;

    // Use current project config for both sources and workspace sources
    // This ensures we're looking in the correct .spago directory (current project's)
    // but with access to the workspace root's package set
    let workspace_sources = crate::sources::generate_sources(
        &workspace_root_config,
        Some(package_set.clone()),
        true,
        false,
        false,
    )?;

    let workspace_modules = discover_all_modules(&workspace_sources)?
        .iter()
        .map(|m| (m.name.clone(), PackageName::new(&m.package_name)))
        .collect::<HashMap<String, PackageName>>();

    let mut to_install: HashMap<PackageName, HashSet<PackageName>> = HashMap::new();
    let mut to_uninstall: HashMap<PackageName, HashSet<PackageName>> = HashMap::new();
    let mut not_found: HashMap<PackageName, HashSet<String>> = HashMap::new();

    for local_package in all_local_packages {
        let imports = extract_imports_from_sources(local_package.path.as_path())?;
        let deps: HashSet<&PackageName> = local_package.dependencies.iter().collect();
        let mut installed = HashSet::new();
        for import in imports {
            if PRIMITIVE.contains(&import.module_name.as_str()) {
                continue; // Is primitive, ignore
            }
            if let Some(import_package) = workspace_modules.get(&import.module_name) {
                if import_package.0 == local_package.name.0 {
                    continue; // Is current package, ignore
                }
                if deps.contains(import_package) {
                    installed.insert(import_package);
                    continue; // direct dependency, ignore
                }

                to_install
                    .entry(local_package.name.clone())
                    .or_insert_with(HashSet::new)
                    .insert(import_package.clone());
            } else {
                not_found
                    .entry(local_package.name.clone())
                    .or_insert_with(HashSet::new)
                    .insert(import.module_name);
            }
        }

        for dep in deps {
            if !installed.contains(dep) {
                to_uninstall
                    .entry(local_package.name.clone())
                    .or_insert_with(HashSet::new)
                    .insert(dep.clone());
            }
        }
    }

    Ok(DependencyStats {
        to_install,
        to_uninstall,
        not_found,
    })
}

pub fn display_dependency_stats(
    package: &PackageName,
    stats: &DependencyStats,
    commands_only: bool,
    broken_only: bool,
) {
    let to_install = stats.to_install.get(package);
    let to_uninstall = stats.to_uninstall.get(package);
    let not_found = stats.not_found.get(package);

    if !commands_only && (!broken_only || not_found.is_some()) {
        if to_install.is_some() || to_uninstall.is_some() || not_found.is_some() {
            println!("");
            println!("Package: {}", package.0);
        }
    }

    if !broken_only {
        if let Some(to_install) = to_install {
            println!(
                "{} {}",
                "To install:".dimmed(),
                to_install
                    .iter()
                    .map(|p| p.0.clone())
                    .collect::<Vec<String>>()
                    .join(" ")
            );
        }

        if let Some(to_uninstall) = to_uninstall {
            println!(
                "{} {}",
                "To uninstall:".dimmed(),
                to_uninstall
                    .iter()
                    .map(|p| p.0.clone())
                    .collect::<Vec<String>>()
                    .join(" ")
            );
        }
    }
    if !commands_only {
        if let Some(not_found) = not_found {
            println!("");
            println!("Not found:");
            println!(
                "Dependencies not found in workspace: {}",
                not_found
                    .iter()
                    .map(|p| p.clone())
                    .collect::<Vec<String>>()
                    .join(" ")
            );
        }
    }
}

fn display_fix_instructions(package: Option<&PackageName>, stats: &DependencyStats) {
    if let Some(package) = package {
        let to_install = stats.to_install.get(package);
        let to_uninstall = stats.to_uninstall.get(package);

        // Only show commands if there are things to install or uninstall
        let has_install = to_install.map_or(false, |set| !set.is_empty());
        let has_uninstall = to_uninstall.map_or(false, |set| !set.is_empty());

        if has_install || has_uninstall {
            println!();
            if let Some(to_install) = to_install {
                if !to_install.is_empty() {
                    let packages: Vec<String> = to_install.iter().map(|p| p.0.clone()).collect();
                    println!("ragu install {}", packages.join(" "));
                }
            }

            if let Some(to_uninstall) = to_uninstall {
                if !to_uninstall.is_empty() {
                    let packages: Vec<String> = to_uninstall.iter().map(|p| p.0.clone()).collect();
                    println!("ragu uninstall {}", packages.join(" "));
                }
            }
        }
    } else {
        // Check if there are any actions needed globally
        let has_any_actions = stats.to_install.values().any(|v| !v.is_empty())
            || stats.to_uninstall.values().any(|v| !v.is_empty());

        if has_any_actions {
            println!();
            println!(
                "{}{}{}",
                "Run ".dimmed(),
                "ragu check-deps -f",
                " to fix all issues.".dimmed()
            );
        }
    }
}

fn fix_dependency_issues(package: &LocalPackage, stats: &DependencyStats) -> Result<()> {
    let to_install = stats.to_install.get(&package.name);
    let to_uninstall = stats.to_uninstall.get(&package.name);
    let not_found = stats.not_found.get(&package.name);

    if let Some(to_install) = to_install {
        let to_install = to_install
            .into_iter()
            .map(|p| p.clone())
            .collect::<Vec<PackageName>>();
        add_packages_to_config(
            &PathBuf::from(Path::join(package.path.as_path(), "spago.yaml")),
            &to_install,
        )?;
    }

    if let Some(to_uninstall) = to_uninstall {
        let to_uninstall = to_uninstall
            .into_iter()
            .map(|p| p.clone())
            .collect::<Vec<PackageName>>();

        remove_packages_from_config(
            &PathBuf::from(Path::join(package.path.as_path(), "spago.yaml")),
            &to_uninstall,
        )?;
    }

    if let Some(not_found) = not_found {
        let not_found = not_found
            .into_iter()
            .map(|p| p.clone())
            .collect::<Vec<String>>();

        println!(
            "📦 {} had dependencies not found in the workspace:",
            package.name.0
        );
        println!("{}", not_found.join("\n"));
    }

    Ok(())
}

const PRIMITIVE: [&str; 5] = [
    "Prim.Row",
    "Prim.RowList",
    "Prim.Symbol",
    "Prim.TypeError",
    "Prim.Boolean",
];
