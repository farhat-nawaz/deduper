use std::{path::PathBuf, str::FromStr};

use clap::{Parser, ValueEnum};

use crate::{error::DedupError, files::FileFilter};

#[derive(Parser, Debug)]
#[command(name = "dedup", about = "Find and remove duplicate files")]
pub struct Options {
    /// Root directory to scan
    pub(crate) root_dir: PathBuf,

    /// Whether to actually delete duplicates, or just dry-run
    #[arg(long)]
    pub(crate) delete: bool,

    /// Whether to include hidden files
    #[arg(long)]
    pub(crate) include_hidden_files: bool,

    /// Which file to keep from a duplicate group
    #[arg(long, value_enum, default_value_t = KeepPolicy::Newest)]
    pub(crate) keep: KeepPolicy,

    /// Comma separated list of extensions to be ignored
    #[arg(long, value_delimiter = ',')]
    pub(crate) exclude_ext: Vec<String>,

    /// Comma separated list of directories to be ignored
    #[arg(long, value_delimiter = ',')]
    pub(crate) exclude_dir: Vec<String>,

    /// Files with size greater than this will be ignored
    #[arg(long)]
    pub(crate) max_file_size: Option<FileSize>,

    /// Files with size less than this will be ignored
    #[arg(long)]
    pub(crate) min_file_size: Option<FileSize>,
}

impl Options {
    pub(crate) fn file_filters(&self) -> FileFilter<'_> {
        self.into()
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Action {
    DryRun,
    Delete,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum KeepPolicy {
    Newest,
    Oldest,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FileSize(u64);

impl FileSize {
    pub(crate) fn bytes(&self) -> u64 {
        self.0
    }
}

impl FromStr for FileSize {
    type Err = DedupError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let error = || DedupError::InvalidArgument {
            message: format!("Invalid file size: {}", s),
        };
        let position = s.find(|c: char| !c.is_ascii_digit()).ok_or_else(error)?;

        let (number, unit) = s.split_at(position);
        let number: u64 = number.parse().map_err(|_| error())?;

        let multiplier = match unit.to_ascii_lowercase().as_str() {
            "b" => 1,
            "kb" => 1_000,
            "mb" => 1_000_000,
            "gb" => 1_000_000_000,
            _ => return Err(error()),
        };

        Ok(FileSize(number.checked_mul(multiplier).ok_or_else(error)?))
    }
}
