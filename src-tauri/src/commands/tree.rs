//! Baum-Scan im Hintergrund, abbrechbar über eine Scan-Kennung.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use explorer_core::path::validate_path;
use explorer_core::tree::{self, DirChunk, Limits, Request};
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::State;

use super::blocking;
use crate::error::AppError;
use crate::shell::places;
use crate::state::AppState;

/// Schutz gegen absurde Offsets vom Frontend.
const MAX_OFFSET: usize = 5_000_000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanArgs {
    path: String,
    scan_id: u32,
    depth: u32,
    total: usize,
    per_dir: usize,
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    show_hidden: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeSummary {
    count: usize,
    cancelled: bool,
    truncated: bool,
    /// Netzwerkpfad: Es wurde bewusst nur eine Ebene gelesen.
    remote_limited: bool,
}

/// Entfernt die Abbruchmarke eines Scans, egal wie er endet.
struct Registration<'a> {
    state: &'a AppState,
    id: u32,
}

impl Drop for Registration<'_> {
    fn drop(&mut self) {
        if let Ok(mut scans) = self.state.scans.lock() {
            scans.remove(&self.id);
        }
    }
}

#[tauri::command]
pub async fn tree_scan(
    state: State<'_, AppState>,
    args: ScanArgs,
    on_chunk: Channel<DirChunk>,
) -> Result<TreeSummary, AppError> {
    let root = validate_path(&args.path)?;
    let mut limits = Limits {
        max_depth: args.depth,
        max_total: args.total,
        max_per_dir: args.per_dir,
    }
    .clamped();

    let cancel = Arc::new(AtomicBool::new(false));
    state
        .scans
        .lock()
        .map_err(|_| AppError::Internal("scan lock poisoned".into()))?
        .insert(args.scan_id, Arc::clone(&cancel));
    let _registration = Registration {
        state: state.inner(),
        id: args.scan_id,
    };

    let offset = args.offset.min(MAX_OFFSET);
    let show_hidden = args.show_hidden;
    blocking(move || {
        // Auf Netzlaufwerken nie automatisch in die Tiefe; Aufklappen lädt gezielt nach.
        let remote_limited = limits.max_depth > 1 && places::is_remote(&root);
        if remote_limited {
            limits.max_depth = 1;
        }
        let request = Request {
            limits,
            offset,
            show_hidden,
        };
        let stop = Arc::clone(&cancel);
        let summary = tree::scan(&root.to_extended(), &request, &cancel, &mut |chunk| {
            // Nimmt das Frontend nichts mehr ab, wird der Scan beendet.
            if on_chunk.send(chunk).is_err() {
                stop.store(true, Ordering::Relaxed);
            }
        })?;
        Ok(TreeSummary {
            count: summary.count,
            cancelled: summary.cancelled,
            truncated: summary.truncated,
            remote_limited,
        })
    })
    .await
}

#[tauri::command]
pub fn tree_cancel(state: State<'_, AppState>, scan_id: u32) {
    if let Ok(scans) = state.scans.lock() {
        if let Some(flag) = scans.get(&scan_id) {
            flag.store(true, Ordering::Relaxed);
        }
    }
}
