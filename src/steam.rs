use std::path::PathBuf;

use super::*;
use keyvalues_parser::{Value, parse};
use lang::LangConfig;
use path::get_steam_path;

#[derive(Debug, Default)]
pub struct App {
    id: String,
    pub(crate) name: String,
    acf_path: PathBuf,
    pub(crate) install_dir: String,
}

pub fn get_info() -> Vec<App> {
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

fn get_app_info(acf_ctx: String, app: &mut App) {
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
