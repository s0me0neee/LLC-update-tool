use log::error;
use log::info;
use octocrab::models::repos::Release;
use std::fs::File;
use std::path::Path;
use url::Url;

#[derive(Debug)]
pub struct AssetWarper(pub(crate) octocrab::models::repos::Asset);
impl std::fmt::Display for AssetWarper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.name)
    }
}

pub fn get_assets(release: Release) -> Vec<AssetWarper> {
    let assets = release
        .assets
        .into_iter()
        .map(AssetWarper)
        .collect::<Vec<_>>();
    info!("Latest release: {}", release.tag_name);
    info!("{} assets found", assets.len());
    assets
}

pub fn extract_asset(file_path: &str, output_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(file_path);
    let output_path = Path::new(output_dir);

    if !output_path.exists() {
        std::fs::create_dir_all(output_path)?;
    }
    match path.extension().and_then(|s| s.to_str()) {
        Some("7z") => {
            info!("Extracting 7z archive...");
            sevenz_rust::decompress_file(file_path, output_dir)?;
        }
        Some("zip") => {
            info!("Extracting ZIP archive...");
            let file = File::open(file_path)?;
            let mut archive = zip::ZipArchive::new(file)?;

            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;

                let outpath = match file.enclosed_name() {
                    Some(path) => output_path.join(path),
                    None => continue, // Skip files with suspicious paths
                };

                if file.is_dir() {
                    std::fs::create_dir_all(&outpath)?;
                } else {
                    if let Some(p) = outpath.parent()
                        && !p.exists()
                    {
                        std::fs::create_dir_all(p)?;
                    }
                    let mut outfile = File::create(&outpath)?;
                    std::io::copy(&mut file, &mut outfile)?;
                }
            }
        }
        _ => return Err("Unsupported file extension. Use .7z or .zip".into()),
    }

    info!("Extraction complete!");
    Ok(())
}

pub async fn download_asset(
    url: &str,
    target_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting download from URL: {}", url);
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("User-Agent", "llc-updater")
        .send()
        .await?;

    let mut file = std::fs::File::create(target_path)?;
    let mut stream = response.bytes_stream();

    while let Some(item) = futures_util::StreamExt::next(&mut stream).await {
        let chunk = item?;
        std::io::Write::write_all(&mut file, &chunk)?;
    }
    info!("Download completed successfully");
    Ok(())
}

pub async fn get_release(url: &str) -> Result<Release, Box<dyn std::error::Error>> {
    let octo = octocrab::instance();
    info!("Using GitHub URL: {}", url);
    let (owner, repo) = parse_github(url)
        .ok_or_else(|| {
            error!("Failed to parse GitHub URL: {}", url);
            panic!();
        })
        .unwrap();

    let latest = octo.repos(owner, repo).releases().get_latest().await?;
    Ok(latest)
}

pub fn parse_github(url_str: &str) -> Option<(String, String)> {
    let url = Url::parse(url_str).ok()?;
    if url.domain() != Some("github.com") {
        return None;
    }

    let mut segments = url.path_segments()?;
    let owner = segments.next()?;
    let repo_raw = segments.next()?; // Get the second segment

    let repo = repo_raw.strip_suffix(".git").unwrap_or(repo_raw);

    if owner.is_empty() || repo.is_empty() {
        return None;
    }

    Some((owner.to_string(), repo.to_string()))
}
