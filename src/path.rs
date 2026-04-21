use super::*;
use std::{
    ops::Deref,
    path::{Path, PathBuf},
};

pub fn get_steam_path() -> PathBuf {
    let base = dirs::data_local_dir();

    match base {
        Some(mut path) => {
            path.push("Steam");
            info!("Steam path: {}", path.display());
            path
        }
        None => {
            error!("Could not find local data directory");
            panic!();
        }
    }
}

pub fn get_cache_path() -> PathBuf {
    let base = dirs::data_dir();

    match base {
        Some(mut path) => {
            path.push("llc-updater/cache");
            info!("Cache path: {}", path.display());
            path
        }
        None => {
            error!("Could not find cache directory");
            panic!();
        }
    }
}
