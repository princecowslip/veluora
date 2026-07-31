//! Exit codes, per `docs/10-cli.md`. Only the codes reachable by
//! shipped commands are wired here; the rest of the documented table
//! (`7 Rate limited`, `10 Safety block`, etc.) applies as later commands
//! land.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ExitCode {
    Success = 0,
    /// Reserved for failures that don't fit a more specific code below —
    /// no command path produces this yet, but it's part of the
    /// documented table and future commands will use it.
    #[allow(dead_code)]
    GeneralFailure = 1,
    InvalidArguments = 2,
    NotFound = 3,
    NetworkFailure = 6,
    UnsupportedCapability = 8,
    DatabaseFailure = 11,
    ConfigurationFailure = 12,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

impl From<&application::AppError> for ExitCode {
    fn from(err: &application::AppError) -> Self {
        match err {
            application::AppError::Database(_) => ExitCode::DatabaseFailure,
            application::AppError::NoDataDirectory | application::AppError::Io(_) => {
                ExitCode::ConfigurationFailure
            }
            application::AppError::NotFound(_) => ExitCode::NotFound,
            application::AppError::InvalidQuery(_) | application::AppError::InvalidPath(_) => {
                ExitCode::InvalidArguments
            }
            application::AppError::Network(_) => ExitCode::NetworkFailure,
            application::AppError::UnsupportedCapability(_) => ExitCode::UnsupportedCapability,
        }
    }
}
