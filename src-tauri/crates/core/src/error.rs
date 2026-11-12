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
    #[error("already exists")]
    AlreadyExists,
    #[error("in use by another process")]
    Busy,
    #[error("a folder cannot be placed inside itself")]
    IntoItself,
    #[error("drive roots cannot be changed this way")]
    RootNotAllowed,
    #[error("file is only in the cloud")]
    CloudOnly,
    #[error("file is too large")]
    TooLarge,
    #[error("not supported")]
    Unsupported,
    #[error("not a file")]
    NotAFile,
    #[error("no program is associated with this file type")]
    NoAssociation,
    #[error("too many items")]
    TooMany,
    #[error("nothing to paste")]
    NothingToPaste,
    #[error("i/o error: {0}")]
    Io(io::Error),
}

impl From<io::Error> for CoreError {
    fn from(err: io::Error) -> Self {
        // Sharing-/Lock-Verletzung (32/33): Datei ist anderweitig geöffnet.
        if matches!(err.raw_os_error(), Some(32 | 33)) {
            return Self::Busy;
        }
        match err.kind() {
            io::ErrorKind::NotFound => Self::NotFound,
            io::ErrorKind::PermissionDenied => Self::AccessDenied,
            io::ErrorKind::NotADirectory => Self::NotADirectory,
            io::ErrorKind::AlreadyExists => Self::AlreadyExists,
            _ => Self::Io(err),
        }
    }
}
