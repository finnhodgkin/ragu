use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use colored::Colorize;
use glob::glob;
use rayon::prelude::*;
mod import_parsing;

use crate::{
    build::compiler::execute_compiler,
    sources::{generate_sources, BuildSources},
    test::TEST_SOURCES,
};

use import_parsing::parse_purescript_file;

pub fn execute(
    include_test_sources: bool,
    build: bool,
    compiler_args: Vec<String>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("{} Generating source globs from src", "→".cyan());
    }

    let config = crate::config::load_config_cwd()?;
    let sources = generate_sources(&config, None, false, verbose)?;

    let modules = discover_all_modules(sources, include_test_sources)?;

    println!("{}", "Starting compilation".dimmed());

    if build {
        let sources = modules
            .iter()
            .map(|m| m.file_path.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        execute_compiler(&sources, &config.output_dir(), compiler_args, verbose)?;
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
fn discover_all_modules(
    sources: BuildSources,
    include_test_sources: bool,
) -> Result<Vec<ModuleInfo>> {
    // First, get all dependency sources to build a module name -> file path mapping
    let mut all_globs = sources
        .dependency_globs
        .into_iter()
        .map(|g| g.glob_pattern.clone())
        .collect::<Vec<String>>();
    all_globs.push(sources.main_sources.clone());

    // Build a mapping from module names to ModuleInfo from all sources
    let module_to_info: HashMap<String, ModuleInfo> = build_module_mapping(&all_globs)?;

    // Get main source files
    let mut main_files = get_files_from_glob(&sources.main_sources)?;

    if include_test_sources {
        main_files.extend(get_files_from_glob(&TEST_SOURCES)?);
    }

    // Recursively find all required modules starting from main sources
    let required_modules = find_transitive_dependencies(&main_files, &module_to_info)?;

    Ok(required_modules)
}

/// Build a mapping from module names to ModuleInfo from all glob patterns
fn build_module_mapping(globs: &[String]) -> Result<HashMap<String, ModuleInfo>> {
    let results: Result<Vec<HashMap<String, ModuleInfo>>> = globs
        .par_iter()
        .map(|pattern| {
            let files = get_files_from_glob(pattern)?;
            let mut mapping = HashMap::new();

            for file in files {
                if let Ok(module_info) = extract_info_from_file(&file) {
                    mapping.insert(module_info.name.clone(), module_info);
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
    module_to_info: &HashMap<String, ModuleInfo>,
) -> Result<Vec<ModuleInfo>> {
    let mut visited_modules = std::collections::HashSet::new();
    let mut modules_to_process = Vec::new();
    let mut all_required_modules = Vec::new();

    // Start with main files - we need to find their module names first
    for file_path in main_files {
        if let Ok(module_info) = extract_info_from_file(file_path) {
            modules_to_process.push(module_info);
        }
    }

    while let Some(module) = modules_to_process.pop() {
        if visited_modules.contains(&module.name) {
            continue;
        }

        visited_modules.insert(module.name.clone());
        all_required_modules.push(module.clone());

        // For each import, find the corresponding module and add it to processing queue
        for import in &module.imports {
            if let Some(dependency_module) = module_to_info.get(import) {
                if !visited_modules.contains(&dependency_module.name) {
                    modules_to_process.push(dependency_module.clone());
                }
            }
        }
    }

    Ok(all_required_modules)
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
