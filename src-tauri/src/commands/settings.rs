//! Einstellungen als JSON im Konfigurationsordner der App.

use std::fs;
use std::path::PathBuf;

use explorer_core::settings::Settings;
use tauri::{AppHandle, Manager, State};

use super::blocking;
use crate::error::AppError;
use crate::state::AppState;

const FILE: &str = "settings.json";
/// Eine größere Datei kann nicht von uns stammen.
const MAX_BYTES: u64 = 64 * 1024;

fn file(app: &AppHandle) -> Result<PathBuf, AppError> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join(FILE))
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Fehlende oder beschädigte Dateien ergeben die Standardwerte.
#[tauri::command]
pub async fn settings_get(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Settings, AppError> {
    let path = file(&app)?;
    let loaded = blocking(move || {
        let settings = fs::metadata(&path)
            .ok()
            .filter(|meta| meta.len() <= MAX_BYTES)
            .and_then(|_| fs::read(&path).ok())
            .and_then(|bytes| serde_json::from_slice::<Settings>(&bytes).ok())
            .unwrap_or_default();
        Ok(settings.sanitized())
    })
    .await?;
    state.set_net_timeout(loaded.network_timeout_secs);
    Ok(loaded)
}

/// Speichert die bereinigten Werte und gibt sie zurück.
#[tauri::command]
pub async fn settings_set(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<Settings, AppError> {
    let path = file(&app)?;
    let saved = blocking(move || {
        let settings = settings.sanitized();
        let json =
            serde_json::to_vec_pretty(&settings).map_err(|e| AppError::Internal(e.to_string()))?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(explorer_core::CoreError::from)?;
        }
        // Erst daneben schreiben, dann umbenennen: nie eine halbe Datei.
        let temp = path.with_extension("json.tmp");
        fs::write(&temp, json).map_err(explorer_core::CoreError::from)?;
        fs::rename(&temp, &path).map_err(explorer_core::CoreError::from)?;
        Ok(settings)
    })
    .await?;
    state.set_net_timeout(saved.network_timeout_secs);
    Ok(saved)
}
