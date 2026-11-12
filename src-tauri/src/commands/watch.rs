use std::path::PathBuf;

use explorer_core::path::validate_path;
use tauri::State;

use crate::error::AppError;
use crate::shell::places;
use crate::state::AppState;

/// Mehr gleichzeitig beobachtete Ordner sind unrealistisch (offene Tabs).
const MAX_DIRS: usize = 64;

/// Setzt die Ordner, deren Änderungen als `dir-changed`-Ereignis gemeldet
/// werden. Netzlaufwerke werden bewusst nicht beobachtet.
#[tauri::command]
pub fn watch_dirs(state: State<'_, AppState>, paths: Vec<String>) -> Result<(), AppError> {
    let dirs: Vec<PathBuf> = paths
        .iter()
        .take(MAX_DIRS)
        .filter_map(|p| validate_path(p).ok())
        .filter(|p| !places::is_remote(p))
        .map(|p| PathBuf::from(p.as_str()))
        .collect();
    state
        .watcher
        .lock()
        .map_err(|_| AppError::Internal("watcher lock poisoned".into()))?
        .set_dirs(&dirs);
    Ok(())
}
