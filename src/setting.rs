use std::{
    io,
    path::{Path, PathBuf},
    process::exit,
};

use log::{error, warn};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use url::Url;

use crate::conf::{Config, ConfigError};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Lock {
    pub(crate) name: String,
    source: Url,
    #[serde(rename = "path")]
    asset_file_path: PathBuf,
    pub(crate) checksum: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Default)]
pub struct Setting {
    pub(crate) date: chrono::NaiveDateTime,
    pub(crate) font: Option<Lock>,
    pub(crate) locks: Vec<Lock>,
}

impl Config for Setting {
    fn config_file_path() -> Result<PathBuf, ConfigError> {
        Ok(crate::path::get_appdata_path().join("lock.json"))
    }
}

impl Lock {
    pub fn new(name: String, source_url: &Url, asset_file_path: &Path) -> Self {
        Lock {
            name,
            source: source_url.clone(),
            asset_file_path: asset_file_path.to_path_buf(),
            checksum: String::new(),
        }
    }

    pub fn refresh_checksum(&mut self) -> Result<(), String> {
        match hash(&self.asset_file_path) {
            Ok(sha) => {
                self.checksum = sha;
                Ok(())
            }
            Err(err) => Err(format!(
                "Failed to hash file at {}: {}",
                self.asset_file_path.display(),
                err
            )),
        }
    }
}

fn hash(file_path: &Path) -> io::Result<String> {
    let mut file = std::fs::File::open(file_path)?;
    let mut hasher = sha2::Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let count = std::io::Read::read(&mut file, &mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let result = hasher.finalize();
    Ok(hex::encode(result))
}

pub fn setting_init() -> Setting {
    match Setting::read() {
        Ok(setting) => {
            log::info!(
                "Loaded settings from {} with {} lock(s)",
                Setting::config_file_path()
                    .map(|config_file_path| config_file_path.display().to_string())
                    .unwrap_or_else(|_| "<unknown config file>".to_string()),
                setting.locks.len()
            );
            setting
        }
        Err(err) => handle_setting_init_error(err),
    }
}

fn handle_setting_init_error(err: ConfigError) -> Setting {
    match err {
        ConfigError::NotFound(config_file_path) => {
            create_default_setting_interactive(config_file_path)
        }
        ConfigError::NotAFile(config_file_path) => {
            error!(
                "Config file path points to a directory, not a file: {}",
                config_file_path.display()
            );
            panic!("filesystem conflict at config path");
        }
        ConfigError::JsonError(err) => {
            error!("Config file is not valid JSON: {}", err);
            panic!("invalid config json");
        }
        ConfigError::IoError(err) => {
            error!("I/O error while reading config: {}", err);
            panic!("config io error");
        }
    }
}

fn create_default_setting_interactive(config_file_path: PathBuf) -> Setting {
    let absolute_path_display = std::path::absolute(&config_file_path)
        .map(|absolute_path| absolute_path.display().to_string())
        .unwrap_or_else(|err| {
            error!("Failed to resolve absolute config path: {}", err);
            config_file_path.display().to_string()
        });

    let prompt_message = format!(
        "Can't find lock.json at {}. Create it now?",
        absolute_path_display
    );

    let should_create = match inquire::Confirm::new(&prompt_message)
        .with_default(true)
        .prompt()
    {
        Ok(answer) => answer,
        Err(err) => {
            error!("Config creation prompt failed: {}", err);
            exit(1);
        }
    };

    if !should_create {
        error!("Config file is required to run. Exiting.");
        exit(1);
    }

    create_default_setting(config_file_path)
}

fn create_default_setting(config_file_path: PathBuf) -> Setting {
    let default_setting = Setting::default();
    warn!(
        "Creating default config file at {}",
        config_file_path.display()
    );

    let config_dir_path = config_file_path.parent().unwrap_or_else(|| {
        error!(
            "Cannot determine parent directory for config file path: {}",
            config_file_path.display()
        );
        panic!("invalid config path");
    });

    std::fs::create_dir_all(config_dir_path).unwrap_or_else(|err| {
        error!(
            "Failed to create config directory {}: {}",
            config_dir_path.display(),
            err
        );
        panic!();
    });

    if let Err(write_err) = Setting::write(&default_setting) {
        warn!(
            "Initial write failed while creating default config: {}",
            write_err
        );

        match write_err {
            ConfigError::NotFound(missing_path) => {
                std::fs::File::create(&missing_path).unwrap_or_else(|err| {
                    error!(
                        "Failed to create config file {}: {}",
                        missing_path.display(),
                        err
                    );
                    panic!();
                });
                Setting::write(&default_setting).unwrap_or_else(|err| {
                    error!(
                        "Failed to write default config after file creation: {}",
                        err
                    );
                    panic!();
                });
            }
            other => {
                error!("Failed to create default config: {}", other);
                panic!();
            }
        }
    }

    log::info!("Default config initialized successfully");
    default_setting
}

#[test]
fn lock_test() {
    let locks = vec![Lock::new(
        "test".to_string(),
        &Url::parse("https://example.com").unwrap(),
        &PathBuf::from("./justfile"),
    )];

    let font = Lock::new(
        "test".to_string(),
        &Url::parse("https://example.com").unwrap(),
        &PathBuf::from("./justfile"),
    );

    let setting = Setting {
        date: chrono::Local::now().naive_local(),
        font: Some(font),
        locks,
    };

    let _ = Setting::write(&setting).expect("Failed to write lock");
    let ctx = Setting::read().expect("Failed to read lock");

    assert_eq!(setting, ctx);
}
