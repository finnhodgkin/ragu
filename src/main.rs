mod build;
mod cli;
mod config;
mod imports;
mod install;
mod modules;
mod registry;
mod sources;
mod workspace;

use clap::Parser;

use cli::{execute_command, Cli};

fn main() {
    let cli = Cli::parse();
    if let Err(e) = execute_command(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
