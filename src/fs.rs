use fs_extra::dir::{CopyOptions, move_dir};
use log::info;
use std::fs;

use crate::Paths;

pub fn install_and_clean(paths: &Paths) -> Result<(), Box<dyn std::error::Error>> {
    let extracted_lang_dir_path = paths.lbc_lang_dir.join("LimbusCompany_Data").join("Lang");
    let target_lang_dir_path = &paths.lbc_lang_dir;

    if !extracted_lang_dir_path.exists() {
        info!(
            "Extracted inner language directory not found at {}; skipping move step",
            extracted_lang_dir_path.display()
        );
        return Ok(());
    }

    info!(
        "Moving extracted language content from {} to {}",
        extracted_lang_dir_path.display(),
        target_lang_dir_path.display()
    );

    let mut move_options = CopyOptions::new();
    move_options.overwrite = true;
    move_options.copy_inside = true;

    for dir_entry in fs::read_dir(&extracted_lang_dir_path)? {
        let dir_entry = dir_entry?;
        let source_entry_path = dir_entry.path();
        let entry_name = source_entry_path.file_name().ok_or("Invalid file name")?;
        let target_entry_path = target_lang_dir_path.join(entry_name);

        if target_entry_path.exists() {
            if target_entry_path.is_dir() {
                info!(
                    "Removing existing directory: {}",
                    target_entry_path.display()
                );
                fs::remove_dir_all(&target_entry_path)?;
            } else {
                info!("Removing existing file: {}", target_entry_path.display());
                fs::remove_file(&target_entry_path)?;
            }
        }

        if source_entry_path.is_dir() {
            info!(
                "Moving directory {} into {}",
                source_entry_path.display(),
                target_lang_dir_path.display()
            );
            move_dir(&source_entry_path, target_lang_dir_path, &move_options)?;
            continue;
        }

        info!(
            "Moving file {} to {}",
            source_entry_path.display(),
            target_entry_path.display()
        );
        fs::rename(&source_entry_path, &target_entry_path)?;
    }

    let redundant_extract_root_dir_path = paths.lbc_lang_dir.join("LimbusCompany_Data");
    if redundant_extract_root_dir_path.exists() {
        info!(
            "Cleaning up redundant extracted root directory: {}",
            redundant_extract_root_dir_path.display()
        );
        fs::remove_dir_all(&redundant_extract_root_dir_path)?;
    }

    info!("Language installation and cleanup completed successfully");
    Ok(())
}

pub fn copy_to_lbc(_paths: &Paths) {}
