use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "dedup")]
#[command(about = "Find and remove duplicate files")]
pub struct Options {
    /// Root directory to scan
    pub(crate) root_dir: PathBuf,

    /// Don't delete anything; only report duplicates
    #[arg(long, default_value_t = true)]
    pub(crate) dry_run: bool,
}
