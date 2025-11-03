use std::process::Command;

use anyhow::Result;

use crate::{build::compiler::execute_compiler, install::install_all_dependencies};

pub async fn execute(
    module: Option<String>,
    skip_compilation: bool,
    verbose: bool,
    node_args: Vec<String>,
) -> Result<()> {
    let config = crate::config::load_config_cwd()?;
    if !skip_compilation {
        let package_set = config.package_set()?;
        install_all_dependencies(&config, &package_set, false).await?;
        let sources = crate::sources::generate_sources(&config, None, false, false, verbose)?;
        let mut all_sources = sources
            .dependency_globs
            .iter()
            .map(|g| g.glob_pattern.clone())
            .collect::<Vec<String>>();

        all_sources.push(sources.main_sources.clone());

        execute_compiler(
            &all_sources,
            &config.output_dir(),
            vec![],
            &config.workspace.psa_options,
            false,
            verbose,
        )?;
    }

    let output_dir = config.output_dir();
    let test_file = output_dir.join(format!("{}/index.js", module.unwrap_or("Main".to_string())));

    // Create a JavaScript snippet that loads the module and calls main
    let js_code = format!(
        "import {{main}} from '{}';main();",
        test_file.to_string_lossy()
    );

    let mut run_process = Command::new("node");
    run_process.arg("-e").arg(js_code);

    // Add any additional node arguments passed from the CLI
    for arg in node_args {
        run_process.arg(arg);
    }

    let mut child = run_process
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    child.wait()?;
    Ok(())
}
