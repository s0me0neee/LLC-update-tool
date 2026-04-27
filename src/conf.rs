use serde::{Deserialize, Serialize};
use std::{fmt::Debug, fs, path::PathBuf};
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
    fn path() -> Result<PathBuf, ConfigError>;

    fn read() -> Result<Self, ConfigError> {
        let path = Self::path()?;

        if !path.exists() {
            return Err(ConfigError::NotFound(path));
        }
        if !path.is_file() {
            return Err(ConfigError::NotAFile(path));
        }

        let ctx = fs::read_to_string(&path)?;
        let config = serde_json::from_str(&ctx)?;
        Ok(config)
    }

    fn write(conf: &Self) -> Result<(), ConfigError> {
        let path = Self::path()?;
        if !path.exists() {
            return Err(ConfigError::NotFound(path));
        }
        if !path.is_file() {
            return Err(ConfigError::NotAFile(path));
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(conf)?;
        fs::write(&path, json)?;
        Ok(())
    }
}
