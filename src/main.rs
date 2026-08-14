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

use std::path::PathBuf;

mod duplicates;
mod error;
mod files;
mod fingerprint;

pub use error::DedupError;

fn main() {
    let root_dir = PathBuf::from("/Users/farhatnawaz/Developer/projects/rust/deduper/test_data/");
    let files = files::find_files(&root_dir).unwrap();
    let candidates = duplicates::find_candidates(&files).unwrap();
    let duplicates = duplicates::find_duplicates(&candidates);
    dbg!("{:?}", duplicates);
}
