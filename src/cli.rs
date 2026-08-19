use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "dedup")]
#[command(about = "Find and remove duplicate files")]
pub struct Options {
    /// Root directory to scan
    pub(crate) root_dir: PathBuf,

    /// whether to actually delete duplicates, or just dry-run
    #[arg(long, value_enum, default_value_t = Action::DryRun)]
    pub(crate) delete: Action,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Action {
    DryRun,
    Delete,
}
