use std::path::PathBuf;

use clap::{Parser, ValueEnum};

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
    // #[arg(long, value_delimiter = ',')]
    // pub(crate) min_size: u64,
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
