use std::path::{Path, PathBuf};

use walkdir::{DirEntry, WalkDir};

use super::DedupError;

pub fn find_files(root: &Path) -> Result<Vec<PathBuf>, DedupError> {
    let files = WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entery| entery.file_type().is_file())
        .map(DirEntry::into_path)
        .collect();

    Ok(files)
}
