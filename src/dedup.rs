use std::{fs, path::PathBuf};

use crate::DedupError;

/// By default, the last modified version of the file is kept
pub fn choose_files_to_delete<'a>(
    files: &[Vec<&'a PathBuf>],
) -> Result<Vec<&'a PathBuf>, DedupError> {
    let mut files_to_remove: Vec<&PathBuf> = vec![];
    for group in files {
        let last_modified = group
            .iter()
            .filter_map(|path| {
                let modified = fs::metadata(path).ok()?.created().ok()?;
                Some((*path, modified))
            })
            .min_by_key(|(_, modified)| *modified)
            .map(|(path, _)| path)
            // TODO: Remove unwrap here
            .unwrap();
        files_to_remove.extend(group.iter().copied().filter(|file| *file != last_modified));
    }
    Ok(files_to_remove)
}

pub fn delete_files(files: Vec<&PathBuf>) -> Result<(), DedupError> {
    dbg!(&files);
    for file in files {
        // fs::remove_file(file)?;
        println!("Removing '{}'", file.display());
    }

    Ok(())
}
