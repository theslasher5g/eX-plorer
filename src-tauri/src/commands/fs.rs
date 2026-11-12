use explorer_core::listing::{self, Entry};
use explorer_core::path::validate_path;
use serde::Serialize;
use tauri::ipc::Channel;

use super::blocking;
use crate::error::AppError;

/// Einträge pro Nachricht ans Frontend.
const BATCH_SIZE: usize = 2000;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSummary {
    /// Normalisierter Pfad des aufgelisteten Ordners.
    path: String,
    count: usize,
}

/// Listet einen Ordner auf (nur Metadaten) und streamt die Einträge in Stapeln.
#[tauri::command]
pub async fn list_dir(
    path: String,
    on_batch: Channel<Vec<Entry>>,
) -> Result<ListSummary, AppError> {
    let valid = validate_path(&path)?;
    blocking(move || {
        // Bricht ab, sobald das Frontend den Kanal nicht mehr abnimmt.
        let count = listing::list_dir(&valid.to_extended(), BATCH_SIZE, |batch| {
            on_batch.send(batch).is_ok()
        })?;
        Ok(ListSummary {
            path: valid.into_string(),
            count,
        })
    })
    .await
}
