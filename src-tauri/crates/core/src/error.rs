use std::io;

use crate::path::PathError;

/// Fehler des Kerns. Die App-Schicht übersetzt sie in stabile Fehlercodes.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("invalid path: {0}")]
    InvalidPath(#[from] PathError),
    #[error("not found")]
    NotFound,
    #[error("access denied")]
    AccessDenied,
    #[error("not a directory")]
    NotADirectory,
    #[error("i/o error: {0}")]
    Io(io::Error),
}

impl From<io::Error> for CoreError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => Self::NotFound,
            io::ErrorKind::PermissionDenied => Self::AccessDenied,
            io::ErrorKind::NotADirectory => Self::NotADirectory,
            _ => Self::Io(err),
        }
    }
}
