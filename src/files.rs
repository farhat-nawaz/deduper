use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use walkdir::{DirEntry, WalkDir};

use crate::{cli::Options, error::DedupError};

pub fn find_files(root: &Path, criteria: FileFilter) -> Result<Vec<FileInfo>, DedupError> {
    WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| should_descend(&entry, &criteria))
        .filter(|entry| match entry {
            Ok(entry) => should_keep(entry, &criteria),
            Err(_) => true,
        })
        .map(|entry| {
            entry.map_err(|source| DedupError::WalkDir {
                path: source
                    .path()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| root.to_path_buf()),
                source,
            })
        })
        .map(|entry| FileInfo::try_from(entry?.into_path()))
        .collect::<Result<Vec<_>, _>>()
}

fn should_descend(entry: &DirEntry, criteria: &FileFilter) -> bool {
    if !entry.file_type().is_dir() {
        return true;
    }

    if !criteria.include_hidden_files && is_hidden(entry.path()) {
        return false;
    }

    !criteria
        .exclude_dirs
        .contains(&entry.file_name().to_string_lossy().to_string())
}

fn should_keep(entry: &DirEntry, criteria: &FileFilter) -> bool {
    let path = entry.path();

    if !entry.file_type().is_file() {
        return false;
    }

    if !criteria.include_hidden_files && is_hidden(path) {
        return false;
    }

    !path.extension().is_none_or(|ext| {
        criteria
            .exclude_extensions
            .contains(&ext.to_string_lossy().to_lowercase())
    })
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

pub(crate) struct FileFilter {
    exclude_extensions: Vec<String>,
    exclude_dirs: Vec<String>,
    include_hidden_files: bool,
}

impl From<&Options> for FileFilter {
    fn from(value: &Options) -> Self {
        Self {
            exclude_extensions: value.exclude_ext.clone(),
            exclude_dirs: value.exclude_dir.clone(),
            include_hidden_files: value.include_hidden_files,
        }
    }
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
