use explorer_core::path::PathError;
use explorer_core::CoreError;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// Fehler, die an das Frontend gehen. Serialisiert als `{ code, message }`;
/// das Frontend übersetzt `code` in einen lokalisierten Text.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("clipboard: {0}")]
    Clipboard(#[from] crate::shell::clipboard::ClipboardError),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<PathError> for AppError {
    fn from(err: PathError) -> Self {
        Self::Core(CoreError::InvalidPath(err))
    }
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::Core(CoreError::InvalidPath(_)) => "invalid_path",
            Self::Core(CoreError::NotFound) => "not_found",
            Self::Core(CoreError::AccessDenied) => "access_denied",
            Self::Core(CoreError::NotADirectory) => "not_a_directory",
            Self::Core(CoreError::AlreadyExists) => "already_exists",
            Self::Core(CoreError::Busy) => "busy",
            Self::Core(CoreError::IntoItself) => "into_itself",
            Self::Core(CoreError::RootNotAllowed) => "root_not_allowed",
            Self::Core(CoreError::CloudOnly) => "cloud_only",
            Self::Core(CoreError::TooLarge) => "too_large",
            Self::Core(CoreError::Unsupported) => "unsupported",
            Self::Core(CoreError::NotAFile) => "not_a_file",
            Self::Core(CoreError::NoAssociation) => "no_association",
            Self::Core(CoreError::TooMany) => "too_many",
            Self::Core(CoreError::NothingToPaste) => "nothing_to_paste",
            Self::Core(CoreError::Io(_)) => "io",
            Self::Clipboard(_) => "clipboard",
            Self::Internal(_) => "internal",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}
