mod build;
mod cache;
mod cli;
mod config;
mod imports;
mod init;
mod install;
mod modules;
mod package_info;
mod package_sets;
mod print_output;
mod registry;
mod run;
mod sources;
mod src_as_sources;
mod test;
mod workspace;

use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = cli::Cli::parse();
    if let Err(e) = cli::execute_command::execute(cli).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
