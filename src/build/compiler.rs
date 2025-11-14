use anyhow::{Context, Result};
use colored::Colorize;
use std::io::BufRead;
use std::path::PathBuf;
use std::process::{self, Command};
use sysinfo::System;

use crate::build::run_from_root::{
    map_diagnostic_paths_from_output_to_cwd, map_sources_to_output_dir,
};
use crate::config::PsaOptionsConfig;

fn compiler_command(psa_options: &Option<PsaOptionsConfig>) -> Command {
    let psa_available = which::which("psa").is_ok();
    match psa_options {
        Some(options) if psa_available => {
            let mut command = Command::new("psa");
            if options.verbose_stats {
                command.arg("--verbose-stats");
            }
            if options.verbose_warnings {
                command.arg("--verbose-warnings");
            }
            if options.censor_warnings {
                command.arg("--censor-warnings");
            }
            if options.censor_lib {
                command.arg("--censor-lib");
            }
            if options.censor_src {
                command.arg("--censor-src");
            }
            if options.censor_codes.len() > 0 {
                command.arg(format!("--censor-codes={}", options.censor_codes.join(",")));
            }
            if options.filter_codes.len() > 0 {
                command.arg(format!("--filter-codes={}", options.filter_codes.join(",")));
            }
            if options.no_colors {
                command.arg("--no-colors");
            }
            if options.no_source {
                command.arg("--no-source");
            }
            if options.strict {
                command.arg("--strict");
            }
            if options.stash {
                command.arg("--stash");
            }
            if options.stash_file.is_some() {
                command.arg("--stash-file");
                command.arg(options.stash_file.clone().unwrap());
            }
            command
        }
        _ => {
            let mut command = Command::new("purs");
            command.arg("compile");
            command
        }
    }
}

/// Execute the purs compiler with streaming output
pub fn execute_compiler(
    sources: &[String],
    output_dir: &PathBuf,
    compiler_args: Vec<String>,
    psa_options: &Option<PsaOptionsConfig>,
    include_rts_stats: bool,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("{} Running purs compiler...", "→".cyan());
    }

    let total_memory = get_total_memory();

    // Build the purs compiler command
    let mut command: Command = compiler_command(psa_options);

    // Use the config output directory to share workspace output
    command.arg("--output");
    command.arg(output_dir.to_string_lossy().to_string());

    // Add RTS arguments when memory is available for it.
    // Helps with compiler performance.
    if total_memory > 15 {
        let mut rts_args = Vec::new();

        // Set memory parameters based on available RAM
        if total_memory > 31 {
            rts_args.extend(["-A256m", "-n16m"]);
        } else if total_memory > 15 {
            rts_args.extend(["-A128m", "-n8m"]);
        }

        if include_rts_stats {
            rts_args.push("-s");
        }

        if rts_args.len() > 0 {
            rts_args.insert(0, "+RTS");
            rts_args.push("-RTS");
            command.args(rts_args);
        }
    }

    command.args(compiler_args);

    // Add all source globs as arguments
    command.arg("--");

    let relative_sources = map_sources_to_output_dir(sources, output_dir)?;

    command.args(relative_sources);

    // Check if we're using psa (which flips stdout/stderr)
    let using_psa = psa_options.is_some() && which::which("psa").is_ok();

    // Run the compiler with streaming output
    let mut child = command
        .current_dir(output_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to start purs compiler")?;

    // Stream stdout and stderr concurrently using threads
    // Note: psa flips stdout/stderr, so we need to swap them when using psa
    let output_dir_clone = output_dir.clone();
    let stdout_thread = if let Some(stdout) = child.stdout.take() {
        let stdout_reader = std::io::BufReader::new(stdout);
        let use_stderr = using_psa; // psa outputs to stdout what should go to stderr
        let output_dir_for_thread = output_dir_clone.clone();
        Some(std::thread::spawn(move || {
            for line in stdout_reader.lines() {
                if let Ok(line) = line {
                    // Map diagnostic paths from output-relative to CWD-relative
                    let mapped_line =
                        map_diagnostic_paths_from_output_to_cwd(&line, &output_dir_for_thread)
                            .unwrap_or_else(|_| line.clone());
                    if use_stderr {
                        eprintln!("{}", mapped_line);
                    } else {
                        println!("{}", mapped_line);
                    }
                }
            }
        }))
    } else {
        None
    };

    let stderr_thread = if let Some(stderr) = child.stderr.take() {
        let stderr_reader = std::io::BufReader::new(stderr);
        let use_stdout = using_psa; // psa outputs to stderr what should go to stdout
        let output_dir_for_thread = output_dir_clone.clone();
        Some(std::thread::spawn(move || {
            for line in stderr_reader.lines() {
                if let Ok(line) = line {
                    // Map diagnostic paths from output-relative to CWD-relative
                    let mapped_line =
                        map_diagnostic_paths_from_output_to_cwd(&line, &output_dir_for_thread)
                            .unwrap_or_else(|_| line.clone());
                    if use_stdout {
                        println!("{}", mapped_line);
                    } else {
                        eprintln!("{}", mapped_line);
                    }
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
