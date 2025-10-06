use anyhow::{Context, Result};
use colored::Colorize;

use crate::registry::{Package, PackageName, PackageQuery};

pub fn execute(
    query: &PackageQuery,
    package_name: &PackageName,
    show_deps: bool,
    show_transitive: bool,
    show_reverse: bool,
) -> Result<()> {
    let package_type = query.get(package_name).context(format!(
        "Package '{}' not found in package set",
        package_name.0
    ))?;

    match &package_type {
        &Package::Local(package) => {
            println!(
                "Local package: {} {}",
                package.name.0,
                package.path.display()
            );
        }
        &Package::Remote(package) => {
            println!(
                "\n{} {}\n",
                "📦".bold(),
                package.name.0.bright_cyan().bold()
            );
            println!("  {} {}", "Version:".dimmed(), package.version.green());
            println!("  {} {}", "Repo:".dimmed(), package.repo);
            println!(
                "  {} {}",
                "Dependencies:".dimmed(),
                package_type.dep_count().to_string().yellow()
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
                            dep.name().0,
                            format!("({})", dep.version().unwrap_or(&"local".to_string())).dimmed()
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
                        println!("  {} {}", "←".cyan(), dep.name().0);
                    }
                }
            }

            println!();
        }
    }

    Ok(())
}
