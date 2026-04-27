use super::*;
use std::path::PathBuf;

pub fn get_steam_path() -> PathBuf {
    let Some(mut local_data_dir) = dirs::data_local_dir() else {
        error!("Could not resolve local data directory for Steam path");
        panic!();
    };

    local_data_dir.push("Steam");
    info!("Resolved Steam path: {}", local_data_dir.display());
    local_data_dir
}

pub fn get_appdata_path() -> PathBuf {
    let Some(mut user_data_dir) = dirs::data_dir() else {
        error!("Could not resolve user data directory for LLC config path");
        panic!();
    };

    if std::env::var_os("TEST").is_some() {
        // NOTE: for testing, we want to use a local directory instead of the actual appdata directory
        let test_override_path = PathBuf::from("./test/llc");
        warn!(
            "Using test override app data path instead of resolved path: {}",
            test_override_path.display()
        );
        return test_override_path;
    }
    user_data_dir.push("llc/");
    user_data_dir
}
