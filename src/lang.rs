use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
    #[serde(rename = "path")]
    pub(crate) language_dir_path: PathBuf,
}

fn get_lang_path(lbc_data_dir_path: &Path) -> PathBuf {
    let language_dir = lbc_data_dir_path.join("Lang");
    info!("Language install directory: {}", language_dir.display());
    language_dir
}

pub fn get_current_lang(language_dir_path: &Path) -> Language {
    let config_file_path = language_dir_path.join("config.json");
    info!(
        "Reading current language configuration from {}",
        config_file_path.display()
    );

    let config_content = std::fs::read_to_string(&config_file_path).unwrap_or_else(|err| {
        error!(
            "Failed to read language config {}: {}",
            config_file_path.display(),
            err
        );
        panic!();
    });

    let lang_config: LangConfig = match serde_json::from_str(&config_content) {
        Ok(config) => config,
        Err(err) => {
            error!(
                "Failed to parse language config {}: {}",
                config_file_path.display(),
                err
            );
            panic!();
        }
    };
    let lang_name = &lang_config.lang;

    info!("Current language from config: {}", lang_name);
    Language {
        name: lang_name.clone(),
        language_dir_path: language_dir_path.join(lang_name),
    }
}

pub fn get_languages(language_dir_path: &Path) -> Vec<Language> {
    info!(
        "Scanning language directory: {}",
        language_dir_path.display()
    );
    let entries = std::fs::read_dir(language_dir_path).unwrap_or_else(|err| {
        error!(
            "Failed to read language directory {}: {}",
            language_dir_path.display(),
            err
        );
        panic!();
    });

    let mut languages = Vec::new();
    for entry_result in entries {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(err) => {
                warn!("Skipping unreadable language entry: {}", err);
                continue;
            }
        };

        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }

        let language_name = entry.file_name().to_string_lossy().to_string();
        info!("Discovered installed language: {}", language_name);
        languages.push(Language {
            name: language_name,
            language_dir_path: entry_path,
        });
    }

    info!("Found {} installed language(s)", languages.len());
    languages
}
