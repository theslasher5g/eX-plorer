use explorer_core::onedrive::contains;
use explorer_core::path::validate_path;
use explorer_core::{ops, CoreError};

use super::blocking;
use crate::error::AppError;
use crate::shell::onedrive;

/// „Immer auf diesem Gerät behalten“ (`pinned`) bzw. „Speicherplatz
/// freigeben“. Nur für Pfade innerhalb eines OneDrive-Ordners. Liefert die
/// Zahl der geänderten Elemente.
#[tauri::command]
pub async fn onedrive_set_pinned(paths: Vec<String>, pinned: bool) -> Result<usize, AppError> {
    if paths.len() > ops::MAX_ITEMS {
        return Err(CoreError::TooMany.into());
    }
    let valid = paths
        .iter()
        .map(|p| validate_path(p).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;
    blocking(move || {
        let accounts = onedrive::accounts();
        // Wurzel nicht: das ganze Konto lässt sich nur über OneDrive selbst ändern.
        if valid
            .iter()
            .any(|p| !contains(&accounts, p) || accounts.iter().any(|a| a.path.same_as(p)))
        {
            return Err(CoreError::Unsupported.into());
        }
        for path in &valid {
            onedrive::set_pinned(path, pinned)?;
        }
        Ok(valid.len())
    })
    .await
}
