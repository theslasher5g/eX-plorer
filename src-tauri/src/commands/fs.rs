use std::time::Duration;

use explorer_core::listing::{self, Entry};
use explorer_core::path::validate_path;
use explorer_core::CoreError;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;

use super::blocking;
use super::network::COOL_DOWN;
use crate::error::AppError;
use crate::shell::places;
use crate::state::AppState;

/// Einträge pro Nachricht ans Frontend.
const BATCH_SIZE: usize = 2000;
/// So lange wartet das Auflisten eines Netzwerkordners auf eine Antwort.
const REMOTE_LIMIT: Duration = Duration::from_secs(10);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSummary {
    /// Normalisierter Pfad des aufgelisteten Ordners.
    path: String,
    count: usize,
}

/// Listet einen Ordner auf (nur Metadaten) und streamt die Einträge in Stapeln.
/// Auf Netzwerkpfaden mit Zeitlimit: Ein toter Server ergibt `timeout` bzw.
/// `offline`, nie eine hängende Oberfläche.
#[tauri::command]
pub async fn list_dir(
    state: State<'_, AppState>,
    path: String,
    on_batch: Channel<Vec<Entry>>,
) -> Result<ListSummary, AppError> {
    let valid = validate_path(&path)?;
    let guard = state.net.clone();
    blocking(move || {
        let remote = places::is_remote(&valid);
        let root = valid.root().to_owned();
        let is_root = valid.is_root();
        let target = valid.to_extended();
        let list = move |cancel: &std::sync::atomic::AtomicBool| {
            // Bricht ab, sobald das Frontend den Kanal nicht mehr abnimmt
            // oder das Zeitlimit abgelaufen ist.
            listing::list_dir(&target, BATCH_SIZE, |batch| {
                !cancel.load(std::sync::atomic::Ordering::Relaxed) && on_batch.send(batch).is_ok()
            })
        };
        let result = if remote {
            guard.run(&root, REMOTE_LIMIT, COOL_DOWN, list)
        } else {
            list(&std::sync::atomic::AtomicBool::new(false))
        };
        let count = result.map_err(|err| match err {
            // An der Wurzel einer Freigabe heißt „Zugriff verweigert“: Anmeldung nötig.
            CoreError::AccessDenied if remote && is_root => CoreError::AuthRequired,
            other => other,
        })?;
        Ok(ListSummary {
            path: valid.into_string(),
            count,
        })
    })
    .await
}
