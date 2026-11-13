//! Netzwerk: zugeordnete Laufwerke, Erreichbarkeit, Anmeldung und Freigaben.
//!
//! Passwörter berühren die App nie: Die Anmeldung läuft über den Windows-
//! Dialog (`WNetAddConnection3W` mit `CONNECT_INTERACTIVE`), gespeichert wird
//! in der Windows-Anmeldeinformationsverwaltung. Alle Funktionen blockieren
//! und gehören auf einen Worker-Thread, die meisten hinter ein Zeitlimit.

use explorer_core::network::{classify, Reachability};
use explorer_core::CoreError;
use windows::core::{HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{HWND, WIN32_ERROR};
use windows::Win32::NetworkManagement::NetManagement::NetApiBufferFree;
use windows::Win32::NetworkManagement::WNet::{
    WNetAddConnection3W, WNetGetConnectionW, CONNECT_CMD_SAVECRED, CONNECT_INTERACTIVE,
    NETRESOURCEW, RESOURCETYPE_DISK,
};
use windows::Win32::Storage::FileSystem::{
    GetDriveTypeW, GetLogicalDrives, NetShareEnum, SHARE_INFO_1,
};

const DRIVE_REMOTE: u32 = 4;
const ERROR_CANCELLED: u32 = 1223;
const ERROR_MORE_DATA: u32 = 234;
const STYPE_MASK: u32 = 0x0000_00FF;
const STYPE_DISKTREE: u32 = 0;
const STYPE_SPECIAL: u32 = 0x8000_0000;
const MAX_PREFERRED_LENGTH: u32 = u32::MAX;
/// Obergrenze für Freigaben pro Server.
const MAX_SHARES: usize = 500;
/// Längster UNC-Pfad, den `WNetGetConnectionW` liefert (inkl. Nullzeichen).
const MAX_REMOTE_UNITS: usize = 1024;

#[derive(Debug, Clone)]
pub struct MappedDrive {
    pub letter: char,
    /// `\\server\freigabe`, wenn Windows ihn kennt.
    pub remote: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Share {
    pub name: String,
    pub comment: String,
}

pub fn remote_name(letter: char) -> Option<String> {
    let local = HSTRING::from(format!("{letter}:"));
    let mut buffer = vec![0u16; MAX_REMOTE_UNITS];
    let mut len = buffer.len() as u32;
    // SAFETY: `local` ist nullterminiert; `buffer` fasst `len` Einheiten.
    let status = unsafe {
        WNetGetConnectionW(
            PCWSTR(local.as_ptr()),
            Some(PWSTR(buffer.as_mut_ptr())),
            &raw mut len,
        )
    };
    if status != WIN32_ERROR(0) {
        return None;
    }
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    let name = String::from_utf16_lossy(&buffer[..end]);
    (!name.is_empty()).then_some(name)
}

/// Alle zugeordneten Netzlaufwerke. Liest nur die lokale Verbindungstabelle,
/// berührt das Netzwerk also nicht.
pub fn mapped_drives() -> Vec<MappedDrive> {
    // SAFETY: Aufruf ohne Parameter.
    let mask = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|bit| mask & (1 << bit) != 0)
        .map(|bit| char::from(b'A' + bit))
        .filter(|letter| {
            let root = HSTRING::from(format!("{letter}:\\"));
            // SAFETY: `root` ist nullterminiert und überlebt den Aufruf.
            unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) == DRIVE_REMOTE }
        })
        .map(|letter| MappedDrive {
            letter,
            remote: remote_name(letter),
        })
        .collect()
}

/// Prüft, ob die Wurzel erreichbar ist. Nur Metadaten, nichts wird gelesen.
pub fn probe(root_extended: &std::path::Path) -> Reachability {
    match std::fs::metadata(root_extended) {
        Ok(_) => Reachability::Online,
        Err(err) => err
            .raw_os_error()
            .and_then(|code| u32::try_from(code).ok())
            .map_or(Reachability::Offline, classify),
    }
}

/// Verbindet `remote` (`\\server\freigabe`), optional mit Laufwerksbuchstaben.
/// Der Windows-Dialog fragt bei Bedarf nach Anmeldedaten und bietet an, sie zu
/// speichern. Liefert `false`, wenn der Benutzer abgebrochen hat.
pub fn connect(owner: isize, remote: &str, local: Option<char>) -> Result<bool, CoreError> {
    let remote_wide = HSTRING::from(remote);
    let local_wide = local.map(|letter| HSTRING::from(format!("{letter}:")));
    let resource = NETRESOURCEW {
        dwType: RESOURCETYPE_DISK,
        lpLocalName: local_wide
            .as_ref()
            .map_or(PWSTR::null(), |l| PWSTR(l.as_ptr().cast_mut())),
        lpRemoteName: PWSTR(remote_wide.as_ptr().cast_mut()),
        ..Default::default()
    };
    let hwnd = (owner != 0).then_some(HWND(owner as *mut _));
    // SAFETY: `resource` und die Zeichenketten leben bis nach dem Aufruf; weder
    // Kennwort noch Benutzername werden übergeben, Windows fragt selbst.
    let status = unsafe {
        WNetAddConnection3W(
            hwnd,
            &raw const resource,
            PCWSTR::null(),
            PCWSTR::null(),
            CONNECT_INTERACTIVE | CONNECT_CMD_SAVECRED,
        )
    };
    match status.0 {
        0 => Ok(true),
        ERROR_CANCELLED => Ok(false),
        code => Err(match classify(code) {
            Reachability::AuthRequired => CoreError::AuthRequired,
            _ => CoreError::Offline,
        }),
    }
}

/// Listet die Datenfreigaben eines Servers (ohne Verwaltungsfreigaben).
pub fn shares(server: &str) -> Result<Vec<Share>, CoreError> {
    let name = HSTRING::from(format!(r"\\{server}"));
    let mut buffer: *mut u8 = std::ptr::null_mut();
    let mut read = 0u32;
    let mut total = 0u32;
    // SAFETY: Alle Ausgabezeiger sind gültig; der von Windows gelieferte Puffer
    // wird unten genau einmal mit NetApiBufferFree freigegeben.
    let status = unsafe {
        NetShareEnum(
            PCWSTR(name.as_ptr()),
            1,
            &raw mut buffer,
            MAX_PREFERRED_LENGTH,
            &raw mut read,
            &raw mut total,
            None,
        )
    };
    let result = if status == 0 || status == ERROR_MORE_DATA {
        let count = (read as usize).min(MAX_SHARES);
        // SAFETY: Bei Erfolg zeigt `buffer` auf `read` SHARE_INFO_1-Einträge,
        // deren Zeiger bis zur Freigabe des Puffers gültig sind.
        let items = unsafe { std::slice::from_raw_parts(buffer.cast::<SHARE_INFO_1>(), count) };
        Ok(items
            .iter()
            .filter(|info| {
                info.shi1_type.0 & STYPE_SPECIAL == 0
                    && info.shi1_type.0 & STYPE_MASK == STYPE_DISKTREE
            })
            .map(|info| Share {
                // SAFETY: Die Zeiger stammen aus dem noch gültigen Puffer.
                name: unsafe { info.shi1_netname.to_string() }.unwrap_or_default(),
                comment: unsafe { info.shi1_remark.to_string() }.unwrap_or_default(),
            })
            .filter(|share| !share.name.is_empty())
            .collect())
    } else {
        Err(match classify(status) {
            Reachability::AuthRequired => CoreError::AuthRequired,
            _ => CoreError::Offline,
        })
    };
    if !buffer.is_null() {
        // SAFETY: Der Puffer stammt von NetShareEnum und wird nicht mehr gelesen.
        unsafe {
            NetApiBufferFree(Some(buffer.cast_const().cast()));
        }
    }
    result
}
