mod config;
mod fs;
mod lang;
mod llc;
mod path;
mod steam;
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
    app_lang: PathBuf,
    lb_data: PathBuf,
    lb_lang: PathBuf,
}

impl Paths {
    fn new(archive: PathBuf, lb_data: PathBuf) -> Self {
        let app_data = path::get_appdata_path();
        let app_cache = app_data.join("cache");
        let app_lang = app_data.join("Lang");
        let lb_lang = lb_data.join("Lang");
        Self {
            archive,
            app_data,
            app_cache,
            app_lang,
            lb_data,
            lb_lang,
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
    std::fs::create_dir_all(&paths.app_data)?;
    std::fs::create_dir_all(&paths.app_lang)
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
        let lb_data = PathBuf::from("./test/LimbusCompany_Data");
        // NOTE: cache path is set to current directory for testing
        // NOTE: lb_data is set to current directory for testing
        Paths::new(archive, lb_data)
    };
    // let _languages = lang::get_languages(&paths.lb_lang);
    // NOTE: enable this for production, it will read the current language from game data and show
    // it in the UI

    create_all_dirs(&paths).unwrap_or_else(|e| {
        error!("Failed to create necessary directories: {}", e);
        panic!();
    });

    info!("Path: {:?}", paths);

    llc::download_and_extract(&paths, download_url)
        .await
        .unwrap_or_else(|e| {
            error!("Error during download and extraction: {}", e);
            panic!();
        });

    fs::move_and_cleanup(&paths).unwrap_or_else(|e| {
        error!("Error during move and cleanup: {}", e);
        panic!();
    });
}

#[tokio::test]
async fn move_test() {}
