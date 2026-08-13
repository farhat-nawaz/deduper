use std::{collections::HashMap, path::PathBuf};

use crate::error::DedupError;
use crate::fingerprint::{Fingerprint, fingerprint_file};

pub fn find_candidates(
    files: &[PathBuf],
) -> Result<HashMap<Fingerprint, Vec<&PathBuf>>, DedupError> {
    let mut fingerprints: HashMap<Fingerprint, Vec<&PathBuf>> = HashMap::new();

    for file in files {
        let fingerprint = fingerprint_file(&file).unwrap();
        fingerprints
            .entry(fingerprint)
            .and_modify(|entries| entries.push(&file))
            .or_insert(vec![file]);
    }

    Ok(fingerprints)
}
