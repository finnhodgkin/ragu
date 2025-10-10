use std::process::Command;

use anyhow::Result;

use crate::{build::compiler::execute_compiler, install::install_all_dependencies};

pub async fn execute(module: Option<String>, skip_compilation: bool, verbose: bool) -> Result<()> {
    let config = crate::config::load_config_cwd()?;
    if !skip_compilation {
        let package_set = config.package_set()?;
        install_all_dependencies(&config, &package_set, false).await?;
        let sources = crate::sources::generate_sources(&config, None, false, verbose)?;
        let mut all_sources = sources
            .dependency_globs
            .iter()
            .map(|g| g.glob_pattern.clone())
            .collect::<Vec<String>>();

        all_sources.push(sources.main_sources.clone());

        execute_compiler(&all_sources, &config.output_dir(), verbose)?;
    }

    let output_dir = config.output_dir();
    let test_file = output_dir.join(format!("{}/index.js", module.unwrap_or("Main".to_string())));
    let mut run_process = Command::new("node")
        .arg("-e")
        .arg(format!(
            "const module = require('{}'); module.main();",
            test_file.to_string_lossy()
        ))
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    run_process.wait()?;
    Ok(())
}
