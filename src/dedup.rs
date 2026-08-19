use std::{fs, path::PathBuf};

use crate::error::DedupError;
use anyhow::Context;

/// By default, the last modified version of the file is kept
pub fn choose_files_to_delete<'a>(
    files: &[Vec<&'a PathBuf>],
) -> Result<Vec<&'a PathBuf>, DedupError> {
    let mut files_to_remove: Vec<&PathBuf> = vec![];
    for group in files {
        let last_modified = group
            .iter()
            .filter_map(|path| {
                let modified = fs::metadata(path).ok()?.modified().ok()?;
                Some((*path, modified))
            })
            .max_by_key(|(_, modified)| *modified)
            .map(|(path, _)| path);
        if let Some(last_modified) = last_modified {
            files_to_remove.extend(group.iter().copied().filter(|file| *file != last_modified));
        }
    }
    Ok(files_to_remove)
}

pub fn delete_files(files: Vec<&PathBuf>) -> anyhow::Result<()> {
    for file in files {
        fs::File::open(file).with_context(|| format!("Could not open file {}", file.display()))?;
        // fs::remove_file(file)?;
        println!("Removing '{}'", file.display());
    }

    Ok(())
}
