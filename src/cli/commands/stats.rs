use anyhow::Result;
use colored::Colorize;

use crate::registry::PackageQuery;

pub fn execute(query: &PackageQuery, tag: &str) -> Result<()> {
    let stats = query.stats();

    println!("\n{} Package Set Statistics\n", "📊".bold());
    println!("  {} {}", "Tag:".dimmed(), tag.cyan());
    println!();
    println!(
        "  {} {}",
        "Total packages:".dimmed(),
        stats.total_packages.to_string().green().bold()
    );
    println!(
        "  {} {}",
        "Total dependencies:".dimmed(),
        stats.total_dependencies.to_string().yellow()
    );
    println!(
        "  {} {:.2}",
        "Average dependencies:".dimmed(),
        stats.avg_dependencies
    );
    println!("  {} {}", "Max dependencies:".dimmed(), stats.max_dependencies);
    println!("  {} {}", "Min dependencies:".dimmed(), stats.min_dependencies);
    println!(
        "  {} {}",
        "Packages with no deps:".dimmed(),
        stats.packages_with_no_deps
    );

    // Show packages with most dependencies
    let mut packages: Vec<_> = query
        .filter(|_, _| true)
        .into_iter()
        .collect();
    packages.sort_by(|a, b| b.dep_count().cmp(&a.dep_count()));

    println!("\n{} Top packages by dependencies:\n", "📈".bold());
    for pkg in packages.iter().take(10) {
        println!(
            "  {} {} {}",
            format!("{:2}", pkg.dep_count()).yellow(),
            "→".dimmed(),
            pkg.name
        );
    }

    println!();
    Ok(())
}

