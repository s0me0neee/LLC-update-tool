use std::{default, path::PathBuf};

use super::*;
use keyvalues_parser::{Value, parse};
use path::get_steam_path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
struct App {
    id: String,
    name: String,
    acf_path: PathBuf,
    install_dir: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct LangConfig {
    lang: String,
    #[serde(rename = "titleFont")]
    title_font: String,
    #[serde(rename = "contextFont")]
    context_font: String,
    #[serde(rename = "samplingPointSize")]
    sampling_point_size: u32,
    padding: u32,
}

fn get_game_info() {
    const GAME_NAME: &str = "Limbus Company";
    let apps = get_info();
    let game = apps
        .iter()
        .find(|e| e.name == GAME_NAME)
        .ok_or_else(|| {
            error!("{GAME_NAME} can't be found or not installed");
            panic!();
        })
        .unwrap();

    let game_data_dir = PathBuf::from(game.install_dir.clone()).join("LimbusCompany_Data");
    let lang_dir = game_data_dir.join("Lang");
    let lang_config: LangConfig = match serde_json::from_str(
        &std::fs::read_to_string(lang_dir.join("config.json")).unwrap_or_else(|e| {
            error!("Failed to read json: {}", e);
            panic!();
        }),
    ) {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to parse json: {}", e);
            panic!();
        }
    };

    info!("Current lang: {}", &lang_config.lang);

    for entry in std::fs::read_dir(lang_dir).unwrap() {
        let entry = entry.unwrap(); // Error handling for individual entries
        let path = entry.path();
        println!("Name: {}", path.display());
    }
}

fn get_info() -> Vec<App> {
    let steam_path = get_steam_path();
    let library_vdf_path = steam_path.join("steamapps/libraryfolders.vdf");
    if !library_vdf_path.exists() {
        error!(
            "Could not find libraryfolders.vdf at {}",
            library_vdf_path.display()
        );
        panic!();
    }
    let ids = parse_library_vdf(library_vdf_path);
    let mut apps: Vec<App> = Vec::new();

    for file_id in ids {
        let mut app = App::default();
        let acf_path = steam_path
            .join("steamapps")
            .join(format!("appmanifest_{}.acf", file_id));
        app.acf_path = acf_path.clone();
        match std::fs::read_to_string(&acf_path) {
            Ok(acf_content) => {
                app.id = file_id;
                get_app_info(acf_content, &mut app);
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

fn get_app_info(ctx: String, app: &mut App) {
    let acf = parse(&ctx)
        .unwrap_or_else(|e| {
            error!("Error reading {}", e);
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
            "".to_string()
        });

    info!("Install directory: {}", install_dir);
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
            _ => (),
        }
    }
    apps
}

#[test]
fn vdf() {
    get_game_info();
}
