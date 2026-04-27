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
use std::{path::PathBuf, process::exit};

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
        .filter_level(log::LevelFilter::Info)
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

#[tokio::main]
async fn main() {
    init();
    let mut setting = setting::setting_init();
    info!(
        "Loaded settings with {} cached lock record(s)",
        setting.locks.len()
    );

    let repository_url = get_repository_url();
    let font_url = get_font_url();
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

        let lbc_data_dir = if std::env::var_os("TEST").is_some() {
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

    #[cfg(windows)]
    // NOTE: enable this for production, it will read the current language from game data and show
    {
        let languages = lang::get_languages(&paths.lbc_lang_dir);
        dbg!(&languages);
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
