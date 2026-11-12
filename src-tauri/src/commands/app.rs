use explorer_core::path::validate_path;
use serde::Serialize;
use tauri::State;

use super::blocking;
use crate::error::AppError;
use crate::shell::places::{self, Sidebar};
use crate::state::{AppState, Material};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    material: Material,
    /// Startordner (Benutzerprofil).
    home_dir: String,
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    let home_dir = std::env::var("USERPROFILE")
        .ok()
        .and_then(|p| validate_path(&p).ok())
        .map(|p| p.into_string())
        .unwrap_or_else(|| String::from(r"C:\"));
    AppInfo {
        material: state.material,
        home_dir,
    }
}

#[tauri::command]
pub async fn sidebar() -> Result<Sidebar, AppError> {
    blocking(|| Ok(places::sidebar())).await
}
