use std::path::{Path, PathBuf};

use log::{debug, info, warn};
use keyvalues_parser::{Value, parse};
use crate::path::get_steam_path;

#[derive(Debug, Default)]
struct Game {
    name: String,
    install_dir_path: String,
}

#[cfg(windows)]
pub mod windows {
    use std::path::PathBuf;

    use log::{error, info, warn};
    use winreg::{
        RegKey,
        enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
    };
    pub fn get_lbc_data_dir_reg() -> Option<PathBuf> {
        const GAME_NAME: &str = "Limbus Company";
        info!("Scanning Steam registry entries for {}", GAME_NAME);

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let apps_root = hkcu.open_subkey("Software\\Valve\\Steam\\Apps").ok()?;

        for entry in apps_root.enum_keys() {
            let Ok(appid_str) = entry else { continue };
            let Ok(appid) = appid_str.parse::<u32>() else { continue };

            if !is_game_installed(appid) {
                continue;
            }

            let Some(display_name) = get_registry_value(appid, "DisplayName") else {
                continue;
            };
            if !display_name.eq_ignore_ascii_case(GAME_NAME) {
                continue;
            }

            let Some(install_dir_path) = get_registry_value(appid, "InstallLocation") else {
                continue;
            };

            let game_install_dir = PathBuf::from(install_dir_path);

            if !game_install_dir.exists() {
                error!(
                    "Game data directory does not exist at {}",
                    game_install_dir.display()
                );
                continue;
            }

            let game_data_dir = game_install_dir.join("LimbusCompany_Data");
            info!("Using game data directory: {}", game_data_dir.display());
            return Some(game_data_dir);
        }

        warn!("Could not find {} in Steam registry entries", GAME_NAME);
        None
    }

    fn is_game_installed(appid: u32) -> bool {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let app_subkey_path = format!("Software\\Valve\\Steam\\Apps\\{}", appid);

        if let Ok(key) = hkcu.open_subkey(app_subkey_path)
            && let Ok(installed) = key.get_value::<u32, _>("Installed")
        {
            return installed == 1;
        }

        false
    }

    fn get_registry_value(appid: u32, value_name: &str) -> Option<String> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let wow = format!(
            "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Steam App {}",
            appid
        );
        if let Ok(key) = hklm.open_subkey(&wow) {
            if let Ok(val) = key.get_value(value_name) {
                return Some(val);
            }
        }
        let plain = format!(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Steam App {}",
            appid
        );
        if let Ok(key) = hklm.open_subkey(&plain) {
            if let Ok(val) = key.get_value(value_name) {
                return Some(val);
            }
        }
        None
    }

    #[test]
    fn get_game() {
        match get_lbc_data_dir_reg() {
            Some(game_data_dir_path) => {
                println!("Found:{}", game_data_dir_path.display());
            }
            None => println!("Game not found"),
        }
    }
}

pub fn get_lbc_data_dir_vdf() -> PathBuf {
    const GAME_NAME: &str = "Limbus Company";
    info!("Looking for game: {}", GAME_NAME);
    let installed_games = get_games();
    let Some(target_game) = installed_games.iter().find(|e| e.name == GAME_NAME) else {
        eprintln!("Error: {GAME_NAME} is not installed or could not be found.");
        std::process::exit(1);
    };

    let lbc_data_dir_path = PathBuf::from(&target_game.install_dir_path).join("LimbusCompany_Data");
    if !lbc_data_dir_path.exists() {
        eprintln!("Error: Game data directory not found at {}", lbc_data_dir_path.display());
        std::process::exit(1);
    }
    info!("Using game data directory: {}", lbc_data_dir_path.display());
    lbc_data_dir_path
}

fn get_games() -> Vec<Game> {
    let steam_path = get_steam_path();
    debug!("Using steam path: {}", steam_path.display());
    let library_vdf_path = steam_path.join("steamapps/libraryfolders.vdf");
    debug!(
        "Looking for libraryfolders.vdf at {}",
        library_vdf_path.display()
    );
    if !library_vdf_path.exists() {
        eprintln!("Error: Could not find libraryfolders.vdf at {}", library_vdf_path.display());
        std::process::exit(1);
    }
    let app_ids = parse_library_vdf(&library_vdf_path);
    debug!(
        "Found {} app manifest id(s) in libraryfolders.vdf",
        app_ids.len()
    );
    let mut games: Vec<Game> = Vec::new();

    for app_id in app_ids {
        let manifest_path = steam_path
            .join("steamapps")
            .join(format!("appmanifest_{}.acf", app_id));
        debug!("Reading manifest: {}", manifest_path.display());

        let manifest_content = match std::fs::read_to_string(&manifest_path) {
            Ok(content) => content,
            Err(err) => {
                debug!(
                    "Skipping app {}: could not read {}: {}",
                    app_id,
                    manifest_path.display(),
                    err
                );
                continue;
            }
        };

        let game_meta = match get_game_info(&manifest_content) {
            Ok(meta) => meta,
            Err(err) => {
                debug!("Skipping app {}: {}", app_id, err);
                continue;
            }
        };

        games.push(Game {
            name: game_meta.name,
            install_dir_path: steam_path
                .join("steamapps/common")
                .join(game_meta.install_dir)
                .to_string_lossy()
                .to_string(),
        });
    }
    info!(
        "Loaded {} install record(s) from app manifests",
        games.len()
    );
    games
}

#[derive(Debug)]
struct GameMeta {
    name: String,
    install_dir: String,
}

fn get_game_info(manifest_content: &str) -> Result<GameMeta, String> {
    let manifest_root = parse(manifest_content)
        .map_err(|e| format!("Failed to parse appmanifest: {}", e))?
        .value
        .unwrap_obj();

    let name = manifest_root
        .get("name")
        .and_then(|v| v.first())
        .and_then(|v| match v {
            Value::Str(s) => Some(s.to_string()),
            _ => None,
        })
        .ok_or_else(|| "Missing or invalid `name` field in appmanifest".to_string())?;

    let install_dir = manifest_root
        .get("installdir")
        .and_then(|v| v.first())
        .and_then(|v| match v {
            Value::Str(s) => Some(s.to_string()),
            _ => None,
        })
        .ok_or_else(|| "Missing or invalid `installdir` field in appmanifest".to_string())?;

    Ok(GameMeta { name, install_dir })
}

fn parse_library_vdf(vdf_file_path: &Path) -> Vec<String> {
    let contents = std::fs::read_to_string(vdf_file_path).unwrap_or_else(|e| {
        eprintln!("Error reading libraryfolders.vdf: {}", e);
        std::process::exit(1);
    });
    let vdf = parse(&contents).unwrap_or_else(|e| {
        eprintln!("Failed to parse libraryfolders.vdf: {}", e);
        std::process::exit(1);
    });

    if vdf.key != "libraryfolders" {
        eprintln!("Error: Unexpected root key in libraryfolders.vdf: {}", vdf.key);
        std::process::exit(1);
    }

    let library_folders = vdf.value.unwrap_obj();
    let mut app_ids: Vec<String> = Vec::new();

    for (folder_id, folder_values) in library_folders.iter() {
        let Some(Value::Obj(folder_map)) = folder_values.first() else {
            warn!(
                "Skipping library folder {}: unexpected folder object format",
                folder_id
            );
            continue;
        };

        let Some(apps_values) = folder_map.get("apps") else {
            warn!("Skipping library folder {}: missing `apps` map", folder_id);
            continue;
        };

        let Some(Value::Obj(apps_map)) = apps_values.first() else {
            warn!(
                "Skipping library folder {}: unexpected `apps` map format",
                folder_id
            );
            continue;
        };

        debug!(
            "Discovered {} app id(s) in Steam library folder {}",
            apps_map.len(),
            folder_id
        );
        for (app_id, _) in apps_map.iter() {
            app_ids.push(app_id.to_string());
        }
    }

    app_ids
}
