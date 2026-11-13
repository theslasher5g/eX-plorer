use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
    /// Zeitlimits und Abkühlzeit für Netzwerkzugriffe.
    pub net: explorer_core::timeout::Guard,
    /// Zeitlimit für Ordner im Netzwerk in Sekunden (aus den Einstellungen).
    pub net_timeout_secs: AtomicU32,
}

impl AppState {
    fn secs(&self) -> u64 {
        u64::from(self.net_timeout_secs.load(Ordering::Relaxed))
    }

    pub fn set_net_timeout(&self, secs: u32) {
        self.net_timeout_secs.store(secs, Ordering::Relaxed);
    }

    /// Ordner auflisten (Standard 10 s).
    pub fn dir_limit(&self) -> Duration {
        Duration::from_secs(self.secs())
    }

    /// Erreichbarkeitsprüfung (Standard 4 s).
    pub fn probe_limit(&self) -> Duration {
        Duration::from_millis(self.secs() * 400)
    }

    /// Freigaben eines Servers (Standard 8 s).
    pub fn shares_limit(&self) -> Duration {
        Duration::from_millis(self.secs() * 800)
    }

    /// Baum-Scan einer Ebene (Standard 15 s).
    pub fn tree_limit(&self) -> Duration {
        Duration::from_millis(self.secs() * 1500)
    }
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
