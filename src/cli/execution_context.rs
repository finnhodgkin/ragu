use crate::config;
use anyhow::{Context, Result};

pub struct ExecutionContext {
    pub config: crate::config::SpagoConfig,
    pub package_set: crate::registry::PackageSet,
}

impl ExecutionContext {
    pub async fn load() -> Result<Self> {
        let config =
            config::load_config_cwd().context("Failed to load spago.yaml configuration")?;
        let package_set = config.package_set().await?;
        Ok(Self {
            config,
            package_set,
        })
    }
}
