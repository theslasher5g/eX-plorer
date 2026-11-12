use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use explorer_core::watch::DirWatcher;
use serde::Serialize;
use tauri::window::{Effect, EffectsBuilder};
use tauri::WebviewWindow;

/// Erster Windows-11-Build; ab hier gibt es Mica.
const WINDOWS_11_BUILD: u32 = 22_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Material {
    /// Durchscheinender Mica-Hintergrund (Windows 11).
    Mica,
    /// Deckender Hintergrund (Windows 10 oder wenn Mica nicht verfügbar ist).
    Solid,
}

pub struct AppState {
    pub material: Material,
    pub watcher: Mutex<DirWatcher>,
    /// Abbruchmarken laufender Baum-Scans, nach Scan-Kennung.
    pub scans: Mutex<HashMap<u32, Arc<AtomicBool>>>,
}

/// Aktiviert Mica, wenn das System es unterstützt. Das Fenster ist transparent
/// angelegt; bei `Solid` zeichnet das Frontend einen deckenden Hintergrund.
pub fn apply_material(window: &WebviewWindow) -> Material {
    if windows_version::OsVersion::current().build < WINDOWS_11_BUILD {
        return Material::Solid;
    }
    let effects = EffectsBuilder::new().effect(Effect::Mica).build();
    match window.set_effects(effects) {
        Ok(()) => Material::Mica,
        Err(_) => Material::Solid,
    }
}
