use log::error;
use log::info;
use octocrab::models::repos::Release;
use std::fs::File;
use std::path::PathBuf;
use url::Url;

#[derive(Debug)]
pub struct AssetWarper(pub(crate) octocrab::models::repos::Asset);
impl std::fmt::Display for AssetWarper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.name)
    }
}

pub async fn download_and_extract(
    paths: &crate::Paths,
    download_url: Url,
) -> Result<(), Box<dyn std::error::Error>> {
    let download_path = paths.app_cache.join(&paths.archive);
    download_asset(download_url, &download_path).await?;
    extract_asset(&download_path, &paths.app_lang).await?;
    Ok(())
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

async fn extract_asset(
    archive_path: &PathBuf,
    output_dir: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        "Extrcting file path: {}, output dir: {}",
        archive_path.display(),
        output_dir.display()
    );

    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir)?;
    }
    match archive_path.extension().and_then(|s| s.to_str()) {
        Some("7z") => {
            println!("Extracting 7z archive...");
            sevenz_rust::decompress_file(archive_path, output_dir)?;
        }
        Some("zip") => {
            let file = File::open(archive_path)?;
            let mut archive = zip::ZipArchive::new(file)?;
            let pb = indicatif::ProgressBar::new(archive.len() as u64);
            pb.set_style(indicatif::ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
        .progress_chars("#>-"));
            pb.set_message(format!("Extrcting zip {}", output_dir.display()));

            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;

                let outpath = match file.enclosed_name() {
                    Some(path) => output_dir.join(path),
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
                pb.inc(1);
            }
            pb.finish_with_message("Extrction completed successfully");
        }
        _ => return Err("Unsupported file extension. Use .7z or .zip".into()),
    }

    Ok(())
}

async fn download_asset(url: Url, target_file: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting download from URL: {}", url);
    let client = reqwest::Client::new();

    let response = client
        .get(url)
        .header("User-Agent", "llc-updater")
        .send()
        .await?;

    let content_size = response.content_length().ok_or_else(|| {
        error!("Failed to get content length from GitHub");
        panic!();
    })?;

    info!("Downloading to: {}", target_file.display());

    let pb = indicatif::ProgressBar::new(content_size);
    pb.set_style(indicatif::ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
        .progress_chars("#>-"));
    pb.set_message(format!("Downloading {}", target_file.display()));

    let mut file = std::fs::File::create(target_file)?;
    let mut stream = response.bytes_stream();

    while let Some(item) = futures_util::StreamExt::next(&mut stream).await {
        let chunk = item?;
        std::io::Write::write_all(&mut file, &chunk)?;
        pb.inc(chunk.len() as u64);
    }
    pb.finish_with_message("Download completed successfully");
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

fn parse_github(url_str: &str) -> Option<(String, String)> {
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
