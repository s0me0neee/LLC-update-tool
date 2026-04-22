use fs_extra::dir::{CopyOptions, move_dir};
use log::info;
use std::fs;

pub fn move_and_cleanup(paths: &crate::Paths) -> Result<(), Box<dyn std::error::Error>> {
    let inner_lang_dir = paths.app_lang.join("LimbusCompany_Data").join("Lang");
    let outer_lang_dir = &paths.app_lang;

    if inner_lang_dir.exists() {
        info!("Moving all contents from inner Lang to outer Lang");
        let mut options = CopyOptions::new();
        options.overwrite = true;
        options.copy_inside = true;

        for entry in fs::read_dir(&inner_lang_dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = path.file_name().ok_or("Invalid file name")?;
            let dest_path = outer_lang_dir.join(name);

            if dest_path.exists() {
                if dest_path.is_dir() {
                    info!("Removing old directory: {:?}", dest_path);
                    fs::remove_dir_all(&dest_path)?;
                } else {
                    info!("Removing old file: {:?}", dest_path);
                    fs::remove_file(&dest_path)?;
                }
            }

            if path.is_dir() {
                move_dir(&path, outer_lang_dir, &options)?;
            } else {
                fs::rename(&path, &dest_path)?;
            }
        }

        let redundant_root = paths.app_lang.join("LimbusCompany_Data");
        if redundant_root.exists() {
            info!("Cleaning up redundant LimbusCompany_Data folder...");
            fs::remove_dir_all(redundant_root)?;
        }
    } else {
        info!("Inner Lang directory not found; skipping move.");
    }
    Ok(())
}
