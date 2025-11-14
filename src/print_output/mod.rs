use anyhow::Result;

use crate::config::load_config_cwd;

pub fn execute() -> Result<()> {
    let config = load_config_cwd()?;
    println!("{}", config.output_dir().display());
    Ok(())
}
