//! Baum-Scan im Hintergrund, abbrechbar über eine Scan-Kennung.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use explorer_core::path::validate_path;
use explorer_core::tree::{self, DirChunk, Limits, Request};
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::State;

use super::blocking;
use super::network::COOL_DOWN;
use crate::error::AppError;
use crate::shell::places;
use crate::state::AppState;

/// So lange wartet ein Scan auf einem Netzwerkpfad auf eine Antwort.
const REMOTE_LIMIT: std::time::Duration = std::time::Duration::from_secs(15);

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
    let guard = state.net.clone();
    blocking(move || {
        // Auf Netzlaufwerken nie automatisch in die Tiefe; Aufklappen lädt gezielt nach.
        let remote = places::is_remote(&root);
        let remote_limited = limits.max_depth > 1 && remote;
        if remote_limited {
            limits.max_depth = 1;
        }
        let request = Request {
            limits,
            offset,
            show_hidden,
        };
        let key = root.root().to_owned();
        let target = root.to_extended();
        let scan_cancel = Arc::clone(&cancel);
        let scan = move |_: &AtomicBool| {
            let stop = Arc::clone(&scan_cancel);
            tree::scan(&target, &request, &scan_cancel, &mut |chunk| {
                // Nimmt das Frontend nichts mehr ab, wird der Scan beendet.
                if on_chunk.send(chunk).is_err() {
                    stop.store(true, Ordering::Relaxed);
                }
            })
        };
        let summary = if remote {
            // Toter Server: nach dem Zeitlimit abbrechen statt zu hängen.
            let result = guard.run(&key, REMOTE_LIMIT, COOL_DOWN, scan);
            if result.is_err() {
                cancel.store(true, Ordering::Relaxed);
            }
            result?
        } else {
            scan(&AtomicBool::new(false))?
        };
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
