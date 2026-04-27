use std::path::{Path, PathBuf};

use super::*;
use keyvalues_parser::{Value, parse};
use path::get_steam_path;

#[derive(Debug, Default)]
struct Game {
    id: String,
    name: String,
    manifest_file_path: PathBuf,
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
            let appid_str = entry.ok()?;
            let appid: u32 = appid_str.parse().ok()?;

            if !is_game_installed(appid) {
                continue;
            }

            let display_name = match get_game_name(appid) {
                Some(name) => name,
                None => continue,
            };

            if !display_name.eq_ignore_ascii_case(GAME_NAME) {
                continue;
            }

            let install_dir_path = match get_game_path(appid) {
                Some(path) => path,
                None => continue,
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

    fn get_game_name(appid: u32) -> Option<String> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let wow = format!(
            "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Steam App {}",
            appid
        );

        if let Ok(key) = hklm.open_subkey(&wow)
            && let Ok(name) = key.get_value("DisplayName")
        {
            return Some(name);
        }
        let plain = format!(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Steam App {}",
            appid
        );

        if let Ok(key) = hklm.open_subkey(&plain)
            && let Ok(name) = key.get_value("DisplayName")
        {
            return Some(name);
        }

        None
    }

    fn get_game_path(appid: u32) -> Option<String> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        let wow = format!(
            "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Steam App {}",
            appid
        );

        if let Ok(key) = hklm.open_subkey(&wow)
            && let Ok(loc) = key.get_value("InstallLocation")
        {
            return Some(loc);
        }

        let plain = format!(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Steam App {}",
            appid
        );

        if let Ok(key) = hklm.open_subkey(&plain)
            && let Ok(loc) = key.get_value("InstallLocation")
        {
            return Some(loc);
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
    let installed_games = steam::get_games();
    let target_game = installed_games
        .iter()
        .find(|e| e.name == GAME_NAME)
        .ok_or_else(|| {
            error!("{GAME_NAME} can't be found or not installed");
            panic!();
        })
        .unwrap();

    let lbc_data_dir_path =
        PathBuf::from(target_game.install_dir_path.clone()).join("LimbusCompany_Data");
    if !lbc_data_dir_path.exists() {
        error!(
            "Game data directory does not exist at {}",
            lbc_data_dir_path.display()
        );
        panic!();
    }
    info!("Using game data directory: {}", lbc_data_dir_path.display());
    lbc_data_dir_path
}

fn get_games() -> Vec<Game> {
    let steam_path = get_steam_path();
    info!("Using steam path: {}", steam_path.display());
    let library_vdf_path = steam_path.join("steamapps/libraryfolders.vdf");
    info!(
        "Looking for libraryfolders.vdf at {}",
        library_vdf_path.display()
    );
    if !library_vdf_path.exists() {
        error!(
            "Could not find libraryfolders.vdf at {}",
            library_vdf_path.display()
        );
        panic!();
    }
    let app_ids = parse_library_vdf(&library_vdf_path);
    info!(
        "Found {} app manifest id(s) in libraryfolders.vdf",
        app_ids.len()
    );
    let mut games: Vec<Game> = Vec::new();

    for app_id in app_ids {
        let manifest_path = steam_path
            .join("steamapps")
            .join(format!("appmanifest_{}.acf", app_id));
        info!("Reading manifest: {}", manifest_path.display());

        let manifest_content = match std::fs::read_to_string(&manifest_path) {
            Ok(content) => content,
            Err(err) => {
                warn!(
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
                warn!("Skipping app {}: {}", app_id, err);
                continue;
            }
        };

        let mut game = Game::default();
        game.id = app_id;
        game.manifest_file_path = manifest_path;
        game.name = game_meta.name;
        game.install_dir_path = steam_path
            .join("steamapps/common")
            .join(game_meta.install_dir)
            .to_string_lossy()
            .to_string();

        games.push(game);
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
        error!("Error reading libraryfolders.vdf: {}", e);
        panic!();
    });
    let vdf = parse(&contents).unwrap_or_else(|e| {
        error!("Failed to parse libraryfolders.vdf: {}", e);
        panic!();
    });

    if vdf.key != "libraryfolders" {
        error!("Unexpected root key in libraryfolders.vdf: {}", vdf.key);
        panic!();
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

        info!(
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
