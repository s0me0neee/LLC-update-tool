use super::*;
use std::path::PathBuf;

fn env_flag_is_true(name: &str) -> bool {
    let Some(value) = std::env::var_os(name) else {
        return false;
    };
    let value = value.to_string_lossy();
    let value = value.trim().to_ascii_lowercase();
    !(value.is_empty() || value == "0" || value == "false" || value == "no")
}

pub fn is_test_mode() -> bool {
    env_flag_is_true("TEST") || env_flag_is_true("GITHUB_ACTIONS")
}

pub fn get_steam_path() -> PathBuf {
    if is_test_mode() {
        let test_override_path = PathBuf::from("./test/Steam");
        warn!(
            "Using test override Steam path instead of resolved path: {}",
            test_override_path.display()
        );
        return test_override_path;
    }

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

    if is_test_mode() {
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
