use log::debug;
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
        return PathBuf::from("./test/Steam");
    }

    let Some(mut local_data_dir) = dirs::data_local_dir() else {
        eprintln!("Error: Could not resolve local data directory for Steam path");
        std::process::exit(1);
    };

    local_data_dir.push("Steam");
    debug!("Resolved Steam path: {}", local_data_dir.display());
    local_data_dir
}

pub fn get_appdata_path() -> PathBuf {
    if is_test_mode() {
        return PathBuf::from("./test/llc");
    }

    let Some(mut user_data_dir) = dirs::data_dir() else {
        eprintln!("Error: Could not resolve user data directory for LLC config path");
        std::process::exit(1);
    };

    user_data_dir.push("llc/");
    user_data_dir
}
