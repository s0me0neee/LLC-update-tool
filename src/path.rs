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

// NOTE: Use reg to get game path
#[cfg(windows)]
fn get_steam_path_reg() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let steam_key = hkcu.open_subkey("Software\\Valve\\Steam")?;

    let steam_path: String = steam_key.get_value("SteamPath")?;
    Ok(PathBuf::from(steam_path))
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
