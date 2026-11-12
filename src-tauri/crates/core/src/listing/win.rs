//! Verzeichnislisting über `FindFirstFileExW`.
//!
//! Einziger `unsafe`-Code dieses Crates. Die Aufrufe lesen nur
//! Verzeichniseinträge; es wird kein Dateihandle geöffnet.

use std::ffi::c_void;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_NO_MORE_FILES, HANDLE};
use windows::Win32::Storage::FileSystem::{
    FindClose, FindExInfoBasic, FindExSearchNameMatch, FindFirstFileExW, FindNextFileW,
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_REPARSE_POINT,
    FILE_ATTRIBUTE_SYSTEM, FIND_FIRST_EX_LARGE_FETCH, WIN32_FIND_DATAW,
};

use super::Entry;
use crate::CoreError;

const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;
const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000_000C;
/// 100-ns-Intervalle zwischen 1601-01-01 und 1970-01-01.
const EPOCH_DIFF_100NS: i64 = 116_444_736_000_000_000;

struct FindHandle(HANDLE);

impl Drop for FindHandle {
    fn drop(&mut self) {
        // SAFETY: Das Handle stammt aus FindFirstFileExW und wird genau einmal geschlossen.
        unsafe {
            let _ = FindClose(self.0);
        }
    }
}

fn filetime_to_ms(low: u32, high: u32) -> Option<i64> {
    let ticks = (i64::from(high) << 32) | i64::from(low);
    if ticks == 0 {
        return None;
    }
    Some((ticks - EPOCH_DIFF_100NS) / 10_000)
}

fn to_entry(data: &WIN32_FIND_DATAW) -> Option<Entry> {
    let len = data
        .cFileName
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(data.cFileName.len());
    let name = String::from_utf16_lossy(&data.cFileName[..len]);
    if name == "." || name == ".." {
        return None;
    }
    let attributes = data.dwFileAttributes;
    let is_dir = attributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0;
    // Cloud-Platzhalter (OneDrive) sind ebenfalls Reparse-Points, aber keine Links.
    let is_link = attributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0
        && matches!(
            data.dwReserved0,
            IO_REPARSE_TAG_MOUNT_POINT | IO_REPARSE_TAG_SYMLINK
        );
    Some(Entry {
        name,
        is_dir,
        size: if is_dir {
            0
        } else {
            (u64::from(data.nFileSizeHigh) << 32) | u64::from(data.nFileSizeLow)
        },
        modified_ms: filetime_to_ms(
            data.ftLastWriteTime.dwLowDateTime,
            data.ftLastWriteTime.dwHighDateTime,
        ),
        created_ms: filetime_to_ms(
            data.ftCreationTime.dwLowDateTime,
            data.ftCreationTime.dwHighDateTime,
        ),
        attributes,
        hidden: attributes & FILE_ATTRIBUTE_HIDDEN.0 != 0,
        system: attributes & FILE_ATTRIBUTE_SYSTEM.0 != 0,
        is_link,
    })
}

pub(super) fn for_each_entry(
    dir: &Path,
    visit: &mut dyn FnMut(Entry) -> bool,
) -> Result<(), CoreError> {
    // Suchmuster `<dir>\*`, ohne doppelten Backslash (in `\\?\`-Pfaden ungültig).
    let mut pattern: Vec<u16> = dir.as_os_str().encode_wide().collect();
    if pattern.contains(&0) {
        return Err(CoreError::NotFound);
    }
    if pattern.last() != Some(&u16::from(b'\\')) {
        pattern.push(u16::from(b'\\'));
    }
    pattern.push(u16::from(b'*'));
    pattern.push(0);

    let mut data = WIN32_FIND_DATAW::default();
    // SAFETY: `pattern` ist nullterminiert und lebt bis nach dem Aufruf; `data`
    // ist ein gültiger, ausreichend großer Puffer für FindExInfoBasic.
    let handle = unsafe {
        FindFirstFileExW(
            PCWSTR(pattern.as_ptr()),
            FindExInfoBasic,
            (&raw mut data).cast::<c_void>(),
            FindExSearchNameMatch,
            None,
            FIND_FIRST_EX_LARGE_FETCH,
        )
    }
    .map_err(|e| CoreError::from(io::Error::from(e)))?;
    let handle = FindHandle(handle);

    loop {
        if let Some(entry) = to_entry(&data) {
            if !visit(entry) {
                break;
            }
        }
        // SAFETY: gültiges Suchhandle und gültiger Ausgabepuffer.
        if let Err(e) = unsafe { FindNextFileW(handle.0, &raw mut data) } {
            if e.code() == ERROR_NO_MORE_FILES.to_hresult() {
                break;
            }
            return Err(CoreError::from(io::Error::from(e)));
        }
    }
    Ok(())
}
