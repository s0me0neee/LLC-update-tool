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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_lang_dir(suffix: &str) -> (PathBuf, PathBuf, Paths) {
        let root = std::env::temp_dir().join(format!(
            "llc-fs-test-{}-{}",
            suffix,
            std::process::id()
        ));
        let lbc_data_dir = root.join("LimbusCompany_Data");
        let lang_dir = lbc_data_dir.join("Lang");
        fs::create_dir_all(&lang_dir).unwrap();
        let paths = crate::Paths::new(PathBuf::from("dummy"), lbc_data_dir);
        (root, lang_dir, paths)
    }

    #[test]
    fn install_and_clean_flattens_nested_structure() {
        let (root, lang_dir, paths) = temp_lang_dir("nested");

        let nested = lang_dir
            .join("LimbusCompany_Data")
            .join("Lang")
            .join("LLC_zh-CN");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("test.json"), b"{}").unwrap();

        install_and_clean(&paths).unwrap();

        assert!(
            lang_dir.join("LLC_zh-CN").join("test.json").exists(),
            "lang file should be moved to flat location"
        );
        assert!(
            !lang_dir.join("LimbusCompany_Data").exists(),
            "redundant wrapper dir should be removed"
        );

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn install_and_clean_noop_when_already_flat() {
        let (root, lang_dir, paths) = temp_lang_dir("flat");

        let lang_subdir = lang_dir.join("LLC_zh-CN");
        fs::create_dir_all(&lang_subdir).unwrap();
        fs::write(lang_subdir.join("test.json"), b"{}").unwrap();

        install_and_clean(&paths).unwrap();

        assert!(
            lang_dir.join("LLC_zh-CN").join("test.json").exists(),
            "existing flat files should be untouched"
        );

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn install_and_clean_replaces_existing_lang_dir() {
        let (root, lang_dir, paths) = temp_lang_dir("replace");

        // Pre-existing flat language dir
        let existing = lang_dir.join("LLC_zh-CN");
        fs::create_dir_all(&existing).unwrap();
        fs::write(existing.join("old.json"), b"old").unwrap();

        // Nested structure (simulating fresh extraction) with updated content
        let nested = lang_dir
            .join("LimbusCompany_Data")
            .join("Lang")
            .join("LLC_zh-CN");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("new.json"), b"new").unwrap();

        install_and_clean(&paths).unwrap();

        let final_lang = lang_dir.join("LLC_zh-CN");
        assert!(
            final_lang.join("new.json").exists(),
            "new file should be present after replacement"
        );
        assert!(
            !final_lang.join("old.json").exists(),
            "old file should be gone after replacement"
        );
        assert!(
            !lang_dir.join("LimbusCompany_Data").exists(),
            "redundant wrapper dir should be removed"
        );

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn install_and_clean_handles_multiple_lang_dirs() {
        let (root, lang_dir, paths) = temp_lang_dir("multi");

        for lang in ["LLC_zh-CN", "LLC_zh-TW"] {
            let nested = lang_dir
                .join("LimbusCompany_Data")
                .join("Lang")
                .join(lang);
            fs::create_dir_all(&nested).unwrap();
            fs::write(nested.join("file.json"), b"{}").unwrap();
        }

        install_and_clean(&paths).unwrap();

        for lang in ["LLC_zh-CN", "LLC_zh-TW"] {
            assert!(
                lang_dir.join(lang).join("file.json").exists(),
                "{lang} file should be at flat location"
            );
        }
        assert!(!lang_dir.join("LimbusCompany_Data").exists());

        fs::remove_dir_all(root).ok();
    }
}

