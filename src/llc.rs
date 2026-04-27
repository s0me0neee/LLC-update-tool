use log::{error, info, warn};
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

#[derive(Debug)]
pub struct ReleaseWrapper(pub(crate) Release);
impl std::fmt::Display for ReleaseWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tag = &self.0.tag_name;

        let name_opt = self.0.name.as_deref().filter(|n| !n.is_empty());

        let parsed_name = if let Some(name) = name_opt {
            if name.len() >= 10 {
                let year = &name[0..4];
                let month = &name[4..6];
                let day = &name[6..8];
                let rev = &name[8..10];
                format!("{}-{}-{} [#{}]", year, month, day, rev)
            } else {
                name.to_string()
            }
        } else {
            "No Release Name".to_string()
        };

        write!(f, "Tag: {} - ({})", tag, parsed_name)
    }
}

pub fn get_assets(release: Release) -> Vec<AssetWarper> {
    let release_tag = release.tag_name.clone();
    let wrapped_assets = release
        .assets
        .into_iter()
        .map(AssetWarper)
        .collect::<Vec<_>>();
    info!("Selected release tag: {}", release_tag);
    info!(
        "Found {} asset(s) in selected release",
        wrapped_assets.len()
    );
    wrapped_assets
}

pub async fn extract_asset(
    archive_file_path: &Path,
    output_dir_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        "Extracting archive {} into {}",
        archive_file_path.display(),
        output_dir_path.display()
    );

    if !output_dir_path.exists() {
        info!(
            "Output directory does not exist, creating {}",
            output_dir_path.display()
        );
        std::fs::create_dir_all(output_dir_path)?;
    }

    let archive_extension = archive_file_path.extension().and_then(|ext| ext.to_str());
    match archive_extension {
        Some("7z") => {
            println!("Extracting 7z archive...");
            sevenz_rust::decompress_file(archive_file_path, output_dir_path)?;
            info!("7z extraction completed successfully");
        }
        Some("zip") => {
            let zip_file = File::open(archive_file_path)?;
            let mut zip_archive = zip::ZipArchive::new(zip_file)?;
            let pb = indicatif::ProgressBar::new(zip_archive.len() as u64);
            pb.set_style(indicatif::ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
        .progress_chars("#>-"));
            pb.set_message(format!("Extracting zip into {}", output_dir_path.display()));

            for index in 0..zip_archive.len() {
                let mut zip_entry = zip_archive.by_index(index)?;

                let output_path = match zip_entry.enclosed_name() {
                    Some(path) => output_dir_path.join(path),
                    None => {
                        warn!(
                            "Skipping zip entry at index {} due to suspicious path",
                            index
                        );
                        pb.inc(1);
                        continue;
                    }
                };

                if zip_entry.is_dir() {
                    std::fs::create_dir_all(&output_path)?;
                } else {
                    let output_parent = output_path.parent();
                    if let Some(parent_dir) = output_parent {
                        if !parent_dir.exists() {
                            std::fs::create_dir_all(parent_dir)?;
                        }
                    }
                    let mut output_file = File::create(&output_path)?;
                    std::io::copy(&mut zip_entry, &mut output_file)?;
                }
                pb.inc(1);
            }
            pb.finish_with_message("Extraction completed successfully");
            info!("Zip extraction completed successfully");
        }
        _ => {
            error!(
                "Unsupported archive extension for {}. Expected .7z or .zip",
                archive_file_path.display()
            );
            return Err("Unsupported file extension. Use .7z or .zip".into());
        }
    }

    Ok(())
}

pub async fn download_asset(
    download_url: Url,
    target_file_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting download from URL: {}", download_url);
    let client = reqwest::Client::new();

    let response = client
        .get(download_url.clone())
        .header("User-Agent", "llc-updater")
        .send()
        .await?
        .error_for_status()?;

    let content_size = match response.content_length() {
        Some(size) => size,
        None => {
            error!("Failed to get content length from {}", download_url);
            return Err("Missing content length header in response".into());
        }
    };

    info!("Downloading to: {}", target_file_path.display());

    let pb = indicatif::ProgressBar::new(content_size);
    pb.set_style(indicatif::ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")?
        .progress_chars("#>-"));
    pb.set_message(format!("Downloading {}", target_file_path.display()));

    let mut output_file = std::fs::File::create(target_file_path)?;
    let mut stream = response.bytes_stream();
    let mut downloaded_bytes = 0u64;

    while let Some(item) = futures_util::StreamExt::next(&mut stream).await {
        let chunk = item?;
        std::io::Write::write_all(&mut output_file, &chunk)?;
        downloaded_bytes += chunk.len() as u64;
        pb.inc(chunk.len() as u64);
    }
    pb.finish_with_message("Download completed successfully");
    info!(
        "Download completed: {} byte(s) written to {}",
        downloaded_bytes,
        target_file_path.display()
    );
    Ok(())
}

pub async fn get_releases(url: &str) -> Result<Vec<ReleaseWrapper>, Box<dyn std::error::Error>> {
    let octo = octocrab::instance();
    info!("Fetching releases from GitHub URL: {}", url);

    let (owner_name, repo_name) = parse_github(url).ok_or_else(|| {
        error!("Failed to parse GitHub URL: {}", url);
        "Invalid GitHub URL format"
    })?;

    let page = octo
        .repos(owner_name.clone(), repo_name.clone())
        .releases()
        .list()
        .per_page(5)
        .send()
        .await?;

    let releases = page
        .items
        .into_iter()
        .map(ReleaseWrapper)
        .collect::<Vec<_>>();

    if releases.is_empty() {
        return Err("No releases found for this repository.".into());
    }

    info!(
        "Found {} release(s) for {}/{}. Newest tag: {}",
        releases.len(),
        owner_name,
        repo_name,
        releases[0].0.tag_name
    );

    Ok(releases)
}

pub async fn select_release(url: &str) -> Result<Release, Box<dyn std::error::Error>> {
    let releases = get_releases(url).await?;

    let selection = inquire::Select::new("Select a release to download:", releases)
        .with_help_message("↑/↓ to navigate, Enter to select (Top is latest)")
        .prompt();

    match selection {
        Ok(release_wrapper) => {
            info!("Selected release: {}", release_wrapper.0.tag_name);
            Ok(release_wrapper.0)
        }
        Err(err) => {
            warn!("Release selection prompt ended: {}", err);
            println!("\nRelease selection canceled.");
            std::process::exit(0);
        }
    }
}

fn parse_github(url_str: &str) -> Option<(String, String)> {
    let url = Url::parse(url_str).ok()?;
    if url.domain() != Some("github.com") {
        return None;
    }

    let mut path_segments = url.path_segments()?;
    let owner = path_segments.next()?;
    let repo_segment = path_segments.next()?;

    let repo = repo_segment.strip_suffix(".git").unwrap_or(repo_segment);

    if owner.is_empty() || repo.is_empty() {
        return None;
    }

    Some((owner.to_string(), repo.to_string()))
}
