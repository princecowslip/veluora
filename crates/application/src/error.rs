use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] database::DatabaseError),
    #[error("could not resolve a data directory for this platform")]
    NoDataDirectory,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("invalid query: {0}")]
    InvalidQuery(String),
    #[error("unsupported capability: {0}")]
    UnsupportedCapability(String),
    #[error("network error: {0}")]
    Network(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
