use anyhow::Result;
use colored::Colorize;

use crate::registry::list_available_tags_with_options;

pub fn execute(show_all: bool, force_refresh: bool) -> Result<()> {
    let tags = list_available_tags_with_options(force_refresh, None)?;

    println!("\n{} Available package sets:\n", "📋".bold());

    let display_count = if show_all { tags.len() } else { 20 };

    for (i, tag) in tags.iter().enumerate().take(display_count) {
        if i == 0 {
            println!("  {} {} {}", "→".green(), tag.bright_green().bold(), "(latest)".dimmed());
        } else {
            println!("  {} {}", "·".dimmed(), tag);
        }
    }

    if tags.len() > display_count {
        println!(
            "\n  {} {} more available (use {} to see all)",
            "...".dimmed(),
            tags.len() - display_count,
            "--all".cyan()
        );
    }

    println!("\n{} Total: {} package sets", "✓".green(), tags.len());

    Ok(())
}

