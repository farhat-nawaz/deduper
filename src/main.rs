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

use std::{collections::HashMap, path::PathBuf};

mod error;
mod files;
mod fingerprint;

pub use error::DedupError;
use fingerprint::{Fingerprint, fingerprint_file};

fn main() {
    let root_dir = PathBuf::from("/Users/farhatnawaz/Developer/projects/rust/deduper/test_data/");
    let all_files = files::find_files(&root_dir).unwrap();
    let mut fingerprints: HashMap<Fingerprint, Vec<PathBuf>> = HashMap::new();

    for file in all_files {
        let fingerprint = fingerprint_file(&file).unwrap();
        fingerprints
            .entry(fingerprint)
            .and_modify(|entries| entries.push(file.clone()))
            .or_insert(vec![file]);
    }
    println!("{:?}", fingerprints);
}
