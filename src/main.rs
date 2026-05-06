mod cli;
mod conf;
mod fs;
mod lang;
mod llc;
mod path;
mod setting;
mod steam;
use clap::Parser;
use conf::Config;
use inquire::InquireError;
use llc::AssetWrapper;
use log::{info, warn};
use std::{path::PathBuf, process::exit, str::FromStr};

use crate::setting::{Lock, Setting};

#[macro_export]
macro_rules! env_dbg_init {
    () => {
        env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .parse_default_env()
            .init();
        info!("logger initialized");
    };
}

#[derive(Debug)]
struct Paths {
    archive_file_path: PathBuf,
    app_cache_dir: PathBuf,
    lbc_data_dir: PathBuf,
    lbc_lang_dir: PathBuf,
}

impl Paths {
    fn new(archive_file_path: PathBuf, lbc_data_dir: PathBuf) -> Self {
        let app_cache_dir = path::get_appdata_path().join("cache");
        let lbc_lang_dir = lbc_data_dir.join("Lang");
        Self {
            archive_file_path,
            app_cache_dir,
            lbc_data_dir,
            lbc_lang_dir,
        }
    }
}

fn init() {
    if cfg!(target_os = "macos") {
        println!("Mac(wine) support is currently in development.");
    }
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .parse_default_env()
        .init();
    info!("logger initialized");

    if rustls::crypto::ring::default_provider()
        .install_default()
        .is_err()
    {
        eprintln!("Error: Failed to install rustls crypto provider");
        exit(1);
    }
}

fn handle_prompt_error(context: &str, err: InquireError) -> ! {
    match err {
        InquireError::OperationCanceled => {
            println!("\n{} canceled by user.", context);
            exit(0);
        }
        InquireError::OperationInterrupted => {
            println!("\n{} interrupted (Ctrl+C).", context);
            exit(0);
        }
        _ => {
            eprintln!("\n{} failed: {}", context, err);
            exit(1);
        }
    }
}

fn prompt_confirm(message: &str, default: bool, context: &str) -> bool {
    match inquire::Confirm::new(message)
        .with_default(default)
        .prompt()
    {
        Ok(ans) => ans,
        Err(err) => handle_prompt_error(context, err),
    }
}

fn select_asset(asset_options: Vec<AssetWrapper>) -> AssetWrapper {
    match inquire::Select::new("Select a release asset to download:", asset_options).prompt() {
        Ok(asset) => asset,
        Err(err) => handle_prompt_error("Asset selection", err),
    }
}

fn create_all_dirs(paths: &Paths) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(&paths.app_cache_dir)?;
    if paths.lbc_data_dir.exists() {
        info!(
            "Using existing game data directory: {}",
            paths.lbc_data_dir.display()
        );
        return Ok(());
    }
    match inquire::Confirm::new("Limbus Company data directory was not found. Create it now?")
        .with_default(false)
        .prompt()
    {
        Ok(true) => {
            std::fs::create_dir_all(&paths.lbc_lang_dir)?;
            info!(
                "Created game language directory: {}",
                paths.lbc_lang_dir.display()
            );
            Ok(())
        }
        Ok(false) => Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "Required game data directory was not found: {}",
                paths.lbc_data_dir.display()
            ),
        )),
        Err(e) => Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            format!("Data directory creation prompt interrupted: {}", e),
        )),
    }
}

fn prompt_url(message: &str, default: &str, help: &str, context: &str) -> String {
    match inquire::Text::new(message)
        .with_default(default)
        .with_help_message(help)
        .prompt()
    {
        Ok(val) => val,
        Err(err) => handle_prompt_error(context, err),
    }
}

fn get_repository_url() -> String {
    prompt_url(
        "GitHub repository URL to fetch releases from:",
        "https://github.com/LocalizeLimbusCompany/LocalizeLimbusCompany",
        "Press Enter to use the default LocalizeLimbusCompany repository",
        "Repository URL input",
    )
}

fn get_font_url() -> String {
    prompt_url(
        "Font asset URL:",
        "https://raw.githubusercontent.com/LocalizeLimbusCompany/LocalizeLimbusCompany/refs/heads/main/Fonts/LLCCN-Font.7z",
        "Press Enter to use the source",
        "Download URL input",
    )
}

fn get_font_archive_file_path(paths: &Paths, font_url: &url::Url) -> PathBuf {
    let file_name = font_url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|s| !s.is_empty())
        .unwrap_or("LLCCN-Font.7z");
    paths.app_cache_dir.join(file_name)
}

fn find_extracted_lang_root_dir_path(font_extract_dir_path: &std::path::Path) -> Option<PathBuf> {
    let direct = font_extract_dir_path
        .join("LimbusCompany_Data")
        .join("Lang");
    if direct.is_dir() {
        return Some(direct);
    }

    let direct_lang = font_extract_dir_path.join("Lang");
    if direct_lang.is_dir() {
        return Some(direct_lang);
    }

    let entries = std::fs::read_dir(font_extract_dir_path).ok()?;
    for entry in entries {
        let entry_path = entry.ok()?.path();
        if !entry_path.is_dir() {
            continue;
        }
        let candidate = entry_path.join("LimbusCompany_Data").join("Lang");
        if candidate.is_dir() {
            return Some(candidate);
        }
        let candidate_lang = entry_path.join("Lang");
        if candidate_lang.is_dir() {
            return Some(candidate_lang);
        }
    }

    None
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

async fn maybe_install_font(
    setting: &mut Setting,
    paths: &Paths,
    font_url: &url::Url,
) -> Result<(), Box<dyn std::error::Error>> {
    let should_install_font = prompt_confirm(
        "Install font package? (Required for some languages)",
        true,
        "Font install confirmation",
    );
    if !should_install_font {
        return Ok(());
    }

    let font_archive_file_path = get_font_archive_file_path(paths, font_url);
    let should_download_font = if font_archive_file_path.exists() {
        if let Some(font_lock) = setting.font.as_mut()
            && let Err(e) = font_lock.refresh_checksum()
        {
            warn!("Failed to hash cached font: {}", e);
        }
        !prompt_confirm(
            "Font archive is cached. Skip download?",
            true,
            "Skip font download confirmation",
        )
    } else {
        true
    };

    if should_download_font {
        println!("Downloading font from: {}", font_url);
        info!(
            "Downloading font archive to {}",
            font_archive_file_path.display()
        );
        llc::download_asset(font_url.clone(), &font_archive_file_path).await?;
    } else {
        info!(
            "Skipping font download; using cached file at {}",
            font_archive_file_path.display()
        );
    }

    let mut font_lock = Lock::new(
        font_archive_file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("LLCCN-Font.7z")
            .to_string(),
        font_url,
        &font_archive_file_path,
    );
    if let Err(e) = font_lock.refresh_checksum() {
        warn!("Failed to compute font checksum: {}", e);
    }
    setting.font = Some(font_lock);
    Setting::write(setting)?;

    let font_extract_dir_path = paths.app_cache_dir.join("font_extract");
    std::fs::remove_dir_all(&font_extract_dir_path).ok();
    std::fs::create_dir_all(&font_extract_dir_path)?;

    info!(
        "Extracting font archive {} into {}",
        font_archive_file_path.display(),
        font_extract_dir_path.display()
    );
    llc::extract_asset(&font_archive_file_path, &font_extract_dir_path).await?;

    let extracted_lang_root_dir_path = find_extracted_lang_root_dir_path(&font_extract_dir_path)
        .ok_or_else(|| {
            format!(
                "Font archive extraction did not contain a Lang directory under {}",
                font_extract_dir_path.display()
            )
        })?;
    info!(
        "Font archive language root directory: {}",
        extracted_lang_root_dir_path.display()
    );
    info!(
        "Installing fonts into game language directory: {}",
        paths.lbc_lang_dir.display()
    );

    let mut installed_font_count = 0usize;
    for entry in std::fs::read_dir(&extracted_lang_root_dir_path)? {
        let entry = entry?;
        let extracted_language_dir_path = entry.path();
        if !extracted_language_dir_path.is_dir() {
            continue;
        }
        let language_name = extracted_language_dir_path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "<unknown>".to_string());

        let extracted_font_dir_path = extracted_language_dir_path.join("Font");
        if !extracted_font_dir_path.is_dir() {
            info!(
                "Skipping {}: no Font directory in extracted archive",
                language_name
            );
            continue;
        }

        let target_language_dir_path = paths.lbc_lang_dir.join(&language_name);
        let target_font_dir_path = target_language_dir_path.join("Font");
        std::fs::create_dir_all(&target_language_dir_path)?;

        if target_font_dir_path.exists() {
            info!(
                "Removing existing font directory before install: {}",
                target_font_dir_path.display()
            );
            std::fs::remove_dir_all(&target_font_dir_path)?;
        }

        info!(
            "Installing font for {} into {}",
            language_name,
            target_font_dir_path.display()
        );
        copy_dir_recursive(&extracted_font_dir_path, &target_font_dir_path)?;
        installed_font_count += 1;
    }

    if installed_font_count == 0 {
        return Err("No font directory was found inside the extracted font archive".into());
    }

    if let Err(e) = std::fs::remove_dir_all(&font_extract_dir_path) {
        warn!("Failed to clean up font extract directory: {}", e);
    }
    info!(
        "Font installation completed for {} language folder(s)",
        installed_font_count
    );
    Ok(())
}

fn should_skip_download(
    setting: &mut Setting,
    asset_name: &str,
    asset_digest: Option<&str>,
    download_file_path: &std::path::Path,
) -> bool {
    let Some(lock) = setting.locks.iter_mut().find(|l| l.name == asset_name) else {
        return false;
    };
    if !download_file_path.exists() {
        return false;
    }
    if let Err(e) = lock.refresh_checksum() {
        warn!("Failed to hash cached asset: {}", e);
        return false;
    }
    let Some(expected_digest) = asset_digest else {
        warn!("Selected release asset has no digest; checksum verification skipped");
        return false;
    };
    let expected_checksum = expected_digest
        .strip_prefix("sha256:")
        .unwrap_or(expected_digest);
    if lock.checksum == expected_checksum {
        info!("Local checksum matches release digest for {}", lock.name);
        prompt_confirm(
            "Asset checksum verified. Skip download?",
            true,
            "Skip download confirmation",
        )
    } else {
        warn!(
            "Checksum mismatch! Local: {}, Expected: {}",
            lock.checksum, expected_checksum
        );
        !prompt_confirm(
            "Checksum differs from release. Redownload asset?",
            true,
            "Redownload confirmation",
        )
    }
}

fn get_lbc_data_dir() -> PathBuf {
    if crate::path::is_test_mode() {
        return PathBuf::from("./test/LimbusCompany_Data");
    }
    if cfg!(not(windows)) {
        return crate::steam::get_lbc_data_dir_vdf();
    }
    #[cfg(windows)]
    {
        info!("Using Windows directory");
        crate::steam::windows::get_lbc_data_dir_reg().unwrap_or_else(|| {
            eprintln!("Error: Could not find Limbus Company data in the Windows Registry.");
            exit(1);
        })
    }
    #[cfg(not(windows))]
    unreachable!()
}

#[tokio::main]
async fn main() {
    init();
    let args = cli::Args::parse();

    if args.list {
        let lang_dir = get_lbc_data_dir().join("Lang");
        let langs = lang::get_languages(&lang_dir);
        if let Some(l) = lang::get_current_lang(&lang_dir) {
            println!(
                "Current language: {}, at: {}",
                l.name,
                l.language_dir_path.display()
            );
        };
        println!("Available languages:");
        for l in langs {
            println!("- {} ({})", l.name, l.language_dir_path.display());
        }
        exit(0);
    }

    let mut setting = setting::setting_init();
    info!(
        "Loaded settings with {} cached lock record(s)",
        setting.locks.len()
    );

    let repository_url = get_repository_url();
    let font_url = url::Url::from_str(&get_font_url()).unwrap_or_else(|e| {
        eprintln!("Error: Invalid font URL: {}", e);
        exit(1);
    });
    info!("Fetching releases from {}", repository_url);
    println!("Fetching releases...");
    let selected_release = llc::select_release(&repository_url)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Error: Failed to fetch releases: {}", e);
            exit(1);
        });
    let asset_list = llc::get_assets(selected_release);
    let asset = select_asset(asset_list).0;
    let asset_name = asset.name;
    let asset_digest = asset.digest;
    let download_url = asset.browser_download_url;
    info!("Selected asset: {}", asset_name);
    println!("Selected asset: {}", asset_name);

    let lbc_data_dir = get_lbc_data_dir();
    info!("Found data directory: {}", lbc_data_dir.display());
    let paths = Paths::new(PathBuf::from(&asset_name), lbc_data_dir);

    create_all_dirs(&paths).unwrap_or_else(|e| {
        eprintln!("Error: Failed to create necessary directories: {}", e);
        exit(1);
    });

    let download_file_path = paths.app_cache_dir.join(&paths.archive_file_path);
    let mut asset_lock = Lock::new(asset_name.clone(), &download_url, &download_file_path);
    let skip_download = should_skip_download(
        &mut setting,
        &asset_name,
        asset_digest.as_deref(),
        &download_file_path,
    );

    if !skip_download {
        println!("Downloading from: {}", download_url);
        info!("Downloading asset to {}", download_file_path.display());
        llc::download_asset(download_url, &download_file_path)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Error: Download failed: {}", e);
                exit(1);
            });

        asset_lock.refresh_checksum().ok();

        if let Some(lock) = setting.locks.iter_mut().find(|l| l.name == asset_name) {
            *lock = asset_lock;
        } else {
            setting.locks.push(asset_lock);
        }

        Setting::write(&setting).unwrap_or_else(|e| {
            eprintln!("Error: Failed to save settings: {}", e);
            exit(1);
        });
        info!("Settings updated with latest lock info");
    } else {
        info!("Skipping download for {}", asset_name);
        println!("Skipping download for {}", asset_name);
    }

    println!("Extracting asset...");
    llc::extract_asset(&download_file_path, &paths.lbc_lang_dir)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Error: Failed to extract asset: {}", e);
            exit(1);
        });

    println!("Installing files...");
    fs::install_and_clean(&paths).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        exit(1);
    });

    maybe_install_font(&mut setting, &paths, &font_url)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Error: Font installation failed: {}", e);
            exit(1);
        });
}

#[cfg(test)]
mod find_lang_root_tests {
    use super::*;

    fn temp_dir(suffix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "llc-main-test-{}-{}",
            suffix,
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finds_direct_nested_path() {
        let root = temp_dir("direct-nested");
        std::fs::create_dir_all(root.join("LimbusCompany_Data").join("Lang")).unwrap();

        let result = find_extracted_lang_root_dir_path(&root).unwrap();
        assert_eq!(result, root.join("LimbusCompany_Data").join("Lang"));

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn finds_direct_lang_path() {
        let root = temp_dir("direct-lang");
        std::fs::create_dir_all(root.join("Lang")).unwrap();

        let result = find_extracted_lang_root_dir_path(&root).unwrap();
        assert_eq!(result, root.join("Lang"));

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn finds_nested_path_under_subdirectory() {
        let root = temp_dir("subdir-nested");
        std::fs::create_dir_all(
            root.join("LimbusLocalize_2026041901")
                .join("LimbusCompany_Data")
                .join("Lang"),
        )
        .unwrap();

        let result = find_extracted_lang_root_dir_path(&root).unwrap();
        assert_eq!(
            result,
            root.join("LimbusLocalize_2026041901")
                .join("LimbusCompany_Data")
                .join("Lang")
        );

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn finds_lang_under_subdirectory() {
        let root = temp_dir("subdir-lang");
        std::fs::create_dir_all(root.join("some_release").join("Lang")).unwrap();

        let result = find_extracted_lang_root_dir_path(&root).unwrap();
        assert_eq!(result, root.join("some_release").join("Lang"));

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn returns_none_when_no_lang_dir() {
        let root = temp_dir("none");
        std::fs::create_dir_all(root.join("random_dir")).unwrap();

        let result = find_extracted_lang_root_dir_path(&root);
        assert!(result.is_none());

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn prefers_direct_nested_over_lang_shortcut() {
        // When both LimbusCompany_Data/Lang and Lang exist at top level,
        // the direct nested path is checked first and wins.
        let root = temp_dir("prefer-nested");
        std::fs::create_dir_all(root.join("LimbusCompany_Data").join("Lang")).unwrap();
        std::fs::create_dir_all(root.join("Lang")).unwrap();

        let result = find_extracted_lang_root_dir_path(&root).unwrap();
        assert_eq!(result, root.join("LimbusCompany_Data").join("Lang"));

        std::fs::remove_dir_all(root).ok();
    }
}

#[test]
fn language_test() {
    if crate::path::is_test_mode() {
        return;
    }
    env_dbg_init!();
    let paths = {
        let archive_file_path = PathBuf::from("test_none");

        #[cfg(windows)]
        let lbc_data_dir = crate::steam::windows::get_lbc_data_dir_reg().unwrap();

        #[cfg(not(windows))]
        let lbc_data_dir = crate::steam::get_lbc_data_dir_vdf();

        dbg!(&lbc_data_dir);
        Paths::new(archive_file_path, lbc_data_dir)
    };
    let languages = lang::get_languages(&paths.lbc_lang_dir);
    dbg!(&languages);
}
