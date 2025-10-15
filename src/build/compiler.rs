use anyhow::{Context, Result};
use colored::Colorize;
use std::io::BufRead;
use std::path::PathBuf;
use std::process::{self, Command};
use sysinfo::System;

/// Execute the purs compiler with streaming output
pub fn execute_compiler(
    sources: &[String],
    output_dir: &PathBuf,
    compiler_args: Vec<String>,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("{} Running purs compiler...", "→".cyan());
    }

    let total_memory = get_total_memory();

    // Build the purs compiler command
    let mut command = Command::new("purs");
    command.arg("compile");

    // Use the config output directory to share workspace output
    command.arg("--output");
    command.arg(output_dir.to_string_lossy().to_string());

    // Add RTS arguments when memory is available for it.
    // Helps with compiler performance.
    if total_memory > 31 {
        command.args(["+RTS", "-A256m", "-n16m", "-RTS"]);
    } else if total_memory > 15 {
        command.args(["+RTS", "-A128m", "-n8m", "-RTS"]);
    }

    command.args(compiler_args);

    // Add all source globs as arguments
    command.arg("--");

    command.args(sources);

    // Run the compiler with streaming output
    let mut child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start purs compiler")?;

    // Stream stdout and stderr concurrently using threads
    let stdout_thread = if let Some(stdout) = child.stdout.take() {
        let stdout_reader = std::io::BufReader::new(stdout);
        Some(std::thread::spawn(move || {
            for line in stdout_reader.lines() {
                if let Ok(line) = line {
                    println!("{}", line);
                }
            }
        }))
    } else {
        None
    };

    let stderr_thread = if let Some(stderr) = child.stderr.take() {
        let stderr_reader = std::io::BufReader::new(stderr);
        Some(std::thread::spawn(move || {
            for line in stderr_reader.lines() {
                if let Ok(line) = line {
                    eprintln!("{}", line);
                }
            }
        }))
    } else {
        None
    };

    // Wait for output threads to finish
    if let Some(stdout_thread) = stdout_thread {
        stdout_thread.join().unwrap();
    }
    if let Some(stderr_thread) = stderr_thread {
        stderr_thread.join().unwrap();
    }

    // Wait for completion
    let status = child.wait().context("Failed to wait for purs compiler")?;

    if !status.success() {
        eprintln!("❌ Compilation failed");
        process::exit(1);
    }
    if verbose {
        println!("  Compiled {} source files", sources.len());
    }

    Ok(())
}

fn get_total_memory() -> u64 {
    let mut sys = System::new_all();
    sys.refresh_all();
    sys.total_memory() / 1024 / 1024 / 1024
}
