use serde::{Deserialize, Serialize};

use std::path::PathBuf;

use crate::steam;
use log::{error, info};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct LangConfig {
    pub(crate) lang: String,
    #[serde(rename = "titleFont")]
    pub(crate) title_font: String,
    #[serde(rename = "contextFont")]
    pub(crate) context_font: String,
    #[serde(rename = "samplingPointSize")]
    pub(crate) sampling_point_size: u32,
    padding: u32,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Language {
    pub(crate) name: String,
    pub(crate) url: Option<String>,
}

pub fn get_lang_info() -> Vec<Language> {
    const GAME_NAME: &str = "Limbus Company";
    info!("Looking for game: {}", GAME_NAME);
    let apps = steam::get_info();
    let game = apps
        .iter()
        .find(|e| e.name == GAME_NAME)
        .ok_or_else(|| {
            error!("{GAME_NAME} can't be found or not installed");
            panic!();
        })
        .unwrap();

    let game_data_dir = PathBuf::from(game.install_dir.clone()).join("LimbusCompany_Data");
    if !game_data_dir.exists() {
        error!(
            "Game data directory does not exist at {}",
            game_data_dir.display()
        );
        panic!();
    }
    info!("Using game data directory: {}", game_data_dir.display());

    let lang_dir = game_data_dir.join("Lang");
    info!("Lang installed directory: {}", lang_dir.display());

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
    get_languages(lang_dir)
}

fn get_languages(dir: PathBuf) -> Vec<Language> {
    let mut langs = Vec::new();
    for entry in std::fs::read_dir(dir)
        .unwrap_or_else(|e| {
            error!("Failed to read lang directory: {}", e);
            panic!();
        })
        .map(|e| {
            e.unwrap_or_else(|e| {
                error!("Failed to read language entry: {}", e);
                panic!();
            })
        })
        .filter(|entry| entry.path().is_dir())
    {
        let name = entry.file_name().to_string_lossy().to_string();
        info!("Found language: {}", name);
        langs.push(Language { name, url: None });
    }
    langs
}

#[test]
fn vdf() {
    env_logger::builder()
        .filter_level(log::LevelFilter::max())
        .parse_default_env()
        .init();

    dbg!(get_lang_info());
}
