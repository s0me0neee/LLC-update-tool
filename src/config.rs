use crate::lang::Language;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, path::PathBuf};

trait Config: Debug + Serialize + for<'de> Deserialize<'de> {
    fn path(&self) -> Result<PathBuf, String>;
    fn read(&self) -> Result<Self, String> {
        let path = self.path()?;
        let ctx = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let config = serde_json::from_str(&ctx).map_err(|e| e.to_string())?;
        Ok(config)
    }
    fn write(&self) -> Result<(), String> {
        let json = serde_json::to_string(self).map_err(|e| e.to_string())?;
        if let Some(parent) = std::path::Path::new(&self.path()?).parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(std::path::Path::new(&self.path()?), json).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Setting {
    language: Vec<Language>,
}

impl Config for Setting {
    fn path(&self) -> Result<PathBuf, String> {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| {
                error!("Failed to get config directory");
                panic!();
            })
            .join("llc-update-tool")
            .join("config.json");
        info!("Config file: {}", config_path.display());
        if config_path.is_dir() {
            error!("Config path is a directory, expected a json file");
            panic!();
        }
        Ok(config_path)
    }

    fn read(&self) -> Result<Self, String> {
        let path = self.path()?;
        info!("Reading config from: {}", path.display());
        let ctx = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let config = serde_json::from_str(&ctx).map_err(|e| e.to_string())?;
        Ok(config)
    }

    fn write(&self) -> Result<(), String> {
        let json = serde_json::to_string(self).map_err(|e| e.to_string())?;
        if let Some(parent) = std::path::Path::new(&self.path()?).parent() {
            warn!(
                "Config file does not exist, creating parent directory: {}",
                parent.display()
            );
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(std::path::Path::new(&self.path()?), json).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod test {
    use crate::env_dbg_init;

    use super::*;

    #[test]
    fn test_config() {
        env_dbg_init!();
        let name = "zh-CN".to_string();
        let setting = Setting {
            language: vec![Language { name, url: None }],
        };
        let _ = setting.write();
        let read_setting = setting.read().unwrap();
        assert_eq!(setting.language[0].name, read_setting.language[0].name);
    }
}
