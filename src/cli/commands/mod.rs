mod cache;
mod info;
mod install;
mod list;
mod search;
mod stats;
mod uninstall;
mod validate;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::cli::{CacheAction, Cli, Command};
use crate::registry::{PackageName, PackageQuery};
use crate::{imports, workspace};

/// Execute the CLI command
pub fn execute_command(cli: Cli) -> Result<()> {
    if cli.verbose {
        println!("{} Verbose mode enabled\n", "→".cyan());
    }

    match cli.command {
        Command::List { all } => list::execute(all, cli.force_refresh),
        Command::Info {
            package,
            deps,
            transitive,
            reverse,
        } => {
            let config = crate::config::load_config_cwd()
                .context("Failed to load spago.yaml configuration")?;
            let package_set = config.package_set()?;
            let query = PackageQuery::new(&package_set);
            info::execute(
                &query,
                &PackageName::new(&package),
                deps,
                transitive,
                reverse,
            )
        }
        Command::Search { query, details } => {
            let config = crate::config::load_config_cwd()
                .context("Failed to load spago.yaml configuration")?;
            let package_set = config.package_set()?;
            let pkg_query = PackageQuery::new(&package_set);
            search::execute(&pkg_query, &query, details)
        }
        Command::Install { packages } => {
            let config = crate::config::load_config_cwd()
                .context("Failed to load spago.yaml configuration")?;
            let package_set = config.package_set()?;
            tokio::runtime::Runtime::new()?.block_on(install::execute(
                &packages,
                &package_set,
                cli.verbose,
            ))
        }
        Command::Uninstall { packages } => {
            let config = crate::config::load_config_cwd()
                .context("Failed to load spago.yaml configuration")?;
            let package_set = config.package_set()?;
            tokio::runtime::Runtime::new()?.block_on(uninstall::execute(
                packages.iter().map(|p| PackageName::new(p)).collect(),
                &package_set,
                cli.verbose,
            ))
        }
        Command::Build { watch, clear } => tokio::runtime::Runtime::new()?
            .block_on(crate::build::execute(watch, clear, cli.verbose)),
        Command::Sources => crate::sources::execute_sources(cli.verbose),
        Command::Cache { action } => match action {
            CacheAction::Info => cache::info(),
            CacheAction::Clear { all } => cache::clear(all),
        },
        Command::Stats => {
            // Load spago.yaml configuration
            let config = crate::config::load_config_cwd()
                .context("Failed to load spago.yaml configuration")?;
            let package_set = config.package_set()?;
            let query = PackageQuery::new(&package_set);
            stats::execute(&query)
        }
        Command::Init { name } => {
            println!("{} Init command not yet implemented", "⚠".yellow().bold());
            println!("  Name: {:?}", name);
            Ok(())
        }
        Command::Validate { path } => validate::execute(path, cli.verbose),
        Command::Modules {
            group_by_package,
            package,
            names_only,
        } => {
            // Load spago.yaml configuration
            let config = crate::config::load_config_cwd()
                .context("Failed to load spago.yaml configuration")?;

            let package_set = config.package_set()?;

            // Generate sources
            let sources =
                crate::sources::generate_sources(&config, Some(package_set), false, cli.verbose)?;

            // Execute modules command
            let options = crate::modules::ModulesOptions {
                group_by_package,
                package_filter: package,
                names_only,
            };

            crate::modules::execute_modules_command(&config, &sources, options)
        }
        Command::Imports => imports::execute(cli.verbose),
        Command::Workspace => workspace::execute_local_packages(),
        Command::CircularDeps => workspace::check_circular_dependencies(),
        Command::CheckDeps {
            package,
            commands_only,
            broken_only,
            fix,
        } => workspace::check_deps(package, commands_only, broken_only, fix),
    }
}
