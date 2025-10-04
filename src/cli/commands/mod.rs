mod build;
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
use crate::registry::{
    get_package_set, list_available_tags, list_available_tags_with_options, PackageQuery,
};

/// Execute the CLI command
pub fn execute_command(cli: Cli) -> Result<()> {
    if cli.verbose {
        println!("{} Verbose mode enabled\n", "→".cyan());
    }

    // Get the tag (either from CLI or latest)
    let tag = if let Some(tag) = &cli.tag {
        if cli.verbose {
            println!("{} Using specified tag: {}", "→".cyan(), tag);
        }
        tag.clone()
    } else {
        match &cli.command {
            Command::List { .. } => {
                // List command doesn't need a package set
                String::new()
            }
            Command::Cache { .. } => {
                // Cache commands might not need a package set
                String::new()
            }
            _ => {
                if cli.verbose {
                    println!("{} Fetching latest package set tag...", "→".cyan());
                }
                // Get tags respecting force_refresh
                let tags = if cli.force_refresh {
                    list_available_tags_with_options(true, None)?
                } else {
                    list_available_tags()?
                };
                tags.first()
                    .cloned()
                    .context("No tags available in the package-sets repository")?
            }
        }
    };

    match cli.command {
        Command::List { all } => list::execute(all, cli.force_refresh),
        Command::Info {
            package,
            deps,
            transitive,
            reverse,
        } => {
            let package_set = get_package_set(&tag, cli.force_refresh)?;
            let query = PackageQuery::new(&package_set);
            info::execute(&query, &package, deps, transitive, reverse)
        }
        Command::Search { query, details } => {
            let package_set = get_package_set(&tag, cli.force_refresh)?;
            let pkg_query = PackageQuery::new(&package_set);
            search::execute(&pkg_query, &query, details)
        }
        Command::Install { packages, no_deps } => {
            let package_set = get_package_set(&tag, cli.force_refresh)?;
            tokio::runtime::Runtime::new()?.block_on(install::execute(
                &packages,
                no_deps,
                &package_set,
                cli.verbose,
            ))
        }
        Command::Uninstall { packages } => {
            let package_set = get_package_set(&tag, cli.force_refresh)?;
            tokio::runtime::Runtime::new()?.block_on(uninstall::execute(
                &packages,
                &package_set,
                cli.verbose,
            ))
        }
        Command::Build { watch, clear } => build::execute(watch, clear, cli.verbose),
        Command::Sources => build::execute_sources(cli.verbose),
        Command::Cache { action } => match action {
            CacheAction::Info => cache::info(),
            CacheAction::Clear => cache::clear(),
            CacheAction::Remove { tag } => cache::remove(&tag),
        },
        Command::Stats => {
            let package_set = get_package_set(&tag, cli.force_refresh)?;
            let query = PackageQuery::new(&package_set);
            stats::execute(&query, &tag)
        }
        Command::Init { name } => {
            println!("{} Init command not yet implemented", "⚠".yellow().bold());
            println!("  Name: {:?}", name);
            Ok(())
        }
        Command::Validate { path } => validate::execute(path, cli.force_refresh, cli.verbose),
    }
}
