use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found at: {0}")]
    NotFound(PathBuf),

    #[error("Path is a directory, not a file: {0}")]
    NotAFile(PathBuf),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub trait Config: Debug + Serialize + for<'de> Deserialize<'de> + Sized {
    fn config_file_path() -> Result<PathBuf, ConfigError>;

    fn read() -> Result<Self, ConfigError> {
        let config_file_path = Self::config_file_path()?;
        ensure_config_file_exists(&config_file_path)?;
        info!("Reading config file {}", config_file_path.display());

        let file_content = fs::read_to_string(&config_file_path)?;
        let config = serde_json::from_str(&file_content)?;
        info!(
            "Config file loaded successfully from {}",
            config_file_path.display()
        );
        Ok(config)
    }

    fn write(config_value: &Self) -> Result<(), ConfigError> {
        let config_file_path = Self::config_file_path()?;
        if config_file_path.exists() && config_file_path.is_dir() {
            return Err(ConfigError::NotAFile(config_file_path));
        }

        if let Some(parent_dir_path) = config_file_path.parent() {
            if !parent_dir_path.exists() {
                warn!(
                    "Parent config directory missing, creating {}",
                    parent_dir_path.display()
                );
            }
            fs::create_dir_all(parent_dir_path)?;
        }

        info!("Writing config file to {}", config_file_path.display());
        let json_content = serde_json::to_string_pretty(config_value)?;
        fs::write(&config_file_path, json_content)?;
        info!(
            "Config file write completed: {}",
            config_file_path.display()
        );
        Ok(())
    }
}

fn ensure_config_file_exists(config_file_path: &Path) -> Result<(), ConfigError> {
    if !config_file_path.exists() {
        return Err(ConfigError::NotFound(config_file_path.to_path_buf()));
    }
    if !config_file_path.is_file() {
        return Err(ConfigError::NotAFile(config_file_path.to_path_buf()));
    }
    Ok(())
}
