//! Orte für die Seitenleiste: bekannte Ordner, Laufwerke, OneDrive.

use explorer_core::onedrive::AccountKind;
use explorer_core::path::validate_path;
use serde::Serialize;
use windows::core::{GUID, PCWSTR};
use windows::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Pictures,
    SHGetKnownFolderPath, KF_FLAG_DEFAULT,
};

// Rückgabewerte von GetDriveTypeW.
const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_FIXED: u32 = 3;
const DRIVE_REMOTE: u32 = 4;
const DRIVE_CDROM: u32 = 5;
const DRIVE_RAMDISK: u32 = 6;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaceKind {
    Desktop,
    Documents,
    Downloads,
    Pictures,
    OneDrive,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    kind: PlaceKind,
    /// Anzeigename; leer bedeutet, dass das Frontend den Namen aus `kind` ableitet.
    label: String,
    path: String,
    /// OneDrive: E-Mail-Adresse des Kontos.
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    /// OneDrive: `personal` oder `business`.
    #[serde(skip_serializing_if = "Option::is_none")]
    account: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DriveKind {
    Fixed,
    Removable,
    Remote,
    Cdrom,
    Ram,
    Unknown,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Drive {
    /// Wurzelpfad, z. B. `C:\`.
    path: String,
    /// Datenträgerbezeichnung; leer, wenn unbekannt oder nicht abgefragt.
    label: String,
    kind: DriveKind,
    total_bytes: Option<u64>,
    free_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sidebar {
    favorites: Vec<Place>,
    drives: Vec<Drive>,
    onedrive: Vec<Place>,
}

pub fn sidebar() -> Sidebar {
    Sidebar {
        favorites: favorites(),
        drives: drives(),
        onedrive: onedrive(),
    }
}

fn favorites() -> Vec<Place> {
    [
        (PlaceKind::Desktop, &FOLDERID_Desktop),
        (PlaceKind::Documents, &FOLDERID_Documents),
        (PlaceKind::Downloads, &FOLDERID_Downloads),
        (PlaceKind::Pictures, &FOLDERID_Pictures),
    ]
    .into_iter()
    .filter_map(|(kind, id)| {
        let path = known_folder(id)?;
        // Nur Pfade anbieten, die unsere eigene Validierung bestehen.
        let path = validate_path(&path).ok()?.into_string();
        Some(Place {
            kind,
            label: String::new(),
            path,
            email: None,
            account: None,
        })
    })
    .collect()
}

fn known_folder(id: &GUID) -> Option<String> {
    // SAFETY: `id` ist eine gültige KNOWNFOLDERID. Der zurückgegebene Puffer
    // wird nach dem Kopieren mit CoTaskMemFree freigegeben.
    unsafe {
        let raw = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None).ok()?;
        let path = raw.to_string().ok();
        CoTaskMemFree(Some(raw.as_ptr().cast_const().cast()));
        path
    }
}

fn drives() -> Vec<Drive> {
    // SAFETY: Aufruf ohne Parameter.
    let mask = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|bit| mask & (1 << bit) != 0)
        .map(|bit| drive_info(char::from(b'A' + bit)))
        .collect()
}

fn drive_info(letter: char) -> Drive {
    let path = format!("{letter}:\\");
    let root: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let root_ptr = PCWSTR(root.as_ptr());

    // SAFETY: `root` ist nullterminiert und überlebt alle Aufrufe unten.
    let kind = match unsafe { GetDriveTypeW(root_ptr) } {
        DRIVE_FIXED => DriveKind::Fixed,
        DRIVE_REMOVABLE => DriveKind::Removable,
        DRIVE_REMOTE => DriveKind::Remote,
        DRIVE_CDROM => DriveKind::Cdrom,
        DRIVE_RAMDISK => DriveKind::Ram,
        _ => DriveKind::Unknown,
    };

    let mut drive = Drive {
        path,
        label: String::new(),
        kind,
        total_bytes: None,
        free_bytes: None,
    };

    // Nur lokale Festplatten abfragen: Netzlaufwerke können hängen, leere
    // Wechsellaufwerke Fehlerdialoge oder Wartezeiten auslösen.
    if matches!(kind, DriveKind::Fixed | DriveKind::Ram) {
        let mut name = [0u16; 261];
        // SAFETY: gültiger Wurzelpfad, `name` ist ein gültiger Ausgabepuffer.
        let has_label =
            unsafe { GetVolumeInformationW(root_ptr, Some(&mut name), None, None, None, None) }
                .is_ok();
        if has_label {
            let len = name.iter().position(|&c| c == 0).unwrap_or(name.len());
            drive.label = String::from_utf16_lossy(&name[..len]);
        }

        let mut total = 0u64;
        let mut free = 0u64;
        // SAFETY: gültiger Wurzelpfad, Zeiger auf lokale u64-Variablen.
        let has_space = unsafe {
            GetDiskFreeSpaceExW(root_ptr, Some(&raw mut free), Some(&raw mut total), None)
        }
        .is_ok();
        if has_space {
            drive.total_bytes = Some(total);
            drive.free_bytes = Some(free);
        }
    }
    drive
}

/// Liegt der Pfad auf einem Netzwerklaufwerk oder einer UNC-Freigabe?
/// Dort wird nichts beobachtet und nichts automatisch geladen.
pub fn is_remote(path: &explorer_core::path::ValidPath) -> bool {
    if path.is_unc() {
        return true;
    }
    let root: Vec<u16> = path.as_str()[..3.min(path.as_str().len())]
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: `root` ist nullterminiert und überlebt den Aufruf.
    unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) == DRIVE_REMOTE }
}

/// Lokale OneDrive-Ordner, ein Eintrag je Konto (Registry und Umgebungsvariablen).
fn onedrive() -> Vec<Place> {
    super::onedrive::accounts()
        .into_iter()
        .map(|account| Place {
            kind: PlaceKind::OneDrive,
            label: account.label,
            path: account.path.into_string(),
            email: account.email,
            account: Some(match account.kind {
                AccountKind::Personal => "personal",
                AccountKind::Business => "business",
            }),
        })
        .collect()
}
