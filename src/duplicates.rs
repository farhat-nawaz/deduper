use std::{collections::HashMap, fs, io, path::PathBuf};

use blake3::{Hash, Hasher};

use crate::error::DedupError;
use crate::fingerprint::{Fingerprint, fingerprint_file};

pub fn find_candidates(
    files: &[PathBuf],
) -> Result<HashMap<Fingerprint, Vec<&PathBuf>>, DedupError> {
    let mut fingerprints: HashMap<Fingerprint, Vec<&PathBuf>> = HashMap::new();

    for file in files {
        let fingerprint = fingerprint_file(&file).unwrap();
        fingerprints.entry(fingerprint).or_default().push(file);
    }

    fingerprints.retain(|_, paths| paths.len() > 1);
    Ok(fingerprints)
}

pub fn find_duplicates<'a>(
    candidates: &HashMap<Fingerprint, Vec<&'a PathBuf>>,
) -> Result<Vec<Vec<&'a PathBuf>>, DedupError> {
    let mut duplicates: HashMap<Hash, Vec<&PathBuf>> = HashMap::new();
    for file_paths in candidates.values() {
        for file_path in file_paths {
            eprintln!("Processing {}", file_path.display());

            let hash = hash_file(file_path)?;
            duplicates.entry(hash).or_default().push(file_path);
        }
    }
    duplicates.retain(|_, paths| paths.len() > 1);

    Ok(duplicates.into_values().collect())
}

pub fn hash_file(path: &PathBuf) -> Result<Hash, DedupError> {
    let mut hasher = Hasher::new();
    let mut file = fs::File::open(path)?;
    io::copy(&mut file, &mut hasher);

    Ok(hasher.finalize())
}
