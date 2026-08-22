use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "dedup", about = "Find and remove duplicate files")]
pub struct Options {
    /// Root directory to scan
    pub(crate) root_dir: PathBuf,

    /// whether to actually delete duplicates, or just dry-run
    #[arg(long)]
    pub(crate) delete: bool,

    /// which file to keep
    #[arg(long, value_enum, default_value_t = KeepPolicy::Newest)]
    pub(crate) keep: KeepPolicy,
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
