mod cli;
mod registry;

use anyhow::Result;
use clap::Parser;

use cli::{execute_command, Cli};

fn main() -> Result<()> {
    let cli = Cli::parse();
    execute_command(cli)
}
