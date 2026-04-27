use super::*;
use std::path::PathBuf;

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

pub fn get_appdata_path() -> PathBuf {
    let base = dirs::data_dir();

    match base {
        Some(mut path) => {
            path.push("llc/");
            info!("App data path: {}", path.display());
            path
        }
        None => {
            error!("Could not find cache directory");
            panic!();
        }
    };
    // NOTE: for testing, we want to use a local directory instead of the actual appdata directory
    PathBuf::from("./test/llc")
}
