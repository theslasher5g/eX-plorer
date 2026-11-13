use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
    let dir_limit = state.dir_limit();
    blocking(move || {
        let remote = places::is_remote(&valid);
        let root = valid.root().to_owned();
        let is_root = valid.is_root();
        let target = valid.to_extended();
        // Nimmt das Fenster den Kanal nicht mehr ab (z. B. nach langem Leerlauf),
        // ist das ein Fehler und kein leeres Ergebnis.
        let channel_lost = Arc::new(AtomicBool::new(false));
        let lost = Arc::clone(&channel_lost);
        let list = move |cancel: &AtomicBool| {
            // Bricht ab, sobald das Zeitlimit abgelaufen ist oder der Kanal nichts mehr annimmt.
            let count = listing::list_dir(&target, BATCH_SIZE, |batch| {
                if cancel.load(Ordering::Relaxed) {
                    return false;
                }
                let sent = on_batch.send(batch).is_ok();
                if !sent {
                    lost.store(true, Ordering::Relaxed);
                }
                sent
            })?;
            if lost.load(Ordering::Relaxed) {
                return Err(CoreError::Io(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "ui channel closed",
                )));
            }
            Ok(count)
        };
        let result = if remote {
            guard.run(&root, dir_limit, COOL_DOWN, list)
        } else {
            list(&AtomicBool::new(false))
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
