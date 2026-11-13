//! OneDrive: Konten finden und „Immer behalten“ / „Speicherplatz freigeben“.
//!
//! Es wird ausschließlich mit Metadaten gearbeitet (Verzeichniseinträge,
//! Dateiattribute). Nichts wird geöffnet, gelesen oder heruntergeladen; das
//! Herunterladen und Freigeben erledigt der OneDrive-Client selbst, sobald er
//! die Attribute sieht.

use std::os::windows::fs::MetadataExt;

use explorer_core::attrs::pin_attributes;
use explorer_core::onedrive::{self, Account, Origin, Raw};
use explorer_core::path::ValidPath;
use explorer_core::CoreError;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Storage::FileSystem::{SetFileAttributesW, FILE_FLAGS_AND_ATTRIBUTES};

use super::items::to_core;
use super::registry;

const ACCOUNTS_KEY: &str = r"Software\Microsoft\OneDrive\Accounts";
const ENV_VARS: [&str; 3] = ["OneDriveConsumer", "OneDriveCommercial", "OneDrive"];

/// Alle lokalen OneDrive-Ordner (mehrere Konten möglich).
pub fn accounts() -> Vec<Account> {
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

    onedrive::merge(raw, &|path| {
        std::fs::metadata(path.to_extended()).is_ok_and(|m| m.is_dir())
    })
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
