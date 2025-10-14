use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use colored::Colorize;
use glob::glob;
use rayon::prelude::*;
mod benchmark;
mod import_parsing;

use crate::{
    build::compiler::execute_compiler,
    sources::{generate_sources, BuildSources},
};

use import_parsing::parse_purescript_file;

pub fn execute(build: bool, verbose: bool) -> Result<()> {
    if verbose {
        println!("{} Generating source globs from src", "→".cyan());
    }

    let config = crate::config::load_config_cwd()?;
    let sources = generate_sources(&config, None, false, verbose)?;

    let modules = discover_all_modules(sources)?;

    println!("{}", "Starting compilation".dimmed());

    if build {
        let sources = modules
            .iter()
            .map(|m| m.file_path.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        execute_compiler(&sources, &config.output_dir(), verbose)?;
    } else {
        println!(
            "{}",
            modules
                .iter()
                .map(|m| m.file_path.display().to_string())
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    println!("{}", "Compilation successful".green());

    Ok(())
}

/// Information about a discovered module
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// The module name (e.g., "Data.Maybe")
    pub name: String,
    /// The file path where this module is defined
    #[allow(dead_code)]
    pub file_path: PathBuf,
    /// The imports for this module
    pub imports: Vec<String>,
}

/// Discover all modules in the project and its dependencies
fn discover_all_modules(sources: BuildSources) -> Result<Vec<ModuleInfo>> {
    // First, get all dependency sources to build a module name -> file path mapping
    let mut all_globs = sources
        .dependency_globs
        .into_iter()
        .map(|g| g.glob_pattern.clone())
        .collect::<Vec<String>>();
    all_globs.push(sources.main_sources.clone());

    // Build a mapping from module names to file paths from all sources
    let module_to_file: HashMap<String, PathBuf> = build_module_mapping(&all_globs)?;

    // Get main source files
    let main_files = get_files_from_glob(&sources.main_sources)?;

    // Recursively find all required files starting from main sources
    let required_files = find_transitive_dependencies(&main_files, &module_to_file)?;

    // Convert file paths back to ModuleInfo structs
    let modules: Vec<ModuleInfo> = required_files
        .par_iter()
        .filter_map(|path| extract_info_from_file(path).ok())
        .collect();

    Ok(modules)
}

/// Build a mapping from module names to file paths from all glob patterns
fn build_module_mapping(globs: &[String]) -> Result<HashMap<String, PathBuf>> {
    let results: Result<Vec<HashMap<String, PathBuf>>> = globs
        .par_iter()
        .map(|pattern| {
            let files = get_files_from_glob(pattern)?;
            let mut mapping = HashMap::new();

            for file in files {
                if let Ok(module_info) = extract_info_from_file(&file) {
                    mapping.insert(module_info.name, file);
                }
            }

            Ok(mapping)
        })
        .collect();

    // Merge all mappings
    let mut combined_mapping = HashMap::new();
    for mapping in results? {
        combined_mapping.extend(mapping);
    }

    Ok(combined_mapping)
}

/// Get all .purs files from a glob pattern
fn get_files_from_glob(pattern: &str) -> Result<Vec<PathBuf>> {
    glob(pattern)
        .context("Failed to read glob pattern")?
        .filter_map(|entry| {
            match entry {
                Ok(path) if path.extension().and_then(|s| s.to_str()) == Some("purs") => {
                    Some(Ok(path))
                }
                Ok(_) => None, // Skip non-.purs files
                Err(e) => Some(Err(anyhow::anyhow!("Glob error: {}", e))),
            }
        })
        .collect()
}

/// Recursively find all transitive dependencies starting from main files
fn find_transitive_dependencies(
    main_files: &[PathBuf],
    module_to_file: &HashMap<String, PathBuf>,
) -> Result<Vec<PathBuf>> {
    let mut visited_files = std::collections::HashSet::new();
    let mut files_to_process = main_files.to_vec();
    let mut all_required_files = Vec::new();

    while let Some(file_path) = files_to_process.pop() {
        if visited_files.contains(&file_path) {
            continue;
        }

        visited_files.insert(file_path.clone());
        all_required_files.push(file_path.clone());

        // Extract imports from this file
        let module = extract_info_from_file(&file_path)?;

        // For each import, find the corresponding file and add it to processing queue
        for import in module.imports {
            if let Some(dependency_file) = module_to_file.get(&import) {
                if !visited_files.contains(dependency_file) {
                    files_to_process.push(dependency_file.clone());
                }
            }
        }
    }

    Ok(all_required_files)
}

fn extract_info_from_file(path: &PathBuf) -> Result<ModuleInfo> {
    use std::fs;

    let content = fs::read_to_string(path).context("Failed to read file")?;

    // Use our new fast parser
    let (_, parsed) = parse_purescript_file(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse PureScript file: {:?}", e))?;

    let module_name = parsed
        .module
        .map(|m| m.name)
        .ok_or_else(|| anyhow::anyhow!("No module declaration found"))?;

    let imports = parsed.imports.into_iter().map(|i| i.module_name).collect();

    Ok(ModuleInfo {
        name: module_name,
        file_path: path.clone(),
        imports,
    })
}
