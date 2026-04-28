mod conf;
mod fs;
mod lang;
mod llc;
mod path;
mod setting;
mod steam;
use conf::Config;
use inquire::InquireError;
use llc::AssetWarper;
use log::{error, info, warn};
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
    app_data_dir: PathBuf,
    app_cache_dir: PathBuf,
    lbc_data_dir: PathBuf,
    lbc_lang_dir: PathBuf,
}

impl Paths {
    fn new(archive_file_path: PathBuf, lbc_data_dir: PathBuf) -> Self {
        let app_data_dir = path::get_appdata_path();
        let app_cache_dir = app_data_dir.join("cache");
        let lbc_lang_dir = lbc_data_dir.join("Lang");
        Self {
            archive_file_path,
            app_data_dir,
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

    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap_or_else(|_| {
            error!("Failed to install rustls crypto provider");
            panic!();
        });
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

fn select_asset(asset_options: Vec<AssetWarper>) -> AssetWarper {
    match inquire::Select::new("Select a release asset to download:", asset_options).prompt() {
        Ok(asset) => asset,
        Err(err) => handle_prompt_error("Asset selection", err),
    }
}

fn create_all_dirs(paths: &Paths) -> Result<(), std::io::Error> {
    info!(
        "Ensuring cache directory exists: {}",
        paths.app_cache_dir.display()
    );
    std::fs::create_dir_all(&paths.app_cache_dir)?;
    if !paths.lbc_data_dir.exists() {
        let create_data_dir_confirmation =
            inquire::Confirm::new("Limbus Company data directory was not found. Create it now?")
                .with_default(false)
                .prompt();
        return match create_data_dir_confirmation {
            Ok(true) => {
                std::fs::create_dir_all(&paths.lbc_lang_dir)?;
                info!(
                    "Created game language directory: {}",
                    &paths.lbc_lang_dir.display()
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
        };
    }
    info!(
        "Using existing game data directory: {}",
        paths.lbc_data_dir.display()
    );
    Ok(())
}

fn get_repository_url() -> String {
    let default_url = "https://github.com/LocalizeLimbusCompany/LocalizeLimbusCompany";

    let prompt_message = "GitHub repository URL to fetch releases from:";

    let input_result = inquire::Text::new(prompt_message)
        .with_default(default_url)
        .with_help_message("Press Enter to use the default LocalizeLimbusCompany repository")
        .prompt();

    match input_result {
        Ok(val) => val,
        Err(err) => handle_prompt_error("Repository URL input", err),
    }
}

fn get_font_url() -> String {
    let default_url = "https://raw.githubusercontent.com/LocalizeLimbusCompany/LocalizeLimbusCompany/refs/heads/main/Fonts/LLCCN-Font.7z";

    let prompt_message = "Font asset URL:";

    let input_result = inquire::Text::new(prompt_message)
        .with_default(default_url)
        .with_help_message("Press Enter to use the source")
        .prompt();

    match input_result {
        Ok(val) => val,
        Err(err) => handle_prompt_error("Download URL input", err),
    }
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

fn copy_dir_recursive(
    source_dir_path: &std::path::Path,
    target_dir_path: &std::path::Path,
) -> std::io::Result<()> {
    std::fs::create_dir_all(target_dir_path)?;
    for entry in std::fs::read_dir(source_dir_path)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target_dir_path.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&source_path, &target_path)?;
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
    let mut should_download_font = !font_archive_file_path.exists();

    if let Some(font_lock) = setting.font.as_mut()
        && font_archive_file_path.exists()
    {
        let _ = font_lock.refresh_checksum();
        should_download_font = !prompt_confirm(
            "Font archive is cached. Skip download?",
            true,
            "Skip font download confirmation",
        );
    }

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
    let _ = font_lock.refresh_checksum();
    setting.font = Some(font_lock);
    Setting::write(setting)?;

    let font_extract_dir_path = paths.app_cache_dir.join("font_extract");
    if font_extract_dir_path.exists() {
        std::fs::remove_dir_all(&font_extract_dir_path)?;
    }
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

    let _ = std::fs::remove_dir_all(&font_extract_dir_path);
    info!(
        "Font installation completed for {} language folder(s)",
        installed_font_count
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    init();
    let mut setting = setting::setting_init();
    info!(
        "Loaded settings with {} cached lock record(s)",
        setting.locks.len()
    );

    let repository_url = get_repository_url();
    let font_url = url::Url::from_str(&get_font_url()).unwrap_or_else(|e| {
        error!("Invalid font URL: {}", e);
        panic!();
    });
    info!("Fetching releases from {}", repository_url);
    println!("Fetching releases...");
    let selected_release = llc::select_release(&repository_url).await.unwrap();
    let asset_list = llc::get_assets(selected_release);
    let selected_asset = select_asset(asset_list).0;
    let download_url = selected_asset.browser_download_url;
    info!("Selected asset: {}", selected_asset.name);
    println!("Selected asset: {}", selected_asset.name);

    let paths = {
        let archive_file_path = PathBuf::from(&selected_asset.name);

        let lbc_data_dir = if crate::path::is_test_mode() {
            PathBuf::from("./test/LimbusCompany_Data")
        } else if cfg!(not(windows)) {
            crate::steam::get_lbc_data_dir_vdf()
        } else {
            #[cfg(windows)]
            crate::steam::windows::get_lbc_data_dir_reg().unwrap_or_else(|e| {
                error!("Detailed Registry Error: {}", e);

                // Print a user-friendly message to stderr before exiting
                eprintln!("Error: Could not find Limbus Company data in the Windows Registry.");
                eprintln!("Try running with --verbose for more details or check your logs.");

                std::process::exit(1); // Exit gracefully instead of panicking
            });
            #[cfg(not(windows))]
            {
                unreachable!("This branch is handled by the cfg!(not(windows)) above")
            }
        };

        Paths::new(archive_file_path, lbc_data_dir)
    };

    create_all_dirs(&paths).unwrap_or_else(|e| {
        error!("Failed to create necessary directories: {}", e);
        panic!();
    });
    let download_file_path = paths.app_cache_dir.join(&paths.archive_file_path);
    let mut asset_lock = Lock::new(
        selected_asset.name.clone(),
        &download_url,
        &download_file_path,
    );
    let mut skip_download = false;

    if let Some(lock) = setting
        .locks
        .iter_mut()
        .find(|l| l.name == selected_asset.name)
        && download_file_path.exists()
    {
        let _ = lock.refresh_checksum();

        if let Some(expected_digest) = &selected_asset.digest {
            let expected_checksum = expected_digest
                .strip_prefix("sha256:")
                .unwrap_or(expected_digest);
            if lock.checksum == expected_checksum {
                info!("Local checksum matches release digest for {}", lock.name);
                skip_download = prompt_confirm(
                    "Asset checksum verified. Skip download?",
                    true,
                    "Skip download confirmation",
                );
            } else {
                warn!(
                    "Checksum mismatch! Local: {}, Expected: {}",
                    lock.checksum, expected_checksum
                );
                let should_redownload = prompt_confirm(
                    "Checksum differs from release. Redownload asset?",
                    true,
                    "Redownload confirmation",
                );

                skip_download = !should_redownload;
            }
        } else {
            warn!("Selected release asset has no digest; checksum verification skipped");
        }
    }

    if !skip_download {
        println!("Downloading from: {}", download_url);
        info!("Downloading asset to {}", download_file_path.display());
        llc::download_asset(download_url, &download_file_path)
            .await
            .expect("Download failed");

        asset_lock.refresh_checksum().ok();

        if let Some(lock) = setting
            .locks
            .iter_mut()
            .find(|l| l.name == selected_asset.name)
        {
            *lock = asset_lock;
        } else {
            setting.locks.push(asset_lock);
        }

        Setting::write(&setting).expect("Failed to save settings");
        info!("Settings updated with latest lock info");
    } else {
        info!("Skipping download for {}", selected_asset.name);
        println!("Skipping download for {}", selected_asset.name);
    }

    println!("Extracting asset...");
    info!(
        "Extracting {} into {}",
        download_file_path.display(),
        paths.lbc_lang_dir.display()
    );

    llc::extract_asset(&download_file_path, &paths.lbc_lang_dir)
        .await
        .unwrap_or_else(|e| {
            error!("Failed to extract asset: {}", e);
            panic!();
        });

    println!("Installing files...");
    info!("Installing extracted files and cleaning temporary artifacts");
    fs::install_and_clean(&paths).unwrap_or_else(|e| {
        error!("Error during move and cleanup: {}", e);
        panic!();
    });

    maybe_install_font(&mut setting, &paths, &font_url)
        .await
        .unwrap_or_else(|e| {
            error!("Font installation failed: {}", e);
            panic!();
        });
}

#[test]
fn language_test() {
    if std::env::var_os("GITHUB_ACTIONS").is_some() {
        return;
    }
    if std::env::var_os("LLC_RUN_INTEGRATION_TESTS").is_none() {
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
