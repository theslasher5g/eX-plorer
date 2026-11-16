//! OneDrive: Konten finden und „Immer behalten“ / „Speicherplatz freigeben“.
//!
//! Es wird ausschließlich mit Metadaten gearbeitet (Verzeichniseinträge,
//! Dateiattribute). Nichts wird geöffnet, gelesen oder heruntergeladen; das
//! Herunterladen und Freigeben erledigt der OneDrive-Client selbst, sobald er
//! die Attribute sieht.

use std::os::windows::fs::MetadataExt;

use explorer_core::attrs::pin_attributes;
use explorer_core::onedrive::{self, Account, Finding, Origin, Raw};
use explorer_core::path::ValidPath;
use explorer_core::CoreError;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Storage::FileSystem::{SetFileAttributesW, FILE_FLAGS_AND_ATTRIBUTES};

use super::items::to_core;
use super::registry;

const ACCOUNTS_KEY: &str = r"Software\Microsoft\OneDrive\Accounts";
const ENV_VARS: [&str; 3] = ["OneDriveConsumer", "OneDriveCommercial", "OneDrive"];

/// Sammelt alle Funde aus Registry, Umgebungsvariablen, Benutzerprofil und den
/// manuell hinzugefügten Ordnern.
fn collect(extra: &[String]) -> Vec<Raw> {
    let mut raw = Vec::new();

    for key in registry::subkeys(ACCOUNTS_KEY) {
        let path = format!(r"{ACCOUNTS_KEY}\{key}");
        if let Some(folder) = registry::string_value(&path, "UserFolder") {
            raw.push(Raw {
                email: registry::string_value(&path, "UserEmail"),
                origin: Origin::Registry(key),
                folder,
            });
        }
    }
    for var in ENV_VARS {
        if let Ok(folder) = std::env::var(var) {
            raw.push(Raw {
                origin: Origin::Env(var.to_owned()),
                folder,
                email: None,
            });
        }
    }
    // Rückfall: Ordner im Benutzerprofil, die wie OneDrive heißen.
    if let Ok(profile) = std::env::var("USERPROFILE") {
        raw.extend(onedrive::scan_profile(std::path::Path::new(&profile)));
    }
    raw.extend(extra.iter().map(|folder| Raw {
        origin: Origin::Manual,
        folder: folder.clone(),
        email: None,
    }));
    raw
}

fn is_dir(path: &ValidPath) -> bool {
    std::fs::metadata(path.to_extended()).is_ok_and(|m| m.is_dir())
}

/// Alle lokalen OneDrive-Ordner (mehrere Konten möglich).
pub fn accounts(extra: &[String]) -> Vec<Account> {
    onedrive::merge(collect(extra), &is_dir)
}

/// Wie `accounts`, aber mit dem Ergebnis jedes Fundes (für die Diagnose).
pub fn diagnose(extra: &[String]) -> Vec<Finding> {
    onedrive::merge_with_report(collect(extra), &is_dir).1
}

/// Setzt „Immer auf diesem Gerät behalten“ (`pinned`) bzw. „Speicherplatz
/// freigeben“. Bei Ordnern gilt das Attribut für den Ordner; der Client wendet
/// es auf den Inhalt an.
pub fn set_pinned(path: &ValidPath, pinned: bool) -> Result<(), CoreError> {
    let extended = path.to_extended();
    // Nur Metadaten, folgt keinen Links und lädt nichts herunter.
    let current = std::fs::symlink_metadata(&extended)?.file_attributes();
    let wide = HSTRING::from(extended.to_string_lossy().as_ref());
    // SAFETY: `wide` ist nullterminiert und überlebt den Aufruf; die Attribute
    // enthalten nur veränderbare Bits (siehe `pin_attributes`).
    unsafe {
        SetFileAttributesW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(pin_attributes(current, pinned)),
        )
    }
    .map_err(|e| to_core(&e))
}
