use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DedupError {
    #[error("failed to read file")]
    FileRead(#[from] std::io::Error),
    #[error("failed to open file `{path}`")]
    Open {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read file `{path}`")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to read metadata for `{path}`")]
    Metadata {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
