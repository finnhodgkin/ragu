mod build;
mod cli;
mod config;
mod install;
mod registry;
mod sources;

use clap::Parser;

use cli::{execute_command, Cli};

fn main() {
    let cli = Cli::parse();
    if let Err(e) = execute_command(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
