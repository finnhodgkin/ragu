pub mod compiler;
pub mod run_from_root;

use anyhow::{Context, Result};
use colored::Colorize;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::sources::BuildSources;
use crate::{install::install_all_dependencies, test::TEST_SOURCES};

/// Execute the build command
pub async fn execute(
    watch: bool,
    clear: bool,
    test: bool,
    compiler_args: Vec<String>,
    include_rts_stats: bool,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("{} Build command executing", "→".cyan());
        println!("  Watch: {}", watch);
        println!("  Clear: {}", clear);
    }

    // Load spago.yaml configuration
    let config =
        crate::config::load_config_cwd().context("Failed to load spago.yaml configuration")?;

    let package_set = config.package_set().await?;

    install_all_dependencies(&config, &package_set, test).await?;

    // Generate source globs for dependencies
    let sources =
        crate::sources::generate_sources(&config, Some(package_set), false, test, verbose).await?;

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

    let mut all_sources = collect_build_sources(&sources, test);

    // Remove any sources that don't contain any .purs files
    all_sources = all_sources
        .into_par_iter()
        .filter(|source| {
            let files = glob::glob(source);
            if let Ok(files) = files {
                files.peekable().peek().is_some()
            } else {
                false
            }
        })
        .collect();

    // Execute the purs compiler
    compiler::execute_compiler(
        &all_sources,
        &config.output_dir(),
        &config.workspace_root,
        compiler_args,
        &config.workspace.psa_options,
        include_rts_stats,
        verbose,
    )
    .await?;

    println!("{} Build successful", "✓".green());

    Ok(())
}

/// Collect all source globs for the compiler from the generated build sources.
fn collect_build_sources(sources: &BuildSources, test: bool) -> Vec<String> {
    let mut all_sources: Vec<String> = sources
        .dependency_globs
        .iter()
        .map(|g| g.glob_pattern.clone())
        .collect();

    if let Some(main) = &sources.main_sources {
        all_sources.push(main.clone());
    }

    if test {
        all_sources.push(TEST_SOURCES.to_string());
    }

    all_sources
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::DependencyGlob;
    use std::path::PathBuf;

    fn dep_glob() -> DependencyGlob {
        DependencyGlob {
            package_name: "prelude".to_string(),
            glob_pattern: ".spago/prelude/src/**/*.purs".to_string(),
            local_path: PathBuf::from(".spago/prelude"),
        }
    }

    #[test]
    fn test_collect_build_sources_includes_main_sources_when_present() {
        let sources = BuildSources {
            main_sources: Some("./src/**/*.purs".to_string()),
            dependency_globs: vec![dep_glob()],
        };

        let result = collect_build_sources(&sources, false);

        assert!(result.contains(&"./src/**/*.purs".to_string()));
        assert!(result.contains(&".spago/prelude/src/**/*.purs".to_string()));
    }

    #[test]
    fn test_collect_build_sources_excludes_main_sources_when_none() {
        let sources = BuildSources {
            main_sources: None,
            dependency_globs: vec![dep_glob()],
        };

        let result = collect_build_sources(&sources, false);

        assert!(!result.contains(&"./src/**/*.purs".to_string()));
        assert!(result.contains(&".spago/prelude/src/**/*.purs".to_string()));
    }
}
