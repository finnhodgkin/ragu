use tokio::process::Command;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::{build::compiler::execute_compiler, install::install_all_dependencies};

pub async fn execute(skip_compilation: bool, verbose: bool) -> Result<()> {
    let config = crate::config::load_config_cwd()?;

    let message = if skip_compilation {
        "Quick test".dimmed()
    } else {
        "Test".dimmed()
    };
    println!("{}", message);
    println!();

    if !skip_compilation {
        let package_set = config.package_set().await?;
        install_all_dependencies(&config, &package_set, true).await?;
        let sources = crate::sources::generate_sources(&config, None, false, false, verbose).await?;
        let mut all_sources = sources
            .dependency_globs
            .iter()
            .map(|g| g.glob_pattern.clone())
            .collect::<Vec<String>>();

        all_sources.push(sources.main_sources.clone());
        all_sources.push(TEST_SOURCES.to_string());

        execute_compiler(
            &all_sources,
            &config.output_dir(),
            &config.workspace_root,
            vec![],
            &config.workspace.psa_options,
            false,
            verbose,
        ).await?;
    }

    let output_dir = config.output_dir();
    let test_config = config
        .package
        .test
        .context("No main test package. Add a package.test.")?;

    let main_test_package = test_config.main.clone();
    let test_file = output_dir.join(format!("{}/index.js", main_test_package));
    let mut test_process = Command::new("node")
        .arg("-e")
        .arg(format!(
            "const module = require('{}'); module.main();",
            test_file.to_string_lossy()
        ))
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    let test_result = test_process.wait().await?;
    if !test_result.success() {
        println!();
        eprintln!("❌ Tests failed");
        std::process::exit(1);
    } else {
        println!();
        println!("{}", "✓ Tests passed".green());
    }

    Ok(())
}

pub const TEST_SOURCES: &str = "./test/**/*.purs";
