use thiserror::Error;

#[derive(Debug, Error)]
pub enum DedupError {
    #[error("failed to read file")]
    FileRead(#[from] std::io::Error),
}
