use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("migration {version} failed: {message}")]
    Migration { version: i64, message: String },
    #[error("backup error: {0}")]
    Backup(String),
}

pub type Result<T> = std::result::Result<T, DatabaseError>;
