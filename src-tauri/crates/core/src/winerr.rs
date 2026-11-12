//! Übersetzung von Windows-Fehlercodes. Reine Zahlenlogik, daher auf jeder
//! Plattform testbar.

use std::io;

use crate::CoreError;

const FACILITY_WIN32: u32 = 0x8007_0000;

const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_PATH_NOT_FOUND: u32 = 3;
const ERROR_ACCESS_DENIED: u32 = 5;
const ERROR_SHARING_VIOLATION: u32 = 32;
const ERROR_LOCK_VIOLATION: u32 = 33;
const ERROR_FILE_EXISTS: u32 = 80;
const ERROR_DIRECTORY: u32 = 267;
const ERROR_ALREADY_EXISTS: u32 = 183;
const ERROR_BAD_NETPATH: u32 = 53;
const ERROR_BAD_NET_NAME: u32 = 67;
/// Benutzer hat abgebrochen.
pub const ERROR_CANCELLED: u32 = 1223;

/// Zieht den Win32-Fehlercode aus einem HRESULT (`0x8007xxxx`) oder einem
/// nackten Win32-Code.
pub fn win32_code(hresult: i32) -> Option<u32> {
    let value = hresult as u32;
    if value & 0xFFFF_0000 == FACILITY_WIN32 {
        Some(value & 0xFFFF)
    } else if value < 0x1_0000 {
        Some(value)
    } else {
        None
    }
}

/// Wurde der Vorgang vom Benutzer abgebrochen?
pub fn is_cancelled(hresult: i32) -> bool {
    win32_code(hresult) == Some(ERROR_CANCELLED)
}

pub fn from_hresult(hresult: i32) -> CoreError {
    match win32_code(hresult) {
        Some(
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND | ERROR_BAD_NETPATH | ERROR_BAD_NET_NAME,
        ) => CoreError::NotFound,
        Some(ERROR_ACCESS_DENIED) => CoreError::AccessDenied,
        Some(ERROR_SHARING_VIOLATION | ERROR_LOCK_VIOLATION) => CoreError::Busy,
        Some(ERROR_FILE_EXISTS | ERROR_ALREADY_EXISTS) => CoreError::AlreadyExists,
        Some(ERROR_DIRECTORY) => CoreError::NotADirectory,
        Some(code) => CoreError::Io(io::Error::from_raw_os_error(code as i32)),
        None => CoreError::Io(io::Error::other(format!(
            "HRESULT 0x{:08X}",
            hresult as u32
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_hresults() {
        assert!(matches!(
            from_hresult(0x8007_0002_u32 as i32),
            CoreError::NotFound
        ));
        assert!(matches!(
            from_hresult(0x8007_0003_u32 as i32),
            CoreError::NotFound
        ));
        assert!(matches!(
            from_hresult(0x8007_0005_u32 as i32),
            CoreError::AccessDenied
        ));
        assert!(matches!(
            from_hresult(0x8007_0020_u32 as i32),
            CoreError::Busy
        ));
        assert!(matches!(
            from_hresult(0x8007_00B7_u32 as i32),
            CoreError::AlreadyExists
        ));
        assert!(matches!(
            from_hresult(0x8007_0050_u32 as i32),
            CoreError::AlreadyExists
        ));
    }

    #[test]
    fn accepts_bare_win32_codes() {
        assert!(matches!(from_hresult(2), CoreError::NotFound));
        assert!(matches!(from_hresult(5), CoreError::AccessDenied));
    }

    #[test]
    fn unknown_codes_become_io() {
        assert!(matches!(
            from_hresult(0x8000_4005_u32 as i32),
            CoreError::Io(_)
        ));
        assert!(matches!(
            from_hresult(0x8007_0057_u32 as i32),
            CoreError::Io(_)
        ));
    }

    #[test]
    fn detects_cancel() {
        assert!(is_cancelled(0x8007_04C7_u32 as i32));
        assert!(!is_cancelled(0x8007_0005_u32 as i32));
    }
}
