#[cfg(not(unix))]
compile_error!("deduper currently supports Unix-like systems only");

mod cli;
mod dedup;
mod duplicates;
mod error;
mod files;
mod fingerprint;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    let options = cli::Options::parse();

    dedup::run(options)?;

    Ok(())
}
