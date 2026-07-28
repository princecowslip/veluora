use thiserror::Error;

#[derive(Debug, Error)]
pub enum MediaError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("archive error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("archive exceeds safety limits: {0}")]
    ArchiveTooLarge(String),
    #[error("archive entry uses an unsafe path: {0}")]
    PathTraversal(String),
    #[error("page {0} not found")]
    PageNotFound(u32),
    #[error("media probe failed: {0}")]
    ProbeFailed(String),
}

pub type Result<T> = std::result::Result<T, MediaError>;
