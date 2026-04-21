mod path;
mod steam;
use log::{debug, error, info, warn};

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();
    info!("logger initialized");
}
