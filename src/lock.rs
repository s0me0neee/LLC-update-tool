use crate::lang::Language;
use futures_util::io;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::{fmt::Debug, path::PathBuf};
use url::Url;

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

#[derive(Debug, Deserialize, Serialize)]
struct Lock {
    git_url: Url,
    path: PathBuf,
    sha: String,
}

impl Config for Lock {
    fn path(&self) -> Result<PathBuf, String> {
        Ok(crate::path::get_appdata_path().join("lock.json"))
    }

    fn read(&self) -> Result<Self, String> {
        let path = self.path()?;
        info!("Reading lock from: {}", path.display());
        let ctx = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let config = serde_json::from_str(&ctx).map_err(|e| e.to_string())?;
        Ok(config)
    }

    fn write(&self) -> Result<(), String> {
        let json = serde_json::to_string(self).map_err(|e| e.to_string())?;
        if let Some(parent) = std::path::Path::new(&self.path()?).parent() {
            warn!(
                "Lock file does not exist, creating parent directory: {}",
                parent.display()
            );
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(std::path::Path::new(&self.path()?), json).map_err(|e| e.to_string())
    }
}

impl Lock {
    fn new(git_url: &Url, file: &PathBuf) -> Self {
        let sha = hash(file).unwrap_or_else(|e| {
            error!("Failed to hash file: {}", e);
            String::new()
        });
        Lock {
            git_url: git_url.clone(),
            path: file.clone(),
            sha,
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
fn test_lock() {
    let file = std::path::PathBuf::from("./justfile");
    let url = Url::parse("https://google.com").unwrap();
    let lock = Lock::new(&url, &file);
}
