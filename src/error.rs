use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DedupError {
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

    #[error("failed to read metadata for `{path}`")]
    Metadata {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read directory `{path}`")]
    WalkDir {
        path: PathBuf,
        #[source]
        source: walkdir::Error,
    },

    #[error("{message}")]
    InvalidArgument { message: String },

    #[error("file Identity changed since discovery. Skipping... {path}")]
    IdentityChanged { path: PathBuf },

    #[error("failed to delete file `{path}`")]
    Delete {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    // #[error("failed to read file")]
    // FileRead(#[from] std::io::Error),
}

pub(crate) type DedupResult<T> = Result<T, DedupError>;
