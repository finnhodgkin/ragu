use anyhow::Result;
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

use crate::config::load_config_cwd;
use crate::install::cache::GlobalPackageCache;
use crate::registry::{clear_cache, clear_cache_for_tag, clear_tags_cache, get_cache_dir};

pub fn info() -> Result<()> {
    let cache_dir = get_cache_dir()?;

    println!("\nCache Information\n");
    println!("  {} {}", "Location:".dimmed(), cache_dir.display());

    if !cache_dir.exists() {
        println!(
            "  {} {}",
            "Status:".dimmed(),
            "Empty (not created yet)".yellow()
        );
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

    // Check for cached packages
    let packages_dir = cache_dir.join("packages");
    if packages_dir.exists() {
        let package_entries = fs::read_dir(&packages_dir)?;
        let mut package_count = 0;
        let mut package_size = 0u64;

        for entry in package_entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    package_count += 1;
                    // Calculate directory size
                    let dir_size = calculate_directory_size(&entry.path()).unwrap_or(0);
                    package_size += dir_size;
                }
            }
        }

        if package_count > 0 {
            println!(
                "  {} {}",
                "Cached packages:".dimmed(),
                package_count.to_string().green()
            );
            total_size += package_size;
        }
    }

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

pub fn clear(also_clear_output: bool) -> Result<()> {
    println!("\nClearing cache...");

    // Clear package set cache
    clear_cache()?;
    clear_tags_cache()?;

    // Clear global package cache
    let package_cache = GlobalPackageCache::new()?;
    package_cache.clear_all()?;

    let config = load_config_cwd()?;

    if also_clear_output {
        // clear .spago and output directories
        fs::remove_dir_all(config.spago_dir())?;
        fs::remove_dir_all(config.output_dir())?;
        println!("Output and .spago directories also cleared");
    }

    println!("Cache cleared (package sets and packages)");
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

/// Calculate the total size of a directory recursively
fn calculate_directory_size(path: &std::path::Path) -> Result<u64> {
    let mut total_size = 0u64;

    if path.is_file() {
        return Ok(std::fs::metadata(path)?.len());
    }

    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            if entry_path.is_dir() {
                total_size += calculate_directory_size(&entry_path)?;
            } else {
                total_size += std::fs::metadata(&entry_path)?.len();
            }
        }
    }

    Ok(total_size)
}
