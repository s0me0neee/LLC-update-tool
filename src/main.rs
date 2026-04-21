mod config;
mod lang;
mod llc;
mod path;
mod steam;
use inquire::InquireError;
use log::{debug, error, info, warn};
use std::{
    path::{Path, PathBuf},
    process::exit,
    time::Duration,
};

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

fn init() {
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

#[tokio::main]
async fn main() {
    init();
    let url = "https://github.com/LocalizeLimbusCompany/LocalizeLimbusCompany";
    let release = llc::get_release(url).await.unwrap();

    let assets = llc::get_assets(release);
    let asset = inquire::Select::new("Select an asset to download:", assets)
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
        });

    let download_url = asset.0.browser_download_url.as_ref();

    info!("Downloading from: {}", download_url);
    let file_name = PathBuf::from(&asset.0.name);
    let target_path = PathBuf::from("./test/");
    let cache_path = path::get_cache_path().join(&file_name);
    let file_path = target_path.join(&file_name);
    info!("Cache path: {}", cache_path.display());

    std::fs::create_dir_all(&target_path).unwrap_or_else(|e| {
        error!(
            "Failed to creat target path: {}, Error: {}",
            &target_path.display(),
            e
        )
    });

    llc::download_asset(download_url, &file_path)
        .await
        .unwrap_or_else(|e| {
            println!("Error downloading asset: {}", e);
            panic!();
        });

    llc::extract_asset(&file_path, &target_path).unwrap_or_else(|e| {
        println!("Error extracting asset: {}", e);
        panic!();
    });
}
