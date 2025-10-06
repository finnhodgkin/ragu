use anyhow::Result;
use colored::Colorize;

use crate::registry::PackageQuery;

pub fn execute(query: &PackageQuery, search_query: &str, show_details: bool) -> Result<()> {
    let results = query.search(search_query);

    println!(
        "\n{} Found {} package(s) matching '{}'\n",
        "🔍".bold(),
        results.len().to_string().yellow().bold(),
        search_query.cyan()
    );

    if results.is_empty() {
        println!("  {}", "No packages found".dimmed());
        return Ok(());
    }

    for result in results.iter().take(50) {
        if show_details {
            println!(
                "  {} {}",
                "→".green(),
                result.name().0.bright_white().bold()
            );
            println!(
                "    {} {}",
                "Version:".dimmed(),
                result.version().unwrap_or(&"Local".to_string())
            );
            println!("    {} {}", "Dependencies:".dimmed(), result.dep_count());
            println!();
        } else {
            println!(
                "  {} {} {} ({} deps)",
                "→".green(),
                result.name().0,
                result.version().unwrap_or(&"Local".to_string()).dimmed(),
                result.dep_count()
            );
        }
    }

    if results.len() > 50 {
        println!(
            "  {} and {} more... (use {} for details)",
            "...".dimmed(),
            results.len() - 50,
            "--details".cyan()
        );
    }

    println!();
    Ok(())
}
