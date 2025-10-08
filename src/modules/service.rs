use anyhow::Result;
use colored::Colorize;

use crate::config::SpagoConfig;
use crate::modules::{discover_all_modules, group_modules_by_package, ModuleInfo};
use crate::sources::BuildSources;

/// Options for the modules command
#[derive(Debug)]
pub struct ModulesOptions {
    pub group_by_package: bool,
    pub package_filter: Option<String>,
    pub names_only: bool,
}

/// Execute the modules command
pub fn execute_modules_command(
    _config: &SpagoConfig,
    sources: &BuildSources,
    options: ModulesOptions,
) -> Result<()> {
    // Discover modules
    let modules = discover_all_modules(sources)?;

    // Filter by package if specified
    let filtered_modules = if let Some(ref package_filter) = options.package_filter {
        modules
            .into_iter()
            .filter(|m| m.package_name == *package_filter)
            .collect()
    } else {
        modules
    };

    // Display modules
    if options.group_by_package {
        display_modules_grouped(&filtered_modules, &options);
    } else {
        display_modules_flat(&filtered_modules, &options);
    }

    Ok(())
}

/// Display modules grouped by package
fn display_modules_grouped(modules: &[ModuleInfo], _options: &ModulesOptions) {
    let grouped = group_modules_by_package(modules);
    for (package_name, package_modules) in grouped {
        println!("{}", package_name.cyan().bold());
        for module in package_modules {
            println!("  {}", module.name);
        }
        println!();
    }
}

/// Display modules in a flat list
fn display_modules_flat(modules: &[ModuleInfo], options: &ModulesOptions) {
    for module in modules {
        if options.names_only || options.package_filter.is_some() {
            println!("{}", module.name);
        } else {
            // When showing all modules, show package name
            println!("{} [{}]", module.name, module.package_name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::{BuildSources, DependencyGlob};
    use std::path::PathBuf;

    fn create_test_sources() -> BuildSources {
        BuildSources {
            main_sources: "./src/**/*.purs".to_string(),
            dependency_globs: vec![
                DependencyGlob {
                    package_name: "prelude".to_string(),
                    glob_pattern: "./.spago/prelude/src/**/*.purs".to_string(),
                    local_path: PathBuf::from("./.spago/prelude"),
                },
                DependencyGlob {
                    package_name: "console".to_string(),
                    glob_pattern: "./.spago/console/src/**/*.purs".to_string(),
                    local_path: PathBuf::from("./.spago/console"),
                },
            ],
        }
    }

    fn create_test_modules() -> Vec<ModuleInfo> {
        vec![
            ModuleInfo {
                name: "Data.Maybe".to_string(),
                file_path: PathBuf::from("./.spago/prelude/src/Data/Maybe.purs"),
                package_name: "prelude".to_string(),
            },
            ModuleInfo {
                name: "Data.Either".to_string(),
                file_path: PathBuf::from("./.spago/prelude/src/Data/Either.purs"),
                package_name: "prelude".to_string(),
            },
            ModuleInfo {
                name: "Effect.Console".to_string(),
                file_path: PathBuf::from("./.spago/console/src/Effect/Console.purs"),
                package_name: "console".to_string(),
            },
        ]
    }

    #[test]
    fn test_display_modules_flat_all_modules() {
        let modules = create_test_modules();
        let options = ModulesOptions {
            group_by_package: false,
            package_filter: None,
            names_only: false,
        };

        // Note: In a real test, you'd want to use a proper output capture mechanism
        // For now, we'll just verify the function doesn't panic
        display_modules_flat(&modules, &options);
    }

    #[test]
    fn test_display_modules_flat_names_only() {
        let modules = create_test_modules();
        let options = ModulesOptions {
            group_by_package: false,
            package_filter: None,
            names_only: true,
        };

        display_modules_flat(&modules, &options);
    }

    #[test]
    fn test_display_modules_flat_filtered_by_package() {
        let modules = create_test_modules();
        let options = ModulesOptions {
            group_by_package: false,
            package_filter: Some("prelude".to_string()),
            names_only: false,
        };

        display_modules_flat(&modules, &options);
    }

    #[test]
    fn test_display_modules_grouped() {
        let modules = create_test_modules();
        let options = ModulesOptions {
            group_by_package: true,
            package_filter: None,
            names_only: false,
        };

        display_modules_grouped(&modules, &options);
    }

    #[test]
    fn test_display_modules_grouped_filtered() {
        let modules = create_test_modules();
        let options = ModulesOptions {
            group_by_package: true,
            package_filter: Some("prelude".to_string()),
            names_only: false,
        };

        display_modules_grouped(&modules, &options);
    }

    #[test]
    fn test_execute_modules_command_flat_display() {
        let sources = create_test_sources();
        let config = crate::config::SpagoConfig {
            package: crate::config::PackageConfig {
                name: crate::registry::PackageName::new("test-package"),
                dependencies: vec![],
                test: None,
            },
            workspace: crate::config::WorkspaceConfig::default(),
            workspace_root: PathBuf::from("."),
        };

        let options = ModulesOptions {
            group_by_package: false,
            package_filter: None,
            names_only: false,
        };

        // This test just verifies the function doesn't panic
        // In a real scenario, you'd want to mock the discover_all_modules function
        // or use dependency injection to make it testable
        let _result = execute_modules_command(&config, &sources, options);
        // We just verify the function can be called without panicking
        // The actual result depends on the file system state
    }

    #[test]
    fn test_modules_options_creation() {
        let options = ModulesOptions {
            group_by_package: true,
            package_filter: Some("prelude".to_string()),
            names_only: false,
        };

        assert!(options.group_by_package);
        assert_eq!(options.package_filter, Some("prelude".to_string()));
        assert!(!options.names_only);
    }
}
