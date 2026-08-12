pub mod auth;

use crate::error::CliError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub api_key: Option<String>,
}

/// Overrides the config directory. Lets tests and sandboxes run against a known-empty
/// config instead of whatever the developer has stored in their home directory.
pub const CONFIG_DIR_ENV: &str = "NOTION_MG_CONFIG_DIR";

impl Config {
    pub fn config_dir() -> Result<PathBuf, CliError> {
        if let Ok(dir) = std::env::var(CONFIG_DIR_ENV)
            && !dir.is_empty()
        {
            return Ok(PathBuf::from(dir));
        }

        let dir = dirs::config_dir()
            .ok_or_else(|| CliError::General("Could not determine config directory".to_string()))?
            .join("notion-mg");
        Ok(dir)
    }

    pub fn config_path() -> Result<PathBuf, CliError> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self, CliError> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), CliError> {
        let dir = Self::config_dir()?;
        std::fs::create_dir_all(&dir)?;
        let path = Self::config_path()?;
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&path, contents)?;
        Ok(())
    }
}
