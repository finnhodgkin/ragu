mod commands;

use clap::{Parser, Subcommand};

pub use commands::execute_command;

/// Spago Rust - A rust port of the popular PureScript package manager
#[derive(Parser, Debug)]
#[command(name = "spago")]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// The package set tag to use (defaults to latest)
    #[arg(short = 't', long, global = true)]
    pub tag: Option<String>,

    /// Force refresh cache (bypass cached package sets)
    #[arg(short = 'f', long, global = true)]
    pub force_refresh: bool,

    /// Verbose output
    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// List available package set versions
    List {
        /// Show all available tags (default: 20)
        #[arg(short = 'a', long)]
        all: bool,
    },

    /// Show information about a specific package
    Info {
        /// Package name to inspect
        #[arg(required = true)]
        package: String,

        /// Show dependencies
        #[arg(short = 'd', long)]
        deps: bool,

        /// Show transitive dependencies
        #[arg(short = 'T', long)]
        transitive: bool,

        /// Show reverse dependencies (packages that depend on this)
        #[arg(short = 'r', long)]
        reverse: bool,
    },

    /// Search for packages by name
    Search {
        /// Search query (partial name match)
        #[arg(required = true)]
        query: String,

        /// Show package details
        #[arg(short = 'd', long)]
        details: bool,
    },

    /// Install packages (resolves dependencies)
    #[command(alias = "i")]
    Install {
        /// Packages to install
        packages: Vec<String>,
    },

    /// Uninstall packages
    #[command(alias = "un")]
    Uninstall {
        /// Packages to uninstall
        packages: Vec<String>,
    },

    /// Build the project
    Build {
        /// Watch for changes
        #[arg(short = 'w', long)]
        watch: bool,

        /// Clear output directory before building
        #[arg(long)]
        clear: bool,
    },

    /// Output source file globs for piping to other tools
    Sources,

    /// Manage cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },

    /// Show package set statistics
    Stats,

    /// Initialize a new Spago project
    Init {
        /// Project name
        #[arg(short = 'n', long)]
        name: Option<String>,
    },

    /// Validate spago.yaml configuration
    Validate {
        /// Path to spago.yaml (defaults to ./spago.yaml)
        #[arg(short = 'p', long)]
        path: Option<String>,
    },

    /// List all modules in the project and dependencies
    Modules {
        /// Show modules grouped by package
        #[arg(short = 'g', long)]
        group_by_package: bool,

        /// Filter modules by package name
        #[arg(short = 'p', long)]
        package: Option<String>,

        /// Show only module names (no additional info)
        #[arg(short = 'n', long)]
        names_only: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum CacheAction {
    /// Show cache location and size
    Info,

    /// Clear all cached package sets
    Clear {
        /// Clear the .spago and output directories as well
        #[arg(short = 'a', long)]
        all: bool,
    },

    /// Clear cache for a specific tag
    Remove {
        /// Tag to remove from cache
        tag: String,
    },
}
