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

    dedup::run(options)?;

    Ok(())
}

// Further Evolution
//
// TODO: KeepPolicy
//
//  - shortest path
//  - longest path
//
// TODO: File size
//
//  - min
//  - max
//
// TODO: File extensions
//
//  - --exclude mp4,iso
//
// TODO: Hidden Files
//
//  - --include-hidden
//
// TODO: Symlinks
//
//  - --follow-symlinks
//
// TODO: Directory Exclusion
//
//  - --exclude node_modules
//  - --exclude .git
//  - --exclude target
//
// FIXME: What should happen when files change between discovery and deletion?
//
// DONE: Reporting
//
//    Duplicate group:
//      KEEP    /photos/2024/vacation.jpg
//      DELETE  /backup/vacation.jpg
//      DELETE  /old/vacation.jpg
//
//      2 files, 8.4 MB reclaimable
//
//    Duplicate group:
//      KEEP    /documents/report.pdf
//      DELETE  /downloads/report.pdf
//
//      1 file, 2.1 MB reclaimable
//
//    Summary:
//      Duplicate groups: 2
//      Files to delete: 3
//      Space reclaimable: 10.5 MB
//
// TODO: Confirmation before deletion
//
//  - $ dedup ~/photos --delete
//    3 files will be deleted (14.2 MB).
//    Continue? [y/N]
//  - dedup ~/photos --delete --yes
