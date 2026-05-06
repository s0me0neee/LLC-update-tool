use log::{debug, error, info, warn};
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

pub fn get_current_lang(language_dir_path: &Path) -> Option<Language> {
    let config_file_path = language_dir_path.join("config.json");
    debug!(
        "Reading current language configuration from {}",
        config_file_path.display()
    );

    let config_content = match std::fs::read_to_string(&config_file_path) {
        Ok(content) => content,
        Err(err) => {
            error!(
                "Failed to read language config {}: {}",
                config_file_path.display(),
                err
            );
            return None;
        }
    };

    let lang_config: LangConfig = match serde_json::from_str(&config_content) {
        Ok(config) => config,
        Err(err) => {
            error!("Failed to parse language config {}: {}", config_file_path.display(), err);
            return None;
        }
    };
    let lang_name = &lang_config.lang;

    info!("Current language from config: {}", lang_name);
    Some(Language {
        name: lang_name.clone(),
        language_dir_path: language_dir_path.join(lang_name),
    })
}

pub fn get_languages(language_dir_path: &Path) -> Vec<Language> {
    debug!(
        "Scanning language directory: {}",
        language_dir_path.display()
    );
    let entries = match std::fs::read_dir(language_dir_path) {
        Ok(e) => e,
        Err(err) => {
            error!("Failed to read language directory {}: {}", language_dir_path.display(), err);
            return Vec::new();
        }
    };

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
        debug!("Discovered installed language: {}", language_name);
        languages.push(Language {
            name: language_name,
            language_dir_path: entry_path,
        });
    }

    info!("Found {} installed language(s)", languages.len());
    languages
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(suffix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "llc-lang-test-{}-{}",
            suffix,
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn get_languages_returns_only_subdirectories() {
        let lang_dir = temp_dir("subdirs");
        fs::create_dir_all(lang_dir.join("LLC_zh-CN")).unwrap();
        fs::create_dir_all(lang_dir.join("LLC_zh-TW")).unwrap();
        fs::write(lang_dir.join("config.json"), b"{}").unwrap();

        let langs = get_languages(&lang_dir);
        let mut names: Vec<_> = langs.iter().map(|l| l.name.as_str()).collect();
        names.sort();

        assert_eq!(names, ["LLC_zh-CN", "LLC_zh-TW"]);

        fs::remove_dir_all(lang_dir).ok();
    }

    #[test]
    fn get_languages_empty_dir_returns_empty() {
        let lang_dir = temp_dir("empty");
        let langs = get_languages(&lang_dir);
        assert!(langs.is_empty());
        fs::remove_dir_all(lang_dir).ok();
    }

    #[test]
    fn get_languages_nonexistent_dir_returns_empty() {
        let lang_dir = PathBuf::from("/tmp/llc-lang-test-nonexistent-path-should-not-exist");
        let langs = get_languages(&lang_dir);
        assert!(langs.is_empty());
    }

    #[test]
    fn get_languages_path_reflects_real_location() {
        let lang_dir = temp_dir("paths");
        fs::create_dir_all(lang_dir.join("LLC_zh-CN")).unwrap();

        let langs = get_languages(&lang_dir);
        assert_eq!(langs.len(), 1);
        assert_eq!(langs[0].language_dir_path, lang_dir.join("LLC_zh-CN"));

        fs::remove_dir_all(lang_dir).ok();
    }

    #[test]
    fn get_current_lang_reads_config_json() {
        let lang_dir = temp_dir("config");
        let config = r#"{"lang":"LLC_zh-CN","titleFont":"f.ttf","contextFont":"f.ttf","samplingPointSize":40,"padding":0}"#;
        fs::write(lang_dir.join("config.json"), config).unwrap();

        let lang = get_current_lang(&lang_dir).unwrap();
        assert_eq!(lang.name, "LLC_zh-CN");
        assert_eq!(lang.language_dir_path, lang_dir.join("LLC_zh-CN"));

        fs::remove_dir_all(lang_dir).ok();
    }

    #[test]
    fn get_current_lang_returns_none_when_config_missing() {
        let lang_dir = temp_dir("no-config");
        let result = get_current_lang(&lang_dir);
        assert!(result.is_none());
        fs::remove_dir_all(lang_dir).ok();
    }

    #[test]
    fn get_current_lang_returns_none_on_invalid_json() {
        let lang_dir = temp_dir("bad-json");
        fs::write(lang_dir.join("config.json"), b"not valid json {{").unwrap();
        let result = get_current_lang(&lang_dir);
        assert!(result.is_none());
        fs::remove_dir_all(lang_dir).ok();
    }
}
