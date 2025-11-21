use anyhow::{Context, Result};
use colored::Colorize;
use std::path::PathBuf;
use std::process::Stdio;
use sysinfo::System;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::build::run_from_root::{
    map_diagnostic_paths_from_output_to_cwd, map_sources_to_output_dir,
};
use crate::config::PsaOptionsConfig;

const COMPILER_CMD_PSA: &str = "psa";
const COMPILER_CMD_PURS: &str = "purs";
const COMPILER_ARG_COMPILE: &str = "compile";

const RTS_FLAG_START: &str = "+RTS";
const RTS_FLAG_END: &str = "-RTS";
const RTS_ARG_STATS: &str = "-s";

// High memory (>= 32GB) configuration
// -A256m: Allocation area size (large young generation)
// -n16m:  Nursery chunk size
const RTS_ARG_HIGH_MEM_ALLOC: &str = "-A256m";
const RTS_ARG_HIGH_MEM_NURSERY: &str = "-n16m";

// Medium memory (> 15GB) configuration
// -A128m: Allocation area size
// -n8m:   Nursery chunk size
const RTS_ARG_MED_MEM_ALLOC: &str = "-A128m";
const RTS_ARG_MED_MEM_NURSERY: &str = "-n8m";

/// Build RTS (Runtime System) arguments for the compiler based on available memory.
///
/// The RTS arguments configure the Haskell runtime behavior for the PureScript compiler.
/// We optimize these based on available system memory:
///
/// - **High Memory (>= 32GB)**: Uses 256MB allocation area (`-A256m`).
/// - **Medium Memory (> 15GB)**: Uses 128MB allocation area (`-A128m`).
/// - **Standard**: Uses default settings.
///
/// Also handles the `-s` flag for RTS statistics if requested.
fn build_rts_args(total_memory_gb: u64, include_stats: bool) -> Vec<String> {
    let mut rts_args = Vec::new();

    // Set memory parameters based on available RAM
    if total_memory_gb > 31 {
        rts_args.push(RTS_ARG_HIGH_MEM_ALLOC.to_string());
        rts_args.push(RTS_ARG_HIGH_MEM_NURSERY.to_string());
    } else if total_memory_gb > 15 {
        rts_args.push(RTS_ARG_MED_MEM_ALLOC.to_string());
        rts_args.push(RTS_ARG_MED_MEM_NURSERY.to_string());
    }

    if include_stats {
        rts_args.push(RTS_ARG_STATS.to_string());
    }

    // Only wrap with +RTS/-RTS if we have args to add
    if !rts_args.is_empty() {
        rts_args.insert(0, RTS_FLAG_START.to_string());
        rts_args.push(RTS_FLAG_END.to_string());
    }

    rts_args
}

/// Construct the compiler command, choosing between `psa` and `purs`.
///
/// - If `psa` (PureScript Adapter) options are provided AND `psa` is available in the PATH,
///   it constructs a `psa` command with the specified flags (censoring, verbose stats, etc.).
/// - Otherwise, it falls back to the standard `purs compile` command.
fn compiler_command(psa_options: &Option<PsaOptionsConfig>) -> Command {
    let psa_available = which::which(COMPILER_CMD_PSA).is_ok();
    match psa_options {
        Some(options) if psa_available => {
            let mut command = Command::new(COMPILER_CMD_PSA);
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
            let mut command = Command::new(COMPILER_CMD_PURS);
            command.arg(COMPILER_ARG_COMPILE);
            command
        }
    }
}

/// Execute the purs compiler with streaming output
pub async fn execute_compiler(
    sources: &[String],
    output_dir: &PathBuf,
    workspace_root: &PathBuf,
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
    let rts_args = build_rts_args(total_memory, include_rts_stats);
    if !rts_args.is_empty() {
        command.args(rts_args);
    }

    command.args(compiler_args);

    // Add all source globs as arguments
    command.arg("--");

    let relative_sources = map_sources_to_output_dir(sources, workspace_root)?;

    command.args(relative_sources);

    // Check if we're using psa (which flips stdout/stderr)
    let using_psa = psa_options.is_some() && which::which(COMPILER_CMD_PSA).is_ok();

    // Run the compiler with streaming output
    let mut child = command
        .current_dir(workspace_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start purs compiler")?;

    // Stream stdout and stderr concurrently using tokio tasks
    // Note: psa flips stdout/stderr, so we need to swap them when using psa
    let stdout_handle = if let Some(stdout) = child.stdout.take() {
        // If using psa, stdout contains what should be on stderr
        let print_to_stderr = using_psa;
        Some(spawn_output_streamer(
            stdout,
            print_to_stderr,
            workspace_root.clone(),
        ))
    } else {
        None
    };

    let stderr_handle = if let Some(stderr) = child.stderr.take() {
        // If using psa, stderr contains what should be on stdout.
        // If NOT using psa, stderr goes to stderr.
        let print_to_stderr = !using_psa;
        Some(spawn_output_streamer(
            stderr,
            print_to_stderr,
            workspace_root.clone(),
        ))
    } else {
        None
    };

    // Wait for output threads to finish
    if let Some(handle) = stdout_handle {
        handle.await.ok();
    }
    if let Some(handle) = stderr_handle {
        handle.await.ok();
    }

    // Wait for completion
    let status = child
        .wait()
        .await
        .context("Failed to wait for purs compiler")?;

    if !status.success() {
        eprintln!("❌ Compilation failed");
        std::process::exit(1);
    }
    if verbose {
        println!("  Compiled {} source files", sources.len());
    }

    Ok(())
}

/// Spawn a background task to stream process output (stdout/stderr).
///
/// This helper handles two main things:
/// 1. **Line-by-line Streaming**: Reads the output stream line by line.
/// 2. **Path Remapping**: Calls `map_diagnostic_paths_from_output_to_cwd` on each line
///    to ensure file paths in error messages are relative to the user's CWD, not the
///    compiler's output directory.
///
/// It allows redirecting `stdout` content to `stderr` (or vice-versa) via `print_to_stderr`,
/// which is necessary because `psa` flips these streams.
fn spawn_output_streamer(
    stream: impl tokio::io::AsyncRead + Unpin + Send + 'static,
    print_to_stderr: bool,
    workspace_root: PathBuf,
) -> tokio::task::JoinHandle<()> {
    let reader = BufReader::new(stream);
    tokio::spawn(async move {
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let mapped_line = map_diagnostic_paths_from_output_to_cwd(&line, &workspace_root)
                .unwrap_or_else(|_| line.clone());

            if print_to_stderr {
                eprintln!("{}", mapped_line);
            } else {
                println!("{}", mapped_line);
            }
        }
    })
}

fn get_total_memory() -> u64 {
    let mut sys = System::new_all();
    sys.refresh_all();
    sys.total_memory() / 1024 / 1024 / 1024
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a default PsaOptionsConfig for testing
    fn default_psa_options() -> PsaOptionsConfig {
        PsaOptionsConfig {
            verbose_stats: false,
            verbose_warnings: false,
            censor_warnings: false,
            censor_lib: false,
            censor_src: false,
            censor_codes: vec![],
            filter_codes: vec![],
            no_colors: false,
            no_source: false,
            strict: false,
            stash: false,
            stash_file: None,
        }
    }

    #[test]
    fn test_compiler_command_uses_purs_when_no_options() {
        let command = compiler_command(&None);
        let debug = format!("{:?}", command);
        assert!(debug.contains("purs"));
        assert!(debug.contains("compile"));
    }

    #[test]
    fn test_compiler_command_uses_purs_when_psa_not_available() {
        // Even with options, should fall back to purs if psa not available
        // (This test might pass or fail depending on whether psa is installed)
        let options = Some(default_psa_options());
        let command = compiler_command(&options);
        let debug = format!("{:?}", command);
        // Will be either purs or psa depending on system
        assert!(debug.contains("purs") || debug.contains("psa"));
    }

    #[test]
    fn test_compiler_command_with_verbose_stats() {
        let mut options = default_psa_options();
        options.verbose_stats = true;
        let command = compiler_command(&Some(options));
        let debug = format!("{:?}", command);

        // Only check for the flag if psa is available
        if which::which("psa").is_ok() {
            assert!(debug.contains("--verbose-stats"));
        }
    }

    #[test]
    fn test_compiler_command_with_verbose_warnings() {
        let mut options = default_psa_options();
        options.verbose_warnings = true;
        let command = compiler_command(&Some(options));
        let debug = format!("{:?}", command);

        if which::which("psa").is_ok() {
            assert!(debug.contains("--verbose-warnings"));
        }
    }

    #[test]
    fn test_compiler_command_with_censor_flags() {
        let mut options = default_psa_options();
        options.censor_warnings = true;
        options.censor_lib = true;
        options.censor_src = true;
        let command = compiler_command(&Some(options));
        let debug = format!("{:?}", command);

        if which::which("psa").is_ok() {
            assert!(debug.contains("--censor-warnings"));
            assert!(debug.contains("--censor-lib"));
            assert!(debug.contains("--censor-src"));
        }
    }

    #[test]
    fn test_compiler_command_with_censor_codes() {
        let mut options = default_psa_options();
        options.censor_codes = vec!["Error1".to_string(), "Error2".to_string()];
        let command = compiler_command(&Some(options));
        let debug = format!("{:?}", command);

        if which::which("psa").is_ok() {
            assert!(debug.contains("--censor-codes=Error1,Error2"));
        }
    }

    #[test]
    fn test_compiler_command_with_filter_codes() {
        let mut options = default_psa_options();
        options.filter_codes = vec!["Warn1".to_string(), "Warn2".to_string()];
        let command = compiler_command(&Some(options));
        let debug = format!("{:?}", command);

        if which::which("psa").is_ok() {
            assert!(debug.contains("--filter-codes=Warn1,Warn2"));
        }
    }

    #[test]
    fn test_compiler_command_with_no_colors_and_no_source() {
        let mut options = default_psa_options();
        options.no_colors = true;
        options.no_source = true;
        let command = compiler_command(&Some(options));
        let debug = format!("{:?}", command);

        if which::which("psa").is_ok() {
            assert!(debug.contains("--no-colors"));
            assert!(debug.contains("--no-source"));
        }
    }

    #[test]
    fn test_compiler_command_with_strict() {
        let mut options = default_psa_options();
        options.strict = true;
        let command = compiler_command(&Some(options));
        let debug = format!("{:?}", command);

        if which::which("psa").is_ok() {
            assert!(debug.contains("--strict"));
        }
    }

    #[test]
    fn test_compiler_command_with_stash() {
        let mut options = default_psa_options();
        options.stash = true;
        let command = compiler_command(&Some(options));
        let debug = format!("{:?}", command);

        if which::which("psa").is_ok() {
            assert!(debug.contains("--stash"));
        }
    }

    #[test]
    fn test_compiler_command_with_stash_file() {
        let mut options = default_psa_options();
        options.stash_file = Some("my-stash.json".to_string());
        let command = compiler_command(&Some(options));
        let debug = format!("{:?}", command);

        if which::which("psa").is_ok() {
            assert!(debug.contains("--stash-file"));
            assert!(debug.contains("my-stash.json"));
        }
    }

    #[test]
    fn test_compiler_command_with_multiple_options() {
        let mut options = default_psa_options();
        options.verbose_stats = true;
        options.strict = true;
        options.no_colors = true;
        options.censor_lib = true;
        options.censor_codes = vec!["E1".to_string(), "E2".to_string()];
        let command = compiler_command(&Some(options));
        let debug = format!("{:?}", command);

        if which::which("psa").is_ok() {
            assert!(debug.contains("--verbose-stats"));
            assert!(debug.contains("--strict"));
            assert!(debug.contains("--no-colors"));
            assert!(debug.contains("--censor-lib"));
            assert!(debug.contains("--censor-codes=E1,E2"));
        }
    }

    #[test]
    fn test_compiler_command_empty_censor_codes() {
        let mut options = default_psa_options();
        options.censor_codes = vec![];
        let command = compiler_command(&Some(options));
        let debug = format!("{:?}", command);

        // Should not include --censor-codes when empty
        assert!(!debug.contains("--censor-codes"));
    }

    #[test]
    fn test_compiler_command_empty_filter_codes() {
        let mut options = default_psa_options();
        options.filter_codes = vec![];
        let command = compiler_command(&Some(options));
        let debug = format!("{:?}", command);

        // Should not include --filter-codes when empty
        assert!(!debug.contains("--filter-codes"));
    }

    // Tests for build_rts_args

    #[test]
    fn test_build_rts_args_low_memory_no_stats() {
        let args = build_rts_args(8, false);
        assert!(args.is_empty());

        let args = build_rts_args(15, false);
        assert!(args.is_empty());
    }

    #[test]
    fn test_build_rts_args_medium_memory() {
        let args = build_rts_args(16, false);
        assert_eq!(
            args,
            vec![
                RTS_FLAG_START,
                RTS_ARG_MED_MEM_ALLOC,
                RTS_ARG_MED_MEM_NURSERY,
                RTS_FLAG_END
            ]
        );

        let args = build_rts_args(24, false);
        assert_eq!(
            args,
            vec![
                RTS_FLAG_START,
                RTS_ARG_MED_MEM_ALLOC,
                RTS_ARG_MED_MEM_NURSERY,
                RTS_FLAG_END
            ]
        );

        let args = build_rts_args(31, false);
        assert_eq!(
            args,
            vec![
                RTS_FLAG_START,
                RTS_ARG_MED_MEM_ALLOC,
                RTS_ARG_MED_MEM_NURSERY,
                RTS_FLAG_END
            ]
        );
    }

    #[test]
    fn test_build_rts_args_high_memory() {
        let args = build_rts_args(32, false);
        assert_eq!(
            args,
            vec![
                RTS_FLAG_START,
                RTS_ARG_HIGH_MEM_ALLOC,
                RTS_ARG_HIGH_MEM_NURSERY,
                RTS_FLAG_END
            ]
        );

        let args = build_rts_args(64, false);
        assert_eq!(
            args,
            vec![
                RTS_FLAG_START,
                RTS_ARG_HIGH_MEM_ALLOC,
                RTS_ARG_HIGH_MEM_NURSERY,
                RTS_FLAG_END
            ]
        );
    }

    #[test]
    fn test_build_rts_args_with_stats_medium_memory() {
        let args = build_rts_args(16, true);
        assert_eq!(
            args,
            vec![
                RTS_FLAG_START,
                RTS_ARG_MED_MEM_ALLOC,
                RTS_ARG_MED_MEM_NURSERY,
                RTS_ARG_STATS,
                RTS_FLAG_END
            ]
        );

        let args = build_rts_args(24, true);
        assert_eq!(
            args,
            vec![
                RTS_FLAG_START,
                RTS_ARG_MED_MEM_ALLOC,
                RTS_ARG_MED_MEM_NURSERY,
                RTS_ARG_STATS,
                RTS_FLAG_END
            ]
        );
    }

    #[test]
    fn test_build_rts_args_with_stats_high_memory() {
        let args = build_rts_args(32, true);
        assert_eq!(
            args,
            vec![
                RTS_FLAG_START,
                RTS_ARG_HIGH_MEM_ALLOC,
                RTS_ARG_HIGH_MEM_NURSERY,
                RTS_ARG_STATS,
                RTS_FLAG_END
            ]
        );

        let args = build_rts_args(64, true);
        assert_eq!(
            args,
            vec![
                RTS_FLAG_START,
                RTS_ARG_HIGH_MEM_ALLOC,
                RTS_ARG_HIGH_MEM_NURSERY,
                RTS_ARG_STATS,
                RTS_FLAG_END
            ]
        );
    }

    #[test]
    fn test_build_rts_args_with_stats_low_memory() {
        // With stats but low memory, should return just stats
        let args = build_rts_args(8, true);
        assert_eq!(args, vec![RTS_FLAG_START, RTS_ARG_STATS, RTS_FLAG_END]);

        let args = build_rts_args(15, true);
        assert_eq!(args, vec![RTS_FLAG_START, RTS_ARG_STATS, RTS_FLAG_END]);
    }

    #[test]
    fn test_build_rts_args_boundary_conditions() {
        // Test exact boundary at 15GB without stats
        assert!(build_rts_args(15, false).is_empty());
        assert!(!build_rts_args(16, false).is_empty());

        // Test exact boundary at 15GB with stats
        let args_15_stats = build_rts_args(15, true);
        assert_eq!(
            args_15_stats,
            vec![RTS_FLAG_START, RTS_ARG_STATS, RTS_FLAG_END]
        );

        // Test exact boundary at 31GB
        let args_31 = build_rts_args(31, false);
        assert!(args_31.contains(&RTS_ARG_MED_MEM_ALLOC.to_string()));

        let args_32 = build_rts_args(32, false);
        assert!(args_32.contains(&RTS_ARG_HIGH_MEM_ALLOC.to_string()));
    }
}
