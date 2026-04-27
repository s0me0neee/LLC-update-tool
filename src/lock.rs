use crate::lang::Language;
use futures_util::io;
use log::{error, info, warn};
use octocrab::models::timelines::Source;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::{fmt::Debug, path::PathBuf};
use url::Url;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Lock {
    name: String,
    source: Url,
    path: PathBuf,
    checksum: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Setting {
    date: chrono::NaiveDateTime,
    locks: Vec<Lock>,
}

trait Config: Debug + Serialize + for<'de> Deserialize<'de> + Sized {
    fn path() -> Result<PathBuf, String>;

    fn read() -> Result<Self, String> {
        let path = Self::path()?;
        let ctx = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&ctx).map_err(|e| e.to_string())
    }

    fn write(conf: &Self) -> Result<(), String> {
        let json = serde_json::to_string(conf).map_err(|e| e.to_string())?;
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, json).map_err(|e| e.to_string())
    }
}

// NOTE: remove redundant code and use the defult Config trait impl
impl Config for Setting {
    fn path() -> Result<PathBuf, String> {
        Ok(crate::path::get_appdata_path().join("lock.json"))
    }

    fn read() -> Result<Self, String> {
        let path = Self::path()?;
        info!("Reading lock from: {}", path.display());
        let ctx = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let config = serde_json::from_str(&ctx).map_err(|e| e.to_string())?;
        Ok(config)
    }

    fn write(conf: &Self) -> Result<(), String> {
        let json = serde_json::to_string(conf).map_err(|e| e.to_string())?;
        if let Some(parent) = std::path::Path::new(&Self::path()?).parent() {
            warn!(
                "Lock file does not exist, creating parent directory: {}",
                parent.display()
            );
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(std::path::Path::new(&Self::path()?), json).map_err(|e| e.to_string())
    }
}

impl Lock {
    fn new(name: String, source: &Url, path: &PathBuf) -> Self {
        let sha = hash(path).unwrap_or_else(|e| {
            error!("Failed to hash file: {}", e);
            String::new()
        });
        Lock {
            name,
            source: source.clone(),
            path: path.clone(),
            checksum: sha,
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
