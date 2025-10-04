use anyhow::Result;
use colored::Colorize;
use std::fs;

use crate::registry::{clear_cache, clear_cache_for_tag, clear_tags_cache, get_cache_dir};

pub fn info() -> Result<()> {
    let cache_dir = get_cache_dir()?;

    println!("\nCache Information\n");
    println!("  {} {}", "Location:".dimmed(), cache_dir.display());

    if !cache_dir.exists() {
        println!("  {} {}", "Status:".dimmed(), "Empty (not created yet)".yellow());
        return Ok(());
    }

    // Count cached package sets
    let entries = fs::read_dir(&cache_dir)?;
    let mut total_size = 0u64;
    let mut file_count = 0;

    for entry in entries.flatten() {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() {
                total_size += metadata.len();
                file_count += 1;
            }
        }
    }

    println!(
        "  {} {}",
        "Cached package sets:".dimmed(),
        file_count.to_string().green()
    );

    let size_kb = total_size as f64 / 1024.0;
    let size_mb = size_kb / 1024.0;
    let size_str = if size_mb > 1.0 {
        format!("{:.2} MB", size_mb)
    } else {
        format!("{:.2} KB", size_kb)
    };

    println!("  {} {}", "Total size:".dimmed(), size_str.yellow());

    println!();
    Ok(())
}

pub fn clear() -> Result<()> {
    println!("\nClearing cache...");
    clear_cache()?;
    clear_tags_cache()?;
    println!("Cache cleared");
    println!();
    Ok(())
}

pub fn remove(tag: &str) -> Result<()> {
    println!("\nRemoving cache for tag: {}", tag);
    clear_cache_for_tag(tag)?;
    println!("Cache removed for tag: {}", tag);
    println!();
    Ok(())
}

