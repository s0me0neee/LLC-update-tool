mod config;
mod lang;
mod llc;
mod path;
mod steam;
use log::{debug, error, info, warn};
use std::{path::Path, time::Duration};

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
    let asset = inquire::Select::new("Select a asset to download:", assets)
        .prompt()
        .map_err(|e| {
            error!("Error selecting asset: {}", e);
            e
        })
        .unwrap();

    let download_url = asset.0.browser_download_url.as_ref();

    info!("Downloading from: {}", download_url);

    let target_path = "./test/".to_string() + &asset.0.name.to_string();

    llc::download_asset(download_url, target_path.as_str())
        .await
        .unwrap_or_else(|e| {
            error!("Error downloading asset: {}", e);
            panic!();
        });

    llc::extract_asset(target_path.as_str(), "./test/").unwrap_or_else(|e| {
        error!("Error extracting asset: {}", e);
        panic!();
    });
}
