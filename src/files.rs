use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use walkdir::WalkDir;

use crate::error::DedupError;

pub fn find_files(root: &Path, exclude: &[String]) -> Result<Vec<FileInfo>, DedupError> {
    let mut files = Vec::new();

    for entry in WalkDir::new(root) {
        let entry = entry.map_err(|source| DedupError::WalkDir {
            path: source
                .path()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| root.to_path_buf()),
            source,
        })?;

        if !entry.file_type().is_file() {
            continue;
        }

        if entry
            .path()
            .extension()
            .is_some_and(|ext| exclude.contains(&ext.to_string_lossy().to_lowercase()))
        {
            continue;
        }

        files.push(FileInfo::try_from(entry.into_path())?);
    }

    Ok(files)
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
