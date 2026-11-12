//! Tauri-Commands. Jeder Command validiert seine Eingaben selbst; das
//! Frontend hat keinen anderen Weg ins Dateisystem.

pub mod app;
pub mod fs;
pub mod menu;
pub mod ops;
pub mod preview;
pub mod watch;

use crate::error::AppError;

/// Führt blockierende Arbeit außerhalb des UI-Threads aus.
pub(crate) async fn blocking<T, F>(work: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, AppError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}
