//! Namenssuche in Unterordnern, im Hintergrund und abbrechbar.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use explorer_core::listing::Entry;
use explorer_core::path::validate_path;
use explorer_core::search::{self, Limits};
use explorer_core::CoreError;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::State;

use super::blocking;
use super::tree::Registration;
use crate::error::AppError;
use crate::shell::places;
use crate::state::AppState;

/// Längere Suchtexte sind keine Dateinamen.
const MAX_QUERY: usize = 260;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchArgs {
    path: String,
    query: String,
    search_id: u32,
    #[serde(default)]
    show_hidden: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSummary {
    found: usize,
    visited: usize,
    cancelled: bool,
    truncated: bool,
    skipped_cloud: usize,
    unreadable: usize,
}

/// Sucht nach Namen im Ordner und allen Unterordnern. Treffer kommen in
/// Stapeln mit dem Pfad relativ zum Ordner im Feld `name`. Auf Netzwerkpfaden
/// gibt es keine Tiefensuche (`unsupported`).
#[tauri::command]
pub async fn search_dir(
    state: State<'_, AppState>,
    args: SearchArgs,
    on_hits: Channel<Vec<Entry>>,
) -> Result<SearchSummary, AppError> {
    let root = validate_path(&args.path)?;
    if args.query.chars().count() > MAX_QUERY {
        return Err(CoreError::Unsupported.into());
    }
    if places::is_remote(&root) {
        return Err(CoreError::Unsupported.into());
    }

    let cancel = Arc::new(AtomicBool::new(false));
    state
        .scans
        .lock()
        .map_err(|_| AppError::Internal("scan lock poisoned".into()))?
        .insert(args.search_id, Arc::clone(&cancel));
    let _registration = Registration {
        state: state.inner(),
        id: args.search_id,
    };

    let query = args.query;
    let show_hidden = args.show_hidden;
    let target = root.to_extended();
    let flag = Arc::clone(&cancel);
    blocking(move || {
        let stop = Arc::clone(&flag);
        let summary = search::search(
            &target,
            &query,
            &Limits::DEFAULT,
            show_hidden,
            &flag,
            &mut |hits| {
                // Nimmt das Frontend nichts mehr ab, endet die Suche.
                if !hits.is_empty() && on_hits.send(hits).is_err() {
                    stop.store(true, Ordering::Relaxed);
                }
            },
        )?;
        Ok(SearchSummary {
            found: summary.found,
            visited: summary.visited,
            cancelled: flag.load(Ordering::Relaxed),
            truncated: summary.truncated,
            skipped_cloud: summary.skipped_cloud,
            unreadable: summary.unreadable,
        })
    })
    .await
}

#[tauri::command]
pub fn search_cancel(state: State<'_, AppState>, search_id: u32) {
    if let Ok(scans) = state.scans.lock() {
        if let Some(flag) = scans.get(&search_id) {
            flag.store(true, Ordering::Relaxed);
        }
    }
}
