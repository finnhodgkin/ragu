use anyhow::Result;
use colored::Colorize;

use crate::cli::{CacheAction, Cli, Command};
use crate::registry::{PackageName, PackageQuery};
use crate::{
    cache, config, imports, init, install, package_info, package_sets, print_output, run,
    src_as_sources, test, workspace,
};

use super::execution_context::ExecutionContext;

/// Execute the CLI command
pub async fn execute(cli: Cli) -> Result<()> {
    if cli.verbose {
        println!("{} Verbose mode enabled\n", "→".cyan());
    }

    match cli.command {
        Command::List { all } => package_sets::list::execute(all, cli.force_refresh).await,
        Command::Info {
            package,
            deps,
            transitive,
            reverse,
            only_workspace,
        } => {
            let ctx = ExecutionContext::load().await?;
            let query = PackageQuery::new(&ctx.package_set);
            package_info::info::execute(
                &query,
                &PackageName::new(&package),
                deps,
                transitive,
                reverse,
                only_workspace,
            )
        }
        Command::Search { query, details } => {
            let ctx = ExecutionContext::load().await?;
            let pkg_query = PackageQuery::new(&ctx.package_set);
            package_info::search::execute(&pkg_query, &query, details)
        }
        Command::Install { packages } => {
            let ctx = ExecutionContext::load().await?;
            install::command::execute(&packages, &ctx.package_set, cli.verbose).await
        }
        Command::Uninstall { packages } => {
            let ctx = ExecutionContext::load().await?;
            install::uninstall::execute(
                packages.iter().map(|p| PackageName::new(p)).collect(),
                &ctx.package_set,
                cli.verbose,
            )
            .await
        }
        Command::Build {
            watch,
            clear,
            exclude_test_deps,
            quick_build,
            compiler_args,
        } => {
            if quick_build {
                src_as_sources::execute(
                    !exclude_test_deps,
                    true,
                    compiler_args,
                    cli.include_rts_stats,
                    cli.verbose,
                )
                .await
            } else {
                crate::build::execute(
                    watch,
                    clear,
                    !exclude_test_deps,
                    compiler_args,
                    cli.include_rts_stats,
                    cli.verbose,
                )
                .await
            }
        }
        Command::OutputDir => print_output::execute(),
        Command::Test { quick_test } => test::execute(quick_test, cli.verbose).await,
        Command::Run {
            module,
            quick_run,
            node_args,
        } => run::execute(module, quick_run, cli.verbose, node_args).await,
        Command::Sources { quick_sources } => {
            if quick_sources {
                src_as_sources::execute(false, false, vec![], false, cli.verbose).await
            } else {
                crate::sources::execute_sources(cli.verbose).await
            }
        }
        Command::Cache { action } => match action {
            CacheAction::Info => cache::info().await,
            CacheAction::Clear { all } => cache::clear(all).await,
        },
        Command::Stats => {
            let ctx = ExecutionContext::load().await?;
            let query = PackageQuery::new(&ctx.package_set);
            package_sets::stats::execute(&query)
        }
        Command::Init {
            name,
            nested_package,
        } => init::execute(name, nested_package).await,
        Command::Validate => config::run_validate::execute(cli.verbose).await,
        Command::Modules {
            group_by_package,
            package,
            names_only,
        } => {
            let ctx = ExecutionContext::load().await?;

            // Generate sources
            let sources = crate::sources::generate_sources(
                &ctx.config,
                Some(ctx.package_set),
                false,
                false,
                cli.verbose,
            )
            .await?;

            // Execute modules command
            let options = crate::modules::ModulesOptions {
                group_by_package,
                package_filter: package,
                names_only,
            };

            crate::modules::execute_modules_command(&ctx.config, &sources, options)
        }
        Command::Imports { location, package } => {
            imports::execute(location, package, cli.verbose).await
        }
        Command::Workspace => workspace::execute_local_packages().await,
        Command::CircularDeps => workspace::check_circular_dependencies().await,
        Command::CheckDeps {
            package,
            commands_only,
            broken_only,
            fix,
        } => workspace::check_deps(package, commands_only, broken_only, fix).await,
    }
}
