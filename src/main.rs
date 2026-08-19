//Find all files under a directory.
//
//For each file:
//    get its size
//    read its first 1 MB
//    calculate a fingerprint
//    put it into a bucket based on that fingerprint
//
//For each bucket containing multiple files:
//    compare the files' complete contents
//    identify actual duplicates
//
//For each duplicate group:
//    determine which file has the newest modification time
//    keep that file
//    remove the others

mod cli;
mod dedup;
mod duplicates;
mod error;
mod files;
mod fingerprint;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    let options = cli::Options::parse();

    let files = files::find_files(&options.root_dir)?;
    let candidates = duplicates::find_candidates(&files)?;
    let duplicates = duplicates::find_duplicates(&candidates)?;
    let files_to_delete = dedup::choose_files_to_delete(&duplicates)?;
    dedup::delete_files(files_to_delete)?;
    // dbg!(&duplicates);

    Ok(())
}
