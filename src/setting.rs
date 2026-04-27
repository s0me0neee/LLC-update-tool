use core::panic;
use std::{io, path::PathBuf};

use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use url::Url;

use crate::conf::{Config, ConfigError};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Lock {
    pub(crate) name: String,
    source: Url,
    path: PathBuf,
    pub(crate) checksum: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Default)]
pub struct Setting {
    pub(crate) date: chrono::NaiveDateTime,
    pub(crate) locks: Vec<Lock>,
}

impl Config for Setting {
    fn path() -> Result<PathBuf, ConfigError> {
        Ok(crate::path::get_appdata_path().join("lock.json"))
    }
}

impl Lock {
    pub fn new(name: String, source: &Url, path: &PathBuf) -> Self {
        Lock {
            name,
            source: source.clone(),
            path: path.clone(),
            checksum: String::new(),
        }
    }

    pub fn refresh_checksum(&mut self) -> Result<(), String> {
        match hash(&self.path) {
            Ok(sha) => {
                self.checksum = sha;
                Ok(())
            }
            Err(e) => Err(format!("Failed to hash file at {:?}: {}", self.path, e)),
        }
    }
}

fn hash(file: &PathBuf) -> io::Result<String> {
    let mut file = std::fs::File::open(file)?;
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
    Setting::read().unwrap_or_else(|e| match e {
        ConfigError::NotFound(path) => {
            let prompt_msg = format!(
                "Can't find lock.json at {}, create one?",
                std::path::absolute(&path)
                    .unwrap_or_else(|e| {
                        error!("Failed to get absolute path: {}", e);
                        panic!();
                    })
                    .display()
            );

            match inquire::Confirm::new(&prompt_msg)
                .with_default(true)
                .prompt()
            {
                Ok(true) => {
                    let default_setting = Setting::default();
                    warn!("Creating default config at {}", path.display());
                    std::fs::create_dir_all(path.parent().unwrap()).unwrap_or_else(|err| {
                        error!("Failed to create config directory: {}", err);
                        panic!();
                    });
                    if let Err(write_err) = Setting::write(&default_setting) {
                        match write_err {
                            ConfigError::NotFound(path_buf) => {
                                std::fs::File::create_new(path_buf).unwrap_or_else(|err| {
                                    error!("Failed to create config file: {}", err);
                                    panic!();
                                });
                            }
                            _ => {
                                error!("Failed to create default config: {}", write_err);
                                panic!();
                            }
                        }
                    }
                    default_setting
                }
                Ok(false) => {
                    error!("Config file is required to run. Exiting.");
                    std::process::exit(1);
                }
                Err(_) => {
                    error!("Selection canceled.");
                    std::process::exit(1);
                }
            }
        }

        ConfigError::NotAFile(path) => {
            error!("Path is a directory, not a file: {}", path.display());
            panic!("Filesystem conflict");
        }

        ConfigError::JsonError(error) => {
            error!("Config file is corrupted: {}", error);
            panic!("Invalid JSON");
        }

        ConfigError::IoError(error) => {
            error!("IO error: {}", error);
            panic!("File system error");
        }
    })
}

#[test]
fn lock_test() {
    let locks = vec![Lock::new(
        "test".to_string(),
        &Url::parse("https://example.com").unwrap(),
        &PathBuf::from("./justfile"),
    )];

    let setting = Setting {
        date: chrono::Local::now().naive_local(),
        locks,
    };

    let _ = Setting::write(&setting).expect("Failed to write lock");
    let ctx = Setting::read().expect("Failed to read lock");

    assert_eq!(setting, ctx);
}
