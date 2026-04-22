use log::{error, info};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Language {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
}

fn get_lang_path(lbc_data_dir: PathBuf) -> PathBuf {
    let lang_dir = lbc_data_dir.join("Lang");
    info!("Lang installed directory: {}", lang_dir.display());
    lang_dir
}

pub fn get_current_lang(lang_dir: PathBuf) -> Language {
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
    let lang_name = &lang_config.lang;

    info!("Current lang: {}", lang_name);
    Language {
        name: lang_name.clone(),
        path: lang_dir.join(lang_name),
    }
}

pub fn get_languages(lang_dir: &PathBuf) -> Vec<Language> {
    let mut langs = Vec::new();
    for entry in std::fs::read_dir(lang_dir)
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
        let path = entry.path();
        info!("Found language: {}", name);
        langs.push(Language { name, path });
    }
    langs
}
