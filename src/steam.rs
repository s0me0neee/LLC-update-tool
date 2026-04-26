use std::path::PathBuf;

use super::*;
use keyvalues_parser::{Value, parse};
use path::get_steam_path;

#[derive(Debug, Default)]
struct Game {
    id: String,
    name: String,
    acf_path: PathBuf,
    install_dir: String,
}

#[cfg(windows)]
pub mod windows {
    use std::path::PathBuf;

    use log::{error, info};
    use winreg::{
        RegKey,
        enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE},
    };
    pub fn get_lbc_data_dir_reg() -> Option<PathBuf> {
        const GAME_NAME: &str = "Limbus Company";

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

            let path = match get_game_path(appid) {
                Some(p) => p,
                None => continue,
            };

            let lbc_data_dir = PathBuf::from(path);

            if !lbc_data_dir.exists() {
                error!(
                    "Game data directory does not exist at {}",
                    lbc_data_dir.display()
                );
                continue;
            }

            info!("Using game data directory: {}", lbc_data_dir.display());
            return Some(lbc_data_dir.join("LimbusCompany_Data"));
        }

        None
    }

    fn is_game_installed(appid: u32) -> bool {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let path = format!("Software\\Valve\\Steam\\Apps\\{}", appid);

        if let Ok(key) = hkcu.open_subkey(path)
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
            && let Ok(name) = key.get_value("DilayName")
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
            Some(path) => {
                println!("Found:{}", path.display());
            }
            None => println!("Game not found"),
        }
    }
}

pub fn get_lbc_data_dir_vdf() -> PathBuf {
    const GAME_NAME: &str = "Limbus Company";
    info!("Looking for game: {}", GAME_NAME);
    let apps = steam::get_games();
    let lbc = apps
        .iter()
        .find(|e| e.name == GAME_NAME)
        .ok_or_else(|| {
            error!("{GAME_NAME} can't be found or not installed");
            panic!();
        })
        .unwrap();

    let lbc_data_dir = PathBuf::from(lbc.install_dir.clone()).join("LimbusCompany_Data");
    if !lbc_data_dir.exists() {
        error!(
            "Game data directory does not exist at {}",
            lbc_data_dir.display()
        );
        panic!();
    }
    info!("Using game data directory: {}", lbc_data_dir.display());
    lbc_data_dir
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
    let ids = parse_library_vdf(library_vdf_path);
    let mut apps: Vec<Game> = Vec::new();

    for file_id in ids {
        let mut app = Game::default();
        let acf_path = steam_path
            .join("steamapps")
            .join(format!("appmanifest_{}.acf", file_id));
        app.acf_path = acf_path.clone();
        match std::fs::read_to_string(&acf_path) {
            Ok(acf_content) => {
                app.id = file_id;
                get_game_info(acf_content, &mut app);
                app.install_dir = steam_path
                    .join("steamapps/common")
                    .join(app.install_dir)
                    .to_string_lossy()
                    .to_string();
                apps.push(app);
            }
            Err(e) => {
                warn!(
                    "Skipping app {}: could not read {}: {}",
                    file_id,
                    acf_path.display(),
                    e
                );
            }
        }
    }
    apps
}

fn get_game_info(acf_ctx: String, app: &mut Game) {
    let acf = parse(&acf_ctx)
        .unwrap_or_else(|e| {
            error!("Error reading acf: {}", e);
            panic!();
        })
        .value
        .unwrap_obj();

    let install_dir = acf
        .get("installdir")
        .and_then(|v| v.first())
        .and_then(|v| {
            if let Value::Str(s) = v {
                Some(s.to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            error!("Could not find installdir in ACF");
            panic!();
        });

    app.name = install_dir.clone();
    app.install_dir = install_dir;
}

fn parse_library_vdf(vdf_path: PathBuf) -> Vec<String> {
    let contents = std::fs::read_to_string(vdf_path).unwrap_or_else(|e| {
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

    let library_map = vdf.value.unwrap_obj();
    let mut apps: Vec<String> = Vec::new();
    for (_folder_id, folder_value) in library_map.iter() {
        match folder_value.first() {
            Some(Value::Obj(folder_map))
                if let Some(apps_vec) = folder_map.get("apps")
                    && let Some(Value::Obj(apps_map)) = apps_vec.first() =>
            {
                for (app_id, _) in apps_map.iter() {
                    apps.push(app_id.to_string());
                }
            }
            _ => {
                error!(
                    "Unexpected format in libraryfolders.vdf for folder: {}",
                    _folder_id
                );
            }
        }
    }
    apps
}
