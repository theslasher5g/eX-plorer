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
    #[error("server or share is not reachable")]
    Offline,
    #[error("credentials are required")]
    AuthRequired,
    #[error("the server did not answer in time")]
    Timeout,
    #[error("too many items")]
    TooMany,
    #[error("nothing to paste")]
    NothingToPaste,
    #[error("i/o error: {0}")]
    Io(io::Error),
}

impl From<io::Error> for CoreError {
    fn from(err: io::Error) -> Self {
        // Rohe Codes sind nur unter Windows Win32-Fehler; anderswo bedeuten
        // dieselben Zahlen etwas anderes.
        #[cfg(windows)]
        if let Some(code) = err.raw_os_error().and_then(|c| u32::try_from(c).ok()) {
            if let Some(mapped) = from_win32(code) {
                return mapped;
            }
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

/// Eindeutige Win32-Codes. Allgemeine Dateifehler (2, 3, 5) bleiben bewusst
/// allgemein, denn ihre Bedeutung hängt vom Kontext ab.
#[cfg(windows)]
fn from_win32(code: u32) -> Option<CoreError> {
    use crate::network::{classify, Reachability};
    match code {
        // Sharing-/Lock-Verletzung: Datei ist anderweitig geöffnet.
        32 | 33 => Some(CoreError::Busy),
        2 | 3 | 5 => None,
        52 | 53 | 55 | 59 | 64 | 65 | 67 | 121 | 1167 | 1203 | 1222 | 1225 | 1226 | 1229 | 1231
        | 1232 | 1235 | 1236 | 2250 => Some(CoreError::Offline),
        _ => match classify(code) {
            Reachability::AuthRequired => Some(CoreError::AuthRequired),
            _ => None,
        },
    }
}
