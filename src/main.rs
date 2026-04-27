mod fs;
mod lang;
mod llc;
mod lock;
mod path;
mod steam;
use futures_util::future::err;
use inquire::InquireError;
use llc::AssetWarper;
use log::{error, info, warn};
use std::{path::PathBuf, process::exit};

#[macro_export]
macro_rules! env_dbg_init {
    () => {
        env_logger::builder()
            .filter_level(log::LevelFilter::max())
            .parse_default_env()
            .init();
        info!("logger initialized");
    };
}

#[derive(Debug)]
struct Paths {
    archive: PathBuf,
    app_data: PathBuf,
    app_cache: PathBuf,
    lbc_data: PathBuf,
    lbc_lang: PathBuf,
}

impl Paths {
    fn new(archive: PathBuf, lbc_data: PathBuf) -> Self {
        let app_data = path::get_appdata_path();
        let app_cache = app_data.join("cache");
        let lbc_lang = lbc_data.join("Lang");
        Self {
            archive,
            app_data,
            app_cache,
            lbc_data,
            lbc_lang,
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

fn select_asset(assets: Vec<AssetWarper>) -> AssetWarper {
    inquire::Select::new("Select an asset to download:", assets)
        .prompt()
        .unwrap_or_else(|match_err| match match_err {
            InquireError::OperationCanceled => {
                println!("\nSelection canceled by user.");
                exit(0);
            }
            InquireError::OperationInterrupted => {
                println!("\nProcess interrupted (Ctrl+C). Cleaning up...");
                exit(0);
            }
            _ => {
                eprintln!("\nAn error occurred: {}", match_err);
                exit(1);
            }
        })
}

fn create_all_dirs(paths: &Paths) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(&paths.app_cache)?;
    if !paths.lbc_data.exists() {
        let ans = inquire::Confirm::new("Can't find Limbus Company data dir, creat one?")
            .with_default(false)
            .prompt();
        return match ans {
            Ok(true) => {
                std::fs::create_dir_all(&paths.lbc_lang)?;
                info!("Created dir {}", &paths.lbc_lang.display());
                Ok(())
            }
            Ok(false) => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Can't find Limbus Company data dir",
            )),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Interrupted, e)),
        };
    }
    Ok(())
}

pub fn get_repository_url() -> String {
    let default_url = "https://github.com/LocalizeLimbusCompany/LocalizeLimbusCompany";

    let prompt_message = "Enter GitHub repository URL: ".to_string();

    let url = inquire::Text::new(&prompt_message)
        .with_default(default_url)
        .with_help_message("Press Enter to use the default Limbus Localize repo")
        .prompt();

    match url {
        Ok(val) => val,
        Err(_) => {
            println!("\nOperation canceled.");
            exit(0);
        }
    }
}

#[tokio::main]
async fn main() {
    init();

    let url = &get_repository_url();
    let release = llc::select_release(url).await.unwrap();
    let assets = llc::get_assets(release);
    let asset = select_asset(assets).0;
    let download_url = asset.browser_download_url;
    println!("Downloading from: {}", download_url);

    let paths = {
        // TODO: complete refactor the paths construction, this is just for testing
        let archive = PathBuf::from(&asset.name);

        #[cfg(not(windows))]
        let lbc_data = crate::steam::get_lbc_data_dir_vdf();

        let lbc_data = PathBuf::from("./test/LimbusCompany_Data");
        // NOTE: cache path is set to current directory for testing
        // NOTE: lb_data is set to current directory for testing
        Paths::new(archive, lbc_data)
    };

    let languages = lang::get_languages(&paths.lbc_lang);
    dbg!(&languages);
    // NOTE: enable this for production, it will read the current language from game data and show
    // it in the UI

    create_all_dirs(&paths).unwrap_or_else(|e| {
        error!("Failed to create necessary directories: {}", e);
        panic!();
    });

    info!("Path: {:?}", paths);

    let download_path = paths.app_cache.join(&paths.archive);
    llc::download_asset(download_url, &download_path)
        .await
        .unwrap_or_else(|e| {
            error!("Failed to download asset: {}", e);
            panic!();
        });
    llc::extract_asset(&download_path, &paths.lbc_lang)
        .await
        .unwrap_or_else(|e| {
            error!("Failed to extract asset: {}", e);
            panic!();
        });

    // TODO:build a lock file from download, and check locked cache before downloading, to avoid
    // unnecessary downloads and extractions

    fs::install_and_clean(&paths).unwrap_or_else(|e| {
        error!("Error during move and cleanup: {}", e);
        panic!();
    });
}

#[test]
fn language_test() {
    env_dbg_init!();
    let paths = {
        let archive = PathBuf::from("test_none");

        #[cfg(windows)]
        let lbc_data = crate::steam::windows::get_lbc_data_dir_reg().unwrap();

        #[cfg(not(windows))]
        let lbc_data = crate::steam::get_lbc_data_dir_vdf();

        dbg!(&lbc_data);
        Paths::new(archive, lbc_data)
    };
    let languages = lang::get_languages(&paths.lbc_lang);
    dbg!(&languages);
}
