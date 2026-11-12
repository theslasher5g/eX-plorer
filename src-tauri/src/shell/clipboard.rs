//! Datei-Zwischenablage im Windows-Format (`CF_HDROP` plus „Preferred
//! DropEffect“). Damit funktionieren Kopieren, Ausschneiden und Einfügen
//! auch mit dem Explorer und anderen Programmen.

use std::mem::size_of;
use std::thread::sleep;
use std::time::Duration;

use windows::core::w;
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE,
};
use windows::Win32::System::Ole::CF_HDROP;
use windows::Win32::UI::Shell::{DragQueryFileW, DROPFILES, HDROP};

const DROPEFFECT_COPY: u32 = 1;
const DROPEFFECT_MOVE: u32 = 2;
/// Obergrenze für gelesene Pfade aus der Ablage (Schutz vor manipulierten Daten).
const MAX_PATHS: u32 = 20_000;

#[derive(Debug, thiserror::Error)]
pub enum ClipboardError {
    #[error("clipboard is busy")]
    Busy,
    #[error("clipboard operation failed")]
    Failed,
}

/// Öffnet die Zwischenablage (mit kurzen Wiederholungen, andere Programme
/// halten sie oft für Millisekunden) und schließt sie beim Drop.
struct Open;

impl Open {
    fn new(owner: isize) -> Result<Self, ClipboardError> {
        let hwnd = (owner != 0).then_some(HWND(owner as *mut _));
        for _ in 0..10 {
            // SAFETY: HWND ist null oder das Hauptfenster.
            if unsafe { OpenClipboard(hwnd) }.is_ok() {
                return Ok(Self);
            }
            sleep(Duration::from_millis(20));
        }
        Err(ClipboardError::Busy)
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        // SAFETY: Paart ein erfolgreiches OpenClipboard dieses Threads.
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

fn drop_effect_format() -> u32 {
    // SAFETY: Konstante, nullterminierte Zeichenkette.
    unsafe { RegisterClipboardFormatW(w!("Preferred DropEffect")) }
}

/// Legt eine Speichergruppe mit `bytes` an und übergibt sie der Zwischenablage.
/// Bei Erfolg gehört der Speicher der Zwischenablage, sonst wird er freigegeben.
fn put(format: u32, bytes: &[u8]) -> Result<(), ClipboardError> {
    // SAFETY: Die Allokation wird vor dem Schreiben gesperrt und passt exakt;
    // nach SetClipboardData gehört das Handle dem System.
    unsafe {
        let mem = GlobalAlloc(GMEM_MOVEABLE, bytes.len()).map_err(|_| ClipboardError::Failed)?;
        let ptr = GlobalLock(mem);
        if ptr.is_null() {
            let _ = GlobalFree(Some(mem));
            return Err(ClipboardError::Failed);
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.cast::<u8>(), bytes.len());
        let _ = GlobalUnlock(mem);
        if SetClipboardData(format, Some(HANDLE(mem.0))).is_err() {
            let _ = GlobalFree(Some(mem));
            return Err(ClipboardError::Failed);
        }
    }
    Ok(())
}

/// Baut die `DROPFILES`-Struktur: Kopf, dann Pfade als UTF-16, je mit `\0`
/// abgeschlossen, zum Schluss ein zusätzliches `\0`.
fn build_hdrop(paths: &[String]) -> Vec<u8> {
    let header = DROPFILES {
        pFiles: size_of::<DROPFILES>() as u32,
        fWide: true.into(),
        ..Default::default()
    };
    let mut bytes = Vec::with_capacity(size_of::<DROPFILES>() + paths.len() * 64);
    // SAFETY: DROPFILES ist eine reine Datenstruktur ohne Zeiger; wir lesen ihre Bytes.
    bytes.extend_from_slice(unsafe {
        std::slice::from_raw_parts((&raw const header).cast::<u8>(), size_of::<DROPFILES>())
    });
    for path in paths {
        for unit in path.encode_utf16().chain(std::iter::once(0)) {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
    }
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes
}

pub fn set(owner: isize, paths: &[String], cut: bool) -> Result<(), ClipboardError> {
    let _open = Open::new(owner)?;
    // SAFETY: Zwischenablage ist geöffnet.
    unsafe { EmptyClipboard() }.map_err(|_| ClipboardError::Failed)?;
    put(u32::from(CF_HDROP.0), &build_hdrop(paths))?;
    let effect = if cut {
        DROPEFFECT_MOVE
    } else {
        DROPEFFECT_COPY
    };
    put(drop_effect_format(), &effect.to_le_bytes())
}

pub fn clear(owner: isize) -> Result<(), ClipboardError> {
    let _open = Open::new(owner)?;
    // SAFETY: Zwischenablage ist geöffnet.
    unsafe { EmptyClipboard() }.map_err(|_| ClipboardError::Failed)
}

/// Gelesener Inhalt: Pfade und ob ausgeschnitten wurde.
pub struct Contents {
    pub paths: Vec<String>,
    pub cut: bool,
}

pub fn get(owner: isize) -> Result<Option<Contents>, ClipboardError> {
    // SAFETY: reine Abfrage ohne geöffnete Zwischenablage erlaubt.
    if unsafe { IsClipboardFormatAvailable(u32::from(CF_HDROP.0)) }.is_err() {
        return Ok(None);
    }
    let _open = Open::new(owner)?;

    // SAFETY: Zwischenablage ist geöffnet; das Handle bleibt Eigentum des Systems.
    let paths = unsafe {
        let Ok(handle) = GetClipboardData(u32::from(CF_HDROP.0)) else {
            return Ok(None);
        };
        let hdrop = HDROP(handle.0);
        let count = DragQueryFileW(hdrop, u32::MAX, None).min(MAX_PATHS);
        let mut out = Vec::with_capacity(count as usize);
        for i in 0..count {
            let len = DragQueryFileW(hdrop, i, None) as usize;
            let mut buf = vec![0u16; len + 1];
            let written = DragQueryFileW(hdrop, i, Some(&mut buf)) as usize;
            out.push(String::from_utf16_lossy(&buf[..written.min(len)]));
        }
        out
    };

    let cut = read_effect().is_some_and(|effect| effect & DROPEFFECT_MOVE != 0);
    Ok(Some(Contents { paths, cut }))
}

fn read_effect() -> Option<u32> {
    // SAFETY: Zwischenablage ist geöffnet (Aufrufer); das Handle gehört dem System,
    // gelesen wird nur innerhalb der gemeldeten Größe.
    unsafe {
        let handle = GetClipboardData(drop_effect_format()).ok()?;
        let mem = HGLOBAL(handle.0);
        if GlobalSize(mem) < size_of::<u32>() {
            return None;
        }
        let ptr = GlobalLock(mem);
        if ptr.is_null() {
            return None;
        }
        let value = ptr.cast::<u32>().read_unaligned();
        let _ = GlobalUnlock(mem);
        Some(value)
    }
}
