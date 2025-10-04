use anyhow::{Context, Result};
use colored::Colorize;

use crate::registry::PackageQuery;

pub fn execute(
    query: &PackageQuery,
    package_name: &str,
    show_deps: bool,
    show_transitive: bool,
    show_reverse: bool,
) -> Result<()> {
    let package = query.get(package_name).context(format!(
        "Package '{}' not found in package set",
        package_name
    ))?;

    println!("\n{} {}\n", "📦".bold(), package.name.bright_cyan().bold());
    println!(
        "  {} {}",
        "Version:".dimmed(),
        package.package.version.green()
    );
    println!("  {} {}", "Repo:".dimmed(), package.package.repo);
    println!(
        "  {} {}",
        "Dependencies:".dimmed(),
        package.dep_count().to_string().yellow()
    );

    if show_deps || show_transitive {
        let deps = if show_transitive {
            query.get_transitive_dependencies(package_name)?
        } else {
            query.get_dependencies(package_name)?
        };

        let label = if show_transitive {
            "Transitive Dependencies"
        } else {
            "Direct Dependencies"
        };

        println!("\n{} ({}):", label.bold(), deps.len());
        if deps.is_empty() {
            println!("  {}", "None".dimmed());
        } else {
            for dep in deps.iter() {
                println!(
                    "  {} {} {}",
                    "→".cyan(),
                    dep.name,
                    format!("({})", dep.package.version).dimmed()
                );
            }
        }
    }

    if show_reverse {
        let dependents = query.get_dependents(package_name);
        println!(
            "\n{} ({}):",
            "Reverse Dependencies".bold(),
            dependents.len()
        );
        if dependents.is_empty() {
            println!("  {}", "None".dimmed());
        } else {
            for dep in dependents.iter() {
                println!("  {} {}", "←".cyan(), dep.name);
            }
        }
    }

    println!();
    Ok(())
}
