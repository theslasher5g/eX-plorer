//! Öffnet eine Datei mit dem verknüpften Programm (nur auf ausdrückliche Handlung).

use explorer_core::path::ValidPath;
use explorer_core::CoreError;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use super::com::ComGuard;

const SE_ERR_NOASSOC: isize = 31;
const SE_ERR_ASSOCINCOMPLETE: isize = 27;
const SE_ERR_ACCESSDENIED: isize = 5;

/// Öffnet `path` mit dem Standardverb. Es werden keine Parameter übergeben.
pub fn open(owner: isize, path: &ValidPath) -> Result<(), CoreError> {
    let _com = ComGuard::sta();
    let file = HSTRING::from(path.as_str());
    let dir = path.parent().map(|p| HSTRING::from(p.as_str()));
    let dir_ptr = dir.as_ref().map_or(PCWSTR::null(), |d| PCWSTR(d.as_ptr()));
    let hwnd = (owner != 0).then_some(HWND(owner as *mut _));

    // SAFETY: Alle Zeichenketten sind nullterminiert und überleben den Aufruf.
    let result = unsafe {
        ShellExecuteW(
            hwnd,
            PCWSTR::null(),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            dir_ptr,
            SW_SHOWNORMAL,
        )
    };
    // Werte bis 32 sind Fehlercodes.
    match result.0 as isize {
        code if code > 32 => Ok(()),
        2 | 3 => Err(CoreError::NotFound),
        SE_ERR_ACCESSDENIED => Err(CoreError::AccessDenied),
        SE_ERR_NOASSOC | SE_ERR_ASSOCINCOMPLETE => Err(CoreError::NoAssociation),
        _ => Err(CoreError::Unsupported),
    }
}
