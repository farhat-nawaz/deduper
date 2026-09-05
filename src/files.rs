use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::SystemTime,
};

use walkdir::{DirEntry, WalkDir};

use crate::{
    cli::{FileSize, Options},
    error::{DedupError, DedupResult},
};

pub fn find_files(root: &Path, criteria: FileFilter) -> DedupResult<Vec<FileInfo>> {
    WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| should_descend(entry, &criteria))
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
        .is_some_and(|dirs| dirs.contains(&entry.file_name().to_string_lossy().to_string()))
}

fn should_keep(entry: &DirEntry, criteria: &FileFilter) -> bool {
    let path = entry.path();

    if !entry.file_type().is_file() {
        return false;
    }

    if !criteria.include_hidden_files && is_hidden(path) {
        return false;
    }

    // TODO: handle error case properly to maybe terminate scanning
    let Ok(metadata) = entry.metadata() else {
        return false;
    };

    if criteria
        .max_file_size
        .is_some_and(|max| metadata.len() > max.bytes())
    {
        return false;
    }

    if criteria
        .min_file_size
        .is_some_and(|min| metadata.len() < min.bytes())
    {
        return false;
    }

    !path.extension().is_none_or(|ext| {
        criteria
            .exclude_extensions
            .is_some_and(|extensions| extensions.contains(&ext.to_string_lossy().to_lowercase()))
    })
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

pub(crate) struct FileFilter<'a> {
    exclude_extensions: Option<&'a [String]>,
    exclude_dirs: Option<&'a [String]>,
    include_hidden_files: bool,
    max_file_size: Option<FileSize>,
    min_file_size: Option<FileSize>,
}

impl<'a> From<&'a Options> for FileFilter<'a> {
    fn from(value: &'a Options) -> Self {
        Self {
            exclude_extensions: value.exclude_ext.as_deref(),
            exclude_dirs: value.exclude_dir.as_deref(),
            include_hidden_files: value.include_hidden_files,
            max_file_size: value.max_file_size,
            min_file_size: value.min_file_size,
        }
    }
}

#[derive(Debug, Ord, PartialEq, PartialOrd, Eq)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub identity: FileIdentity,
}

impl TryFrom<PathBuf> for FileInfo {
    type Error = DedupError;
    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        let error = |source| DedupError::Metadata {
            path: value.clone(),
            source,
        };
        let metadata = std::fs::metadata(&value).map_err(error)?;
        let modified = metadata.modified().map_err(error)?;

        Ok(FileInfo {
            path: value,
            size: metadata.len(),
            modified: modified,
            identity: FileIdentity {
                dev: metadata.dev(),
                ino: metadata.ino(),
            },
        })
    }
}

#[derive(Debug, Ord, Eq, PartialEq, PartialOrd)]
pub(crate) struct FileIdentity {
    pub(crate) dev: u64,
    pub(crate) ino: u64,
}
