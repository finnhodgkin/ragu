use anyhow::Result;
use colored::Colorize;

use crate::registry::PackageQuery;

pub fn execute(query: &PackageQuery) -> Result<()> {
    let stats = query.stats();

    println!("\n{} Package Set Statistics\n", "📊".bold());
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
    println!(
        "  {} {}",
        "Max dependencies:".dimmed(),
        stats.max_dependencies
    );
    println!(
        "  {} {}",
        "Min dependencies:".dimmed(),
        stats.min_dependencies
    );
    println!(
        "  {} {}",
        "Packages with no deps:".dimmed(),
        stats.packages_with_no_deps
    );

    // Show packages with most dependencies
    let mut packages: Vec<_> = query.filter(|_| true).into_iter().collect();
    packages.sort_by(|a, b| b.dep_count().cmp(&a.dep_count()));

    println!("\n{} Top packages by dependencies:\n", "📈".bold());
    for pkg in packages.iter().take(10) {
        println!(
            "  {} {} {}",
            format!("{:2}", pkg.dep_count()).yellow(),
            "→".dimmed(),
            pkg.name().0
        );
    }

    // Show packages with most dependents
    let mut packages_with_dependents: Vec<_> = query.get_packages_with_dependents_count();
    packages_with_dependents.sort_by(|a, b| b.1.cmp(&a.1));

    println!("\n{} Top packages by dependents:\n", "📊".bold());
    for (pkg, dependents_count) in packages_with_dependents.iter().take(25) {
        println!(
            "  {} {} {}",
            format!("{:2}", dependents_count).green(),
            "←".dimmed(),
            pkg.name().0
        );
    }

    println!();
    Ok(())
}
