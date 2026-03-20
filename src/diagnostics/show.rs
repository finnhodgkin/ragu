use anyhow::Result;

use crate::{config::load_config_cwd, diagnostics::store};

pub async fn execute(id: &str) -> Result<()> {
    let config = load_config_cwd()?;
    let found = store::load_diagnostic_by_id(&config.spago_dir(), id)?;

    println!(
        "{}",
        serde_json::to_string_pretty(&found.full)
            .unwrap_or_else(|_| found.full.to_string())
    );
    Ok(())
}
