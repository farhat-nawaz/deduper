use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use walkdir::{DirEntry, WalkDir};

use crate::error::DedupError;

pub fn find_files(root: &Path) -> Result<Vec<FileInfo>, DedupError> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(DirEntry::into_path)
        .map(FileInfo::try_from)
        .collect()

    // Ok(files)
}

#[derive(Debug, Ord, PartialEq, PartialOrd, Eq)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
}

impl TryFrom<PathBuf> for FileInfo {
    type Error = DedupError;
    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        let metadata = std::fs::metadata(&value)?;
        Ok(FileInfo {
            path: value,
            size: metadata.len(),
            modified: metadata.modified()?,
        })
    }
}
